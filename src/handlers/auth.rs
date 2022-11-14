use axum::{
    extract,
    http::StatusCode,
    response,
    Extension,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use crate::state::{AuthData, JamfTokenJson, State};

const JAMF_AUTH_URL: &str = "/api/v1/auth/token";

pub async fn set_auth(
    extract::Json(payload): extract::Json<AuthData>,
    Extension(state): Extension<Arc<State>>,
) -> (StatusCode, response::Json<Value>) {

    // If we can auth with the payload creds, update state with new creds
    match validate_auth(payload.clone()).await {
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

async fn validate_auth(payload: AuthData) -> Result<JamfTokenJson, Box<dyn std::error::Error>> {
    let client = Client::new();
    let password: Option<String> = Some(payload.password);
    let response = client
        .post(format!("{}{}", payload.url, JAMF_AUTH_URL))
        .basic_auth(payload.username, password)
        .send()
        .await?
        .json::<JamfTokenJson>()
        .await?;

    Ok(response)
}

// todo: try_read and reject rather than read() and blocking
pub async fn get_auth(Extension(state): Extension<Arc<State>>) -> response::Json<Value> {
    let auth_data = state.auth_data.read().unwrap();
    println!("auth_data: {:?}", *auth_data);
    response::Json(json!(*auth_data))
}