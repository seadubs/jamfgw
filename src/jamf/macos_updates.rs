use reqwest::Client;
use serde_json::{from_str, from_value, Value};

const JAMF_MACOS_VERSION_LIST_URL: &str = "api/v1/macos-managed-software-updates/available-updates";

pub async fn query_jamf_macos_available_updates(
    url: String,
    token: String,
) -> Result<Vec<String>, &'static str> {
    let client = Client::new();

    let response = match client
        .get(format!("{}{}", url, JAMF_MACOS_VERSION_LIST_URL))
        .bearer_auth(&token)
        .send()
        .await
    {
        Ok(result) => result,
        Err(_) => return Err("failed to query jamf macos available updates"),
    };

    let response = match response.text().await {
        Ok(result) => result,
        Err(_) => return Err("unexpected response body received from jamf"),
    };

    parse_jamf_macos_updates_response(response)
}

fn parse_jamf_macos_updates_response(query_text: String) -> Result<Vec<String>, &'static str> {
    let query_val: Value = match from_str(&query_text) {
        Ok(val) => val,
        Err(_) => return Err("failed to deserialize jamf response to json"),
    };
    match from_value::<Vec<String>>(query_val["availableUpdates"].clone()) {
        Ok(versions) => Ok(versions),
        Err(_) => return Err("failed to extract version array from jamf response"),
    }
}
