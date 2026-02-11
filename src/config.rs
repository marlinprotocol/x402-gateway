use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NetworkConfig {
    Evm {
        network: String,
        payment_address: String,
    },
    Solana {
        network: String,
        payment_address: String,
    },
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProtectedRoute {
    pub path: String,
    pub usdc_amount: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoutesConfig {
    pub free: Vec<String>,
    pub protected: Vec<ProtectedRoute>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub gateway_port: u16,
    pub facilitator_url: String,
    pub target_api_url: String,
    pub networks: Vec<NetworkConfig>,
    pub routes: RoutesConfig,
}

pub fn load_config() -> Config {
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.json".to_string());
    let config_str = fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("Failed to read config file: {}", config_path));
    serde_json::from_str(&config_str).expect("Failed to parse config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config_json() -> &'static str {
        r#"{
            "gateway_port": 3000,
            "facilitator_url": "https://example.com/facilitator",
            "target_api_url": "http://127.0.0.1:3001",
            "networks": [
                {
                    "type": "evm",
                    "network": "base-sepolia",
                    "payment_address": "0xd232A8b0F63a555d054134f67b298ffE955f3BAf"
                },
                {
                    "type": "solana",
                    "network": "solana-devnet",
                    "payment_address": "EGBQqKn968sVv5cQh5Cr72pSTHfxsuzq7o7asqYB5uEV"
                }
            ],
            "routes": {
                "free": ["/free", "/health"],
                "protected": [
                    { "path": "/protected", "usdc_amount": 1000 },
                    { "path": "/premium", "usdc_amount": 5000 }
                ]
            }
        }"#
    }

    #[test]
    fn test_deserialize_full_config() {
        let config: Config = serde_json::from_str(sample_config_json()).unwrap();
        assert_eq!(config.gateway_port, 3000);
        assert_eq!(config.facilitator_url, "https://example.com/facilitator");
        assert_eq!(config.target_api_url, "http://127.0.0.1:3001");
        assert_eq!(config.networks.len(), 2);
        assert_eq!(config.routes.free.len(), 2);
        assert_eq!(config.routes.protected.len(), 2);
    }

    #[test]
    fn test_deserialize_evm_network() {
        let json = r#"{ "type": "evm", "network": "base-sepolia", "payment_address": "0xABC" }"#;
        let net: NetworkConfig = serde_json::from_str(json).unwrap();
        match net {
            NetworkConfig::Evm {
                network,
                payment_address,
            } => {
                assert_eq!(network, "base-sepolia");
                assert_eq!(payment_address, "0xABC");
            }
            _ => panic!("Expected EVM variant"),
        }
    }

    #[test]
    fn test_deserialize_solana_network() {
        let json =
            r#"{ "type": "solana", "network": "solana-devnet", "payment_address": "SolAddr123" }"#;
        let net: NetworkConfig = serde_json::from_str(json).unwrap();
        match net {
            NetworkConfig::Solana {
                network,
                payment_address,
            } => {
                assert_eq!(network, "solana-devnet");
                assert_eq!(payment_address, "SolAddr123");
            }
            _ => panic!("Expected Solana variant"),
        }
    }

    #[test]
    fn test_deserialize_protected_route() {
        let json = r#"{ "path": "/api/data", "usdc_amount": 2500 }"#;
        let route: ProtectedRoute = serde_json::from_str(json).unwrap();
        assert_eq!(route.path, "/api/data");
        assert_eq!(route.usdc_amount, 2500);
    }

    #[test]
    fn test_load_config_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test_config.json");
        fs::write(&file_path, sample_config_json()).unwrap();

        // SAFETY: This test runs in isolation; mutating env vars is acceptable.
        unsafe {
            std::env::set_var("CONFIG_PATH", file_path.to_str().unwrap());
        }
        let config = load_config();
        assert_eq!(config.gateway_port, 3000);
        assert_eq!(config.networks.len(), 2);
        unsafe {
            std::env::remove_var("CONFIG_PATH");
        }
    }

    #[test]
    #[should_panic(expected = "Failed to read config file")]
    fn test_load_config_missing_file() {
        // SAFETY: This test runs in isolation; mutating env vars is acceptable.
        unsafe {
            std::env::set_var("CONFIG_PATH", "/tmp/nonexistent_x402_config_12345.json");
        }
        let _config = load_config();
        unsafe {
            std::env::remove_var("CONFIG_PATH");
        }
    }
}
