use crate::state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    response::Response,
};
use k256::ecdsa::SigningKey;
use sha3::{Digest, Keccak256};
use std::sync::Arc;
use tracing::{error, info};

pub async fn proxy_request(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let request_path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();

    // Strip the -v2 suffix if present (used only for x402 protocol version, not the backend endpoint)
    let backend_path = path.strip_suffix("-v2").unwrap_or(&path);
    let target_url = format!("{}{}{}", state.config.target_api_url, backend_path, query);
    println!("Target {} URL: {}", method.as_str(), target_url);

    let mut proxy_req = state.http_client.request(method.clone(), &target_url);

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

    let request_body_bytes = body_bytes.clone();
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
        if key == "transfer-encoding" || key == "content-length" {
            continue;
        }
        response_builder = response_builder.header(key, value);
    }

    let signing_message = build_signing_message(
        &method,
        &request_path_and_query,
        request_body_bytes.as_ref(),
        body.as_ref(),
    );
    let signature = sign_message(&state.signing_key, &signing_message);
    response_builder = response_builder.header("X-Signature", &signature);

    response_builder
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn build_signing_message(
    request_method: &Method,
    request_path_and_query: &str,
    request_body: &[u8],
    response_body: &[u8],
) -> Vec<u8> {
    let method = request_method.as_str().as_bytes();
    let path_and_query = request_path_and_query.as_bytes();

    let mut message = Vec::with_capacity(
        16 + method.len() + path_and_query.len() + request_body.len() + response_body.len(),
    );
    message.extend_from_slice(b"oyster-signature-v2\0");
    message.extend_from_slice(&(method.len() as u32).to_be_bytes());
    message.extend_from_slice(method);
    message.extend_from_slice(&(path_and_query.len() as u32).to_be_bytes());
    message.extend_from_slice(path_and_query);
    message.extend_from_slice(&(request_body.len() as u64).to_be_bytes());
    message.extend_from_slice(request_body);
    message.extend_from_slice(&(response_body.len() as u64).to_be_bytes());
    message.extend_from_slice(response_body);
    message
}

fn sign_message(signing_key: &SigningKey, message: &[u8]) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(message);
    let (signature, recovery_id) = signing_key
        .sign_digest_recoverable(hasher)
        .expect("signing failed");

    let mut sig_bytes = signature.to_vec();
    sig_bytes.push(recovery_id.to_byte() + 27);
    hex::encode(sig_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ProtectedRoute, RoutesConfig};
    use axum::http::Request;
    use k256::ecdsa::SigningKey;
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
            signing_key: test_signing_key(),
        })
    }

    fn test_signing_key() -> SigningKey {
        let key_bytes = [1u8; 32];
        SigningKey::from_bytes(&key_bytes.into()).unwrap()
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

    #[tokio::test]
    async fn test_proxy_request_adds_signature_header() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/signed"))
            .respond_with(ResponseTemplate::new(200).set_body_string("signed body"))
            .mount(&mock_server)
            .await;

        let state = make_state(&mock_server.uri());
        let req = Request::builder()
            .uri("/signed")
            .body(Body::empty())
            .unwrap();

        let response = proxy_request(State(state), req).await.unwrap();
        let sig = response.headers().get("X-Signature").unwrap();
        assert_eq!(sig.as_bytes().len(), 130);
    }
}
