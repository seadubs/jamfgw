use serde::{Deserialize, Serialize};
use std::sync::RwLock;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct AuthData {
    pub username: String,
    pub password: String,
    pub url: String,
}

#[derive(Deserialize, Default)]
pub struct JamfTokenJson {
    pub token: String,
    //  Suppress warning, field necessary for strictly typed json deserialization but unused for our simple functionality
    #[allow(dead_code)]
    expires: String,
}

// todo: refactor auth to be all one RwLock
pub struct State {
    pub auth_data: RwLock<AuthData>,
    pub auth_token: RwLock<JamfTokenJson>,
    pub auth_valid: RwLock<bool>,
}
