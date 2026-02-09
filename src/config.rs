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
