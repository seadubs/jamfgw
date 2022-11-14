use axum::{
    http::StatusCode,
    response,
    Extension,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, from_value, json, Value};
use std::sync::Arc;

use crate::state::State;

const JAMF_DEVICE_LIST_URL: &str = "api/v1/computers-inventory";

#[derive(Serialize, Deserialize)]
struct Device {
    device_id: String, // Why does Jamf use strings to represent numeric ids?
    name: String,
    model: String,
    os: String,
    os_is_latest: bool,
}

// todo: try_read and reject rather than block
pub async fn get_devices(Extension(state): Extension<Arc<State>>) -> (StatusCode, response::Json<Value>) {
    // Copy auth data for consistency
    let auth_valid = *state.auth_valid.read().unwrap();
    let auth_token = (*state.auth_token.read().unwrap().token).to_string();
    let url = (*state.auth_data.read().unwrap().url).to_string();

    // Bail if auth has not been validated
    if !auth_valid {
        return (
            StatusCode::UNAUTHORIZED,
            response::Json(json!({"err": "unauthorized"})),
        );
    }

    // Get list of devices, 500 if we fail
    match query_jamf_devices(auth_token, url).await {
        Ok((devices, status)) => return (status, response::Json(json!({ "devices": devices }))),
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                response::Json(json!({ "err": format!("{:?}", err) })),
            )
        }
    };
}

async fn query_jamf_devices(
    token: String,
    url: String,
) -> Result<(Vec<Device>, StatusCode), Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut device_list: Vec<Device> = vec![];
    let mut status = StatusCode::OK;
    let mut query_val = json!("");
    let mut expected_devices = 0;
    let mut current_page = 0;

    // Handle pagination
    while device_list.len() < expected_devices || current_page == 0 {
        // todo: process response to provide more accurate http response codes to our client
        let query_text: String = client
            .get(format!("{}{}", url, JAMF_DEVICE_LIST_URL))
            .bearer_auth(&token)
            .query(&[
                ("section", "GENERAL"),
                ("section", "HARDWARE"),
                ("section", "OPERATING_SYSTEM"),
                ("section", "SOFTWARE_UPDATES"),
            ])
            .query(&[("page", current_page)])
            .send()
            .await?
            .text()
            .await?;

        // Ensure json is well formed and parsable
        match from_str(&query_text[..]) {
            Ok(val) => query_val = val,
            Err(_) => status = StatusCode::BAD_GATEWAY,
        };

        // Update total number of expected devices
        expected_devices = match from_value(query_val["totalCount"].clone()) {
            Ok(total_count) => total_count,
            Err(err) => {
                println!("unable to parse totalCount from computers query: {:?}", err);
                status = StatusCode::BAD_GATEWAY;
                0
            }
        };

        // Extract list of results
        let devices = match serde_json::from_value::<Vec<Value>>(query_val["results"].clone()) {
            Ok(devices) => devices,
            Err(_) => {
                status = StatusCode::BAD_GATEWAY;
                Vec::new()
            }
        };

        // Not using strictly typed serde_json deserialization to avoid breaking on minor API changes
        for device in devices {
            let device_id = from_value(device["id"].clone())
                .expect(&format!("unable to parse device id: {:?}", device["id"]));
            let name = from_value(device["general"]["name"].clone()).expect(&format!(
                "unable to parse device name: {:?}",
                device["general"]["name"]
            ));
            let model = from_value(device["hardware"]["model"].clone())
                .expect(&format!("unable to parse device model: {:?}", device["id"]));
            let os_name: String =
                from_value(device["operatingSystem"]["name"].clone()).expect(&format!(
                    "unable to parse device os name: {:?}",
                    device["operatingSystem"]["name"]
                ));
            let os_version: String = from_value(device["operatingSystem"]["version"].clone())
                .expect(&format!(
                    "unable to parse device os version: {:?}",
                    device["operatingSystem"]["version"]
                ));
            let software_udates: Vec<Value> =
                from_value(device["softwareUpdates"].clone()).expect(&format!(
                    "unable to parse device software updates list: {:?}",
                    device["softwareUpdates"]
                ));

            device_list.push(Device {
                device_id,
                name,
                model,
                os: format!("{} {}", os_name, os_version),
                // todo: replace with comparison against versions retrieved from jamf
                os_is_latest: { software_udates.len() == 0 },
            })
        }

        current_page += 1;
    }

    // todo: for each device in devices_json, query for it's OS update state and push to device_list

    Ok((device_list, status))
}