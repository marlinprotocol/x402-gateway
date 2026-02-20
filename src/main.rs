mod config;
mod handlers;
mod pricing;
mod state;

use axum::{Router, routing::any};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use tracing::info;
use x402_axum::X402Middleware;

use crate::config::{NetworkConfig, load_config};
use crate::handlers::proxy_request;
use crate::pricing::{build_v1_layer, build_v2_layer};
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let config = load_config();

    // Log configured networks
    for net in &config.networks {
        match net {
            NetworkConfig::Evm {
                network,
                payment_address,
            } => {
                info!(network = %network, address = %payment_address, chain_type = "EVM", "Configured network");
            }
            NetworkConfig::Solana {
                network,
                payment_address,
            } => {
                info!(network = %network, address = %payment_address, chain_type = "Solana", "Configured network");
            }
        }
    }

    info!(
        facilitator = %config.facilitator_url,
        target_api = %config.target_api_url,
        network_count = config.networks.len(),
        free_routes = ?config.routes.free,
        protected_routes_count = config.routes.protected.len(),
        gateway_port = config.gateway_port,
        "Loaded configuration"
    );

    // Create x402 middleware
    let x402 = X402Middleware::try_from(config.facilitator_url.as_str())?;

    let state = Arc::new(AppState::new(config.clone()));

    // Build router dynamically from config
    let mut app = Router::new();

    // Add free routes (no payment required)
    for route in &config.routes.free {
        info!(route = %route, "Registering FREE route");
        app = app.route(route, any(proxy_request));
    }

    // Add protected routes with V1 price tags (all configured networks)
    for route_config in &config.routes.protected {
        info!(route = %route_config.path, amount = route_config.usdc_amount, protocol = "V1", "Registering PROTECTED route");
        let v1_layer = build_v1_layer(&x402, &config.networks, route_config.usdc_amount);
        app = app.route(&route_config.path, any(proxy_request).layer(v1_layer));
    }

    // Add V2 protected routes with -v2 suffix (all configured networks)
    for route_config in &config.routes.protected {
        let v2_route = format!("{}-v2", route_config.path);
        info!(route = %v2_route, amount = route_config.usdc_amount, protocol = "V2", "Registering PROTECTED route");
        let v2_layer = build_v2_layer(&x402, &config.networks, route_config.usdc_amount);
        app = app.route(&v2_route, any(proxy_request).layer(v2_layer));
    }

    // Add CORS layer to allow frontend requests
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any);

    // Add state and CORS to the router
    let app = app.layer(cors).with_state(state);

    let address = format!("0.0.0.0:{}", config.gateway_port);
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect(&format!("Failed to bind to {}", address));

    info!(address = %address, "x402 Gateway started");

    axum::serve(listener, app).await?;

    Ok(())
}
