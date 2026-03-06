use crate::config::Config;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub async fn new(config: Config) -> Self {
        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .read_timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            http_client,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProtectedRoute, RoutesConfig};

    fn make_test_config() -> Config {
        Config {
            gateway_port: 8080,
            facilitator_url: "https://example.com".to_string(),
            target_api_url: "http://localhost:3001".to_string(),
            networks: vec![],
            routes: RoutesConfig {
                free: vec!["/health".to_string()],
                protected: vec![ProtectedRoute {
                    path: "/paid".to_string(),
                    usdc_amount: 100,
                }],
            },
        }
    }

    #[tokio::test]
    async fn test_app_state_new() {
        let config = make_test_config();
        let state = AppState::new(config).await;
        assert_eq!(state.config.gateway_port, 8080);
        assert_eq!(state.config.facilitator_url, "https://example.com");
        assert_eq!(state.config.target_api_url, "http://localhost:3001");
        assert_eq!(state.config.routes.free.len(), 1);
        assert_eq!(state.config.routes.protected.len(), 1);
        assert_eq!(state.config.routes.protected[0].usdc_amount, 100);
    }

    #[tokio::test]
    async fn test_app_state_clone() {
        let config = make_test_config();
        let state = AppState::new(config).await;
        let cloned = state.clone();
        assert_eq!(cloned.config.gateway_port, state.config.gateway_port);
        assert_eq!(
            cloned.config.facilitator_url,
            state.config.facilitator_url
        );
    }
}
