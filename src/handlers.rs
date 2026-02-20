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
    let method = req.method().clone();
    let path = req.uri().path();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();

    // Strip the -v2 suffix if present (used only for x402 protocol version, not the backend endpoint)
    let backend_path = path.strip_suffix("-v2").unwrap_or(path);
    let target_url = format!("{}{}{}", state.config.target_api_url, backend_path, query);
    println!("Target {} URL: {}", method.as_str(), target_url);

    let mut proxy_req = state.http_client.request(method, &target_url);

    for (name, value) in req.headers() {
        if name != axum::http::header::HOST {
            proxy_req = proxy_req.header(name.as_str(), value.as_bytes());
        }
    }

    let body_bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to read request body");
            StatusCode::BAD_REQUEST
        })?;

    proxy_req = proxy_req.body(body_bytes);

    let response = proxy_req
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, "Proxy request failed");
            StatusCode::BAD_GATEWAY
        })?;

    let status = response.status();
    let resp_headers = response.headers().clone();
    let body = response
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let mut response_builder = Response::builder().status(status.as_u16());

    for (key, value) in resp_headers.iter() {
        response_builder = response_builder.header(key, value);
    }

    response_builder
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ProtectedRoute, RoutesConfig};
    use axum::http::Request;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_state(target_url: &str) -> Arc<AppState> {
        Arc::new(AppState {
            config: Config {
                gateway_port: 3000,
                facilitator_url: "https://www.x402.org/facilitator".to_string(),
                target_api_url: target_url.to_string(),
                networks: vec![],
                routes: RoutesConfig {
                    free: vec!["/free".to_string()],
                    protected: vec![ProtectedRoute {
                        path: "/protected".to_string(),
                        usdc_amount: 1000,
                    }],
                },
            },
            http_client: reqwest::Client::new(),
        })
    }

    #[tokio::test]
    async fn test_proxy_request_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/free"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("Hello from backend"),
            )
            .mount(&mock_server)
            .await;

        let state = make_state(&mock_server.uri());
        let req = Request::builder()
            .uri("/free")
            .body(Body::empty())
            .unwrap();

        let response = proxy_request(State(state), req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body_bytes, "Hello from backend");
    }

    #[tokio::test]
    async fn test_proxy_request_strips_v2_suffix() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/endpoint"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("v2 stripped"),
            )
            .mount(&mock_server)
            .await;

        let state = make_state(&mock_server.uri());
        let req = Request::builder()
            .uri("/endpoint-v2")
            .body(Body::empty())
            .unwrap();

        let response = proxy_request(State(state), req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body_bytes, "v2 stripped");
    }

    #[tokio::test]
    async fn test_proxy_request_target_down() {
        // Point to an unreachable address
        let state = make_state("http://127.0.0.1:1");
        let req = Request::builder()
            .uri("/anything")
            .body(Body::empty())
            .unwrap();

        let result = proxy_request(State(state), req).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn test_proxy_request_preserves_status_code() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/not-found"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let state = make_state(&mock_server.uri());
        let req = Request::builder()
            .uri("/not-found")
            .body(Body::empty())
            .unwrap();

        let response = proxy_request(State(state), req).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
