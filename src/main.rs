use axum::{
    middleware,
    routing::{get, post},
    Extension, Router,
};
use http::Method;
use std::sync::{Arc, RwLock};
use tower_http::cors::{Any, CorsLayer};

use crate::utility::logging;
pub mod utility;

use crate::handlers::{auth, devices, hello};
pub mod handlers;

pub mod jamf;

pub mod state;
use crate::state::{AuthData, JamfTokenJson, State};

// todo: testing https://doc.rust-lang.org/rustc/tests/index.html
// todo: add command line options (e.g. port, log level)
// todo: add config/env file for static creds
#[tokio::main]
async fn main() {
    let state = Arc::new(State {
        auth_data: RwLock::new(AuthData::default()),
        auth_token: RwLock::new(JamfTokenJson::default()),
        auth_valid: RwLock::new(false),
    });

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any)
        .allow_origin(Any);

    let app = Router::new()
        .route("/api/hello", get(hello::handler))
        .route(
            "/api/jamf/credentials",
            post(auth::put_auth)
                .get(auth::get_auth)
                .delete(auth::del_auth),
        )
        .route("/api/jamf/devices", get(devices::get_devices))
        // Wide open cors policy for the sake of FE app testing
        .layer(cors)
        // Shared state for handler access to credentials
        .layer(Extension(state))
        // Log request and response headers (replace with logging/metrics/tracing library in a "real" service)
        .layer(middleware::from_fn(logging::log_req_res_stdout));

    axum::Server::bind(&"0.0.0.0:3001".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
