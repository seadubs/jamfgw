use axum::{http::StatusCode, response, Extension};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, Value};
use std::sync::Arc;
use version_compare::Version;

use crate::jamf::computer_inventory::query_jamf_computer_inventory;
use crate::jamf::macos_updates;
use crate::state::State;

#[derive(Serialize, Deserialize)]
struct ResponseDevice {
    device_id: String,
    name: String,
    model: String,
    os: String,
    os_is_latest: bool,
}

#[derive(Serialize, Deserialize)]
struct Device {
    // ? Why does Jamf use strings to represent numeric ids?
    device_id: String,
    name: String,
    model: String,
    os: String,
    os_name: String,
    os_version: String,
    os_is_latest: bool,
}

// todo: try_read and reject rather than block
pub async fn get_devices(
    Extension(state): Extension<Arc<State>>,
) -> (StatusCode, response::Json<Value>) {
    // Copy auth data for consistency
    let auth_valid = *state.auth_valid.read().unwrap();
    let auth_token = (*state.auth_token.read().unwrap().token).to_string();
    let url = (*state.auth_data.read().unwrap().url).to_string();

    // Bail if auth has not been validated
    if !auth_valid {
        return (
            StatusCode::UNAUTHORIZED,
            response::Json(json!({"err": "server credentials are not set and/or not valid"})),
        );
    }

    // Get list of devices, 500 if we fail
    match query_jamf_devices(auth_token, url).await {
        Ok(devices) => {
            return (
                StatusCode::OK,
                response::Json(json!({ "devices": devices })),
            )
        }
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                response::Json(json!({ "err": format!("{:?}", err) })),
            )
        }
    };
}

async fn query_jamf_devices(token: String, url: String) -> Result<Vec<Device>, &'static str> {
    let mut device_list: Vec<Device> = vec![];

    let latest_macos_version = match latest_macos_update(token.clone(), url.clone()).await {
        Ok((response, _status)) => response,
        Err(_) => return Err("failed to retrieve latest macos update version info"),
    };
    let latest_macos_version = Version::from(&latest_macos_version).unwrap();

    let devices = match query_jamf_computer_inventory(&url, &token).await {
        Ok(value) => value,
        Err(_) => return Err("Failed to get list of devices from jamf"),
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
        let os_version: String =
            from_value(device["operatingSystem"]["version"].clone()).expect(&format!(
                "unable to parse device os version: {:?}",
                device["operatingSystem"]["version"]
            ));

        device_list.push(Device {
            device_id,
            name,
            model,
            os_name: os_name.clone(),
            os_version: os_version.clone(),
            os: format!("{} {}", os_name, os_version),
            os_is_latest: false,
        })
    }

    for device in device_list.iter_mut() {
        device.os_is_latest = match device.os_name.as_str() {
            "macOS" => {
                let version = Version::from(&device.os_version).unwrap();
                version >= latest_macos_version
            }
            // Current spec does not handle non-macOS devices, fail safe (no false confidence in update status)
            &_ => false,
        };
    }

    Ok(device_list)
}

async fn latest_macos_update(
    token: String,
    url: String,
) -> Result<(String, StatusCode), &'static str> {
    let versions = match macos_updates::query_jamf_macos_available_updates(url, token).await {
        Ok(value) => value,
        Err(err) => return Err(err),
    };

    let latest_version = find_latest_update(&versions);

    Ok((latest_version.to_string(), StatusCode::OK))
}

fn find_latest_update<'a>(versions: &'a Vec<String>) -> Version {
    let latest_version = Version::from("0").expect("failed to set version floor");
    // Reduce version list to its "greatest" value
    let latest_version = versions
        .iter()
        .fold(latest_version, |latest_version, version| {
            let version = Version::from(version).unwrap();
            if version > latest_version {
                version
            } else {
                latest_version
            }
        });
    latest_version
}
