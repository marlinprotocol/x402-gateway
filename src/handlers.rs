use crate::state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::Response,
};
use std::sync::Arc;
use tracing::error;

pub async fn proxy_request(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    // Strip the -v2 suffix if present (used only for x402 protocol version, not the backend endpoint)
    let backend_path = path.strip_suffix("-v2").unwrap_or(path);
    let target_url = format!("{}{}", state.config.target_api_url, backend_path);

    let response = state
        .http_client
        .get(&target_url)
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Proxy request failed");
            StatusCode::BAD_GATEWAY
        })?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.map_err(|_| StatusCode::BAD_GATEWAY)?;

    let mut response_builder = Response::builder().status(status.as_u16());

    for (key, value) in headers.iter() {
        response_builder = response_builder.header(key, value);
    }

    response_builder
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
