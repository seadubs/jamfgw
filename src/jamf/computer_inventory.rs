use reqwest::Client;
use serde_json::{from_str, from_value, Value};

const JAMF_DEVICE_LIST_URL: &str = "api/v1/computers-inventory";

pub async fn query_jamf_computer_inventory(url: &String, token: &String) -> Result<Vec<Value>, &'static str> {
    let mut expected_devices = 0;
    let mut current_page = 0;
    let mut all_devices: Vec<Value> = vec![];

    // Handle pagination
    while all_devices.len() < expected_devices || current_page == 0 {
        let query_text = match query_jamf_computer_inventory_by_page(&url, &token, current_page).await {
            Ok(value) => value,
            Err(_) => return Err("failed to retrieve jamf computer inventory"),
        };

        // Ensure json is well formed and parsable
        let query_val: Value = match from_str(&query_text[..]) {
            Ok(value) => value,
            Err(_) => return Err("failed to deserialize jamf response to json"),
        };

        // Update total number of expected devices
        expected_devices = match from_value(query_val["totalCount"].clone()) {
            Ok(total_count) => total_count,
            Err(_) => return Err("unable to parse totalCount from jamf response"),
        };

        // Extract list of results
        let mut devices = match serde_json::from_value::<Vec<Value>>(query_val["results"].clone()) {
            Ok(devices) => devices,
            Err(_) => return Err("unable to parse device list from jamf response")
        };
        
        all_devices.append(&mut devices);

        current_page += 1;
    }

    Ok(all_devices)
}

pub async fn query_jamf_computer_inventory_by_page(url: &String, token: &String, current_page: i32) -> Result<String, &'static str> {
    let client = Client::new();

    let query_text = match client
        .get(format!("{}{}", url, JAMF_DEVICE_LIST_URL))
        .bearer_auth(token)
        .query(&[
            ("section", "GENERAL"),
            ("section", "HARDWARE"),
            ("section", "OPERATING_SYSTEM"),
            ("section", "SOFTWARE_UPDATES"),
        ])
        .query(&[("page", current_page)])
        .send()
        .await {
            Ok(response) => response,
            Err(_) => return Err("failed to retrieve computer inventory from jamf"),
        };
    let query_text = match query_text.text()
        .await {
            Ok(value) => value,
            Err(_) => return Err("unexpected response body received from jamf")
        };
    Ok(query_text)
}