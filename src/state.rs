use crate::config::Config;
use k256::ecdsa::SigningKey;
use std::env;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub http_client: reqwest::Client,
    pub signing_key: SigningKey,
}

impl AppState {
    pub async fn new(config: Config) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            signing_key: load_signing_key().await,
        }
    }
}

async fn load_signing_key() -> SigningKey {
    if let Ok(private_key_hex) = env::var("SIGNING_PRIVATE_KEY_HEX") {
        let decoded = hex::decode(private_key_hex)
            .expect("SIGNING_PRIVATE_KEY_HEX must be valid hex for a 32-byte secp256k1 key");
        let key_bytes: [u8; 32] = decoded
            .as_slice()
            .try_into()
            .expect("SIGNING_PRIVATE_KEY_HEX must decode to exactly 32 bytes");
        return SigningKey::from_bytes(&key_bytes.into())
            .expect("SIGNING_PRIVATE_KEY_HEX is not a valid secp256k1 private key");
    }

    let kms_url = env::var("SIGNING_KEY_DERIVE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:1100/derive/secp256k1?path=signing-server".to_string());

    let key_vec = reqwest::get(&kms_url)
        .await
        .unwrap_or_else(|e| panic!("failed to fetch signing key from {}: {}", kms_url, e))
        .bytes()
        .await
        .expect("failed to read signing key response body");

    let key_bytes: [u8; 32] = key_vec
        .get(0..32)
        .expect("signing key response must contain at least 32 bytes")
        .try_into()
        .expect("failed to parse 32-byte signing key from response");
    SigningKey::from_bytes(&key_bytes.into())
        .expect("invalid secp256k1 signing key returned by signer service")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RoutesConfig, ProtectedRoute};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

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
        let _guard = env_lock().lock().unwrap();
        let config = make_test_config();
        // SAFETY: This test runs in isolation; mutating env vars is acceptable.
        unsafe {
            std::env::set_var(
                "SIGNING_PRIVATE_KEY_HEX",
                "0101010101010101010101010101010101010101010101010101010101010101",
            );
        }
        let state = AppState::new(config).await;
        assert_eq!(state.config.gateway_port, 8080);
        assert_eq!(state.config.facilitator_url, "https://example.com");
        assert_eq!(state.config.target_api_url, "http://localhost:3001");
        assert_eq!(state.config.routes.free.len(), 1);
        assert_eq!(state.config.routes.protected.len(), 1);
        assert_eq!(state.config.routes.protected[0].usdc_amount, 100);
        unsafe {
            std::env::remove_var("SIGNING_PRIVATE_KEY_HEX");
        }
    }

    #[tokio::test]
    async fn test_app_state_clone() {
        let _guard = env_lock().lock().unwrap();
        let config = make_test_config();
        // SAFETY: This test runs in isolation; mutating env vars is acceptable.
        unsafe {
            std::env::set_var(
                "SIGNING_PRIVATE_KEY_HEX",
                "0101010101010101010101010101010101010101010101010101010101010101",
            );
        }
        let state = AppState::new(config).await;
        let cloned = state.clone();
        assert_eq!(cloned.config.gateway_port, state.config.gateway_port);
        assert_eq!(
            cloned.config.facilitator_url,
            state.config.facilitator_url
        );
        unsafe {
            std::env::remove_var("SIGNING_PRIVATE_KEY_HEX");
        }
    }
}
