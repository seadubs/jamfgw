use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::Utc;

// todo: update to use tracing or log library instead of stdout
pub async fn log_req_res_stdout(
    req: Request<Body>,
    next: Next<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    println!(
        "\n{} Request rec'd:\n{:?}",
        Utc::now().to_rfc2822(),
        parts
    );
    let req = Request::from_parts(parts, body);

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    println!(
        "\n{} Response sent:\n{:?}",
        Utc::now().to_rfc2822(),
        parts
    );
    let res = Response::from_parts(parts, body);

    Ok(res)
}