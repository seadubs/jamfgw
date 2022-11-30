use crate::state::{AuthData, JamfTokenJson, State};
use axum::{extract, http::StatusCode, response, Extension};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;

const JAMF_AUTH_URL: &str = "/api/v1/auth/token";

pub async fn put_auth(
    extract::Json(payload): extract::Json<AuthData>,
    Extension(state): Extension<Arc<State>>,
) -> (StatusCode, response::Json<Value>) {
    match validate_auth(payload.clone()).await {
        // If we can auth with the payload creds, update state with new creds
        Ok(jamf_token) => {
            let mut auth_data = state.auth_data.write().unwrap();
            let mut auth_token = state.auth_token.write().unwrap();
            let mut auth_valid = state.auth_valid.write().unwrap();
            *auth_data = payload;
            *auth_valid = true;
            *auth_token = jamf_token;
            return (StatusCode::ACCEPTED, response::Json(json!(*auth_data)));
        }
        Err(err) => {
            return (
                StatusCode::UNAUTHORIZED,
                response::Json(
                    json!({"message": "credential update failed", "err": format!("{:?}", err) }),
                ),
            );
        }
    }
}

// Test provided credentials against provided url, return token if valid
async fn validate_auth(payload: AuthData) -> Result<JamfTokenJson, &'static str> {
    let client = Client::new();
    let password: Option<String> = Some(payload.password);

    let response = match client
        .post(format!("{}{}", payload.url, JAMF_AUTH_URL))
        .basic_auth(payload.username, password)
        .send()
        .await
    {
        Ok(result) => result,
        Err(_) => return Err("failed to authenticate to jamf"),
    };

    match response.json::<JamfTokenJson>().await {
        Ok(token_json) => Ok(token_json),
        Err(_) => Err("failed to extract bearer token from jamf response"),
    }
}

// todo: try_read and reject rather than read() and blocking
pub async fn get_auth(Extension(state): Extension<Arc<State>>) -> response::Json<Value> {
    let auth_data = state.auth_data.read().unwrap();
    let response_data = AuthData{
        username: auth_data.username.clone(),
        password: "********".parse().unwrap(),
        url: auth_data.url.clone(),
    };
    response::Json(json!(response_data))
}

pub async fn del_auth(Extension(state): Extension<Arc<State>>) -> StatusCode {
    let mut auth_data = state.auth_data.write().unwrap();
    let mut auth_token = state.auth_token.write().unwrap();
    let mut auth_valid = state.auth_valid.write().unwrap();
    *auth_data = AuthData::default();
    *auth_valid = false;
    *auth_token = JamfTokenJson::default();
    StatusCode::NO_CONTENT
}
