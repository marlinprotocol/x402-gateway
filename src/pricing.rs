use crate::config::NetworkConfig;
use alloy_primitives::Address;
use std::{str::FromStr, sync::Arc};

use x402_axum::{
    StaticPriceTags, X402LayerBuilder, X402Middleware, facilitator_client::FacilitatorClient,
};
use x402_chain_eip155::{KnownNetworkEip155, V1Eip155Exact, V2Eip155Exact};
use x402_chain_solana::{KnownNetworkSolana, V1SolanaExact, V2SolanaExact};
use x402_types::{
    networks::USDC, proto::v1::PriceTag as V1PriceTag, proto::v2::PriceTag as V2PriceTag,
};

/// Get USDC deployment for EVM networks
fn get_evm_usdc(network: &str) -> x402_chain_eip155::chain::Eip155TokenDeployment {
    match network {
        // Mainnets
        "base" => USDC::base(),
        "polygon" => USDC::polygon(),
        "avalanche" => USDC::avalanche(),
        "sei" => USDC::sei(),
        "xdc" => USDC::xdc(),
        "xrpl-evm" => USDC::xrpl_evm(),
        "peaq" => USDC::peaq(),
        "iotex" => USDC::iotex(),
        "celo" => USDC::celo(),
        // Testnets
        "base-sepolia" | "base_sepolia" => USDC::base_sepolia(),
        "polygon-amoy" | "polygon_amoy" => USDC::polygon_amoy(),
        "avalanche-fuji" | "avalanche_fuji" => USDC::avalanche_fuji(),
        "sei-testnet" | "sei_testnet" => USDC::sei_testnet(),
        "celo-sepolia" | "celo_sepolia" => USDC::celo_sepolia(),
        _ => panic!("Unsupported EVM network: {}", network),
    }
}

/// Get USDC deployment for Solana networks
fn get_solana_usdc(network: &str) -> x402_chain_solana::chain::SolanaTokenDeployment {
    match network {
        "solana" | "solana-mainnet" => USDC::solana(),
        "solana-devnet" | "solana_devnet" => USDC::solana_devnet(),
        _ => panic!("Unsupported Solana network: {}", network),
    }
}

/// Parse Solana address from string
fn parse_solana_address(address: &str) -> x402_chain_solana::chain::Address {
    x402_chain_solana::chain::Address::from_str(address).expect("Invalid Solana address")
}

/// Build V1 price tags layer for a specific route
pub fn build_v1_layer(
    x402: &X402Middleware<Arc<FacilitatorClient>>,
    networks: &[NetworkConfig],
    usdc_amount: u64,
) -> X402LayerBuilder<StaticPriceTags<V1PriceTag>, Arc<FacilitatorClient>> {
    // Collect all price tags first
    let mut tags: Vec<V1PriceTag> = Vec::new();

    for net_config in networks {
        let tag = match net_config {
            NetworkConfig::Evm {
                network,
                payment_address,
            } => {
                let address: Address = payment_address.parse().expect("Invalid EVM address");
                let usdc = get_evm_usdc(network);
                V1Eip155Exact::price_tag(address, usdc.amount(usdc_amount))
            }
            NetworkConfig::Solana {
                network,
                payment_address,
            } => {
                let solana_addr = parse_solana_address(payment_address);
                let usdc = get_solana_usdc(network);
                V1SolanaExact::price_tag(solana_addr, usdc.amount(usdc_amount))
            }
        };
        tags.push(tag);
    }

    if tags.is_empty() {
        panic!("At least one network must be configured");
    }

    // Initialize builder with first tag
    let mut builder = x402.with_price_tag(tags[0].clone());

    // Add remaining tags
    for tag in tags.into_iter().skip(1) {
        builder = builder.with_price_tag(tag);
    }

    builder
}

/// Build V2 price tags layer for a specific route
pub fn build_v2_layer(
    x402: &X402Middleware<Arc<FacilitatorClient>>,
    networks: &[NetworkConfig],
    usdc_amount: u64,
) -> X402LayerBuilder<StaticPriceTags<V2PriceTag>, Arc<FacilitatorClient>> {
    // Collect all price tags first
    let mut tags: Vec<V2PriceTag> = Vec::new();

    for net_config in networks {
        let tag = match net_config {
            NetworkConfig::Evm {
                network,
                payment_address,
            } => {
                let address: Address = payment_address.parse().expect("Invalid EVM address");
                let usdc = get_evm_usdc(network);
                V2Eip155Exact::price_tag(address, usdc.amount(usdc_amount))
            }
            NetworkConfig::Solana {
                network,
                payment_address,
            } => {
                let solana_addr = parse_solana_address(payment_address);
                let usdc = get_solana_usdc(network);
                V2SolanaExact::price_tag(solana_addr, usdc.amount(usdc_amount))
            }
        };
        tags.push(tag);
    }

    if tags.is_empty() {
        panic!("At least one network must be configured");
    }

    // Initialize builder with first tag
    let mut builder = x402.with_price_tag(tags[0].clone());

    // Add remaining tags
    for tag in tags.into_iter().skip(1) {
        builder = builder.with_price_tag(tag);
    }

    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_evm_usdc_known_mainnets() {
        // Should not panic for any supported mainnet
        let networks = [
            "base", "polygon", "avalanche", "sei", "xdc", "xrpl-evm", "peaq", "iotex", "celo",
        ];
        for network in &networks {
            let _usdc = get_evm_usdc(network);
        }
    }

    #[test]
    fn test_get_evm_usdc_known_testnets() {
        let networks = [
            "base-sepolia",
            "polygon-amoy",
            "avalanche-fuji",
            "sei-testnet",
            "celo-sepolia",
        ];
        for network in &networks {
            let _usdc = get_evm_usdc(network);
        }
    }

    #[test]
    fn test_get_evm_usdc_testnet_aliases() {
        // Underscore aliases should also work
        let aliases = [
            "base_sepolia",
            "polygon_amoy",
            "avalanche_fuji",
            "sei_testnet",
            "celo_sepolia",
        ];
        for alias in &aliases {
            let _usdc = get_evm_usdc(alias);
        }
    }

    #[test]
    #[should_panic(expected = "Unsupported EVM network")]
    fn test_get_evm_usdc_unsupported_network() {
        get_evm_usdc("unknown-chain");
    }

    #[test]
    fn test_get_solana_usdc_known_networks() {
        let networks = ["solana", "solana-mainnet", "solana-devnet", "solana_devnet"];
        for network in &networks {
            let _usdc = get_solana_usdc(network);
        }
    }

    #[test]
    #[should_panic(expected = "Unsupported Solana network")]
    fn test_get_solana_usdc_unsupported_network() {
        get_solana_usdc("solana-unknown");
    }

    #[test]
    fn test_parse_solana_address_valid() {
        // A valid base58 Solana public key (32 bytes)
        let addr = parse_solana_address("EGBQqKn968sVv5cQh5Cr72pSTHfxsuzq7o7asqYB5uEV");
        let addr_str = addr.to_string();
        assert_eq!(addr_str, "EGBQqKn968sVv5cQh5Cr72pSTHfxsuzq7o7asqYB5uEV");
    }

    #[test]
    #[should_panic(expected = "Invalid Solana address")]
    fn test_parse_solana_address_invalid() {
        parse_solana_address("not-a-valid-solana-address!!!");
    }
}
