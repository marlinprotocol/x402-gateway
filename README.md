# x402 Gateway Service

A high-performance, multi-chain payment gateway/proxy built with Rust and Axum. It implements the [x402](https://www.x402.org) protocol (V1 and V2) to monetize HTTP APIs using USDC on EVM and Solana networks.

## Features

- **Multi-Chain Support**: Accept payments on multiple networks simultaneously (e.g., Base, Polygon, Solana).
- **Dual Protocol Support**: Fully supports both x402 V1 and V2 protocols.
- **Per-Endpoint Pricing**: Configure different payment amounts for different routes (e.g., higher cost for VIP endpoints).

## Configuration

The service is configured via a `config.json` file. You can set the path using the `CONFIG_PATH` environment variable (defaults to `config.json`).

### Example `config.json`

```json
{
  "gateway_port": 3000,
  "facilitator_url": "https://www.x402.org/facilitator",
  "target_api_url": "http://127.0.0.1:3001",
  "networks": [
    {
      "type": "evm",
      "network": "base-sepolia",
      "payment_address": "0xYOUR_EVM_ADDRESS"
    },
    {
      "type": "solana",
      "network": "solana-devnet",
      "payment_address": "YOUR_SOLANA_PUBKEY"
    }
  ],
  "routes": {
    "free": [
      "/health",
      "/public"
    ],
    "protected": [
      {
        "path": "/protected",
        "usdc_amount": 1000
      },
      {
        "path": "/vip",
        "usdc_amount": 10000
      }
    ]
  }
}
```

### Key Fields

- `gateway_port`: The port the gateway listens on (default: 3000).
- `target_api_url`: The backend API URL to proxy requests to.
- `networks`: Array of supported blockchain networks.
  - `type`: "evm" or "solana".
  - `network`: Network identifier (e.g., "base", "polygon", "solana-mainnet").
  - `payment_address`: Your wallet address for receiving payments.
- `routes`:
  - `free`: List of public routes that bypass payment checks.
  - `protected`: List of routes requiring payment.
    - `path`: The URL path.
    - `usdc_amount`: Cost in USDC microunits (e.g., 1000 = 0.001 USDC).

## Running the Service

1.  **Install Rust**: Ensure you have Rust and Cargo installed.
2.  **Configure**: Copy `config.example.json` to `config.json` and update with your details.
3.  **Run**:
    ```bash
    cargo run
    ```
    Or with custom config path:
    ```bash
    CONFIG_PATH=production.json cargo run --release
    ```

## Docker Support

You can run the entire stack (gateway + http-server) using Docker Compose.

1.  **Navigate to the gateway directory**:
    ```bash
    cd x402-gateway
    ```
2.  **Start the services**:
    ```bash
    docker-compose up --build
    ```
3.  **Access**:
    - Gateway: `http://localhost:3000`
    - Backend: `http://localhost:3001` (internal only, unless ports mapped)

The Docker setup uses `config.docker.json` which treats the backend as `http://backend:3001`.

## Usage

### V1 Protocol
Access protected routes directly. The gateway will return `402 Payment Required` with payment details in the body if no valid payment header is present.

```bash
curl http://localhost:3000/protected
```

### V2 Protocol
Append `-v2` to your configured protected routes (e.g., `/protected-v2`). The gateway returns payment requirements in the `payment-required` header.

```bash
curl -v http://localhost:3000/protected-v2
```

## Supported Networks

**EVM Mainnets**:
- Base: `base`
- Polygon: `polygon`
- Avalanche: `avalanche`
- Sei: `sei`
- XDC: `xdc`
- XRPL EVM: `xrpl-evm`
- Peaq: `peaq`
- IoTeX: `iotex`
- Celo: `celo`

**EVM Testnets**:
- Base Sepolia: `base-sepolia`
- Polygon Amoy: `polygon-amoy`
- Avalanche Fuji: `avalanche-fuji`
- Sei Testnet: `sei-testnet`
- Celo Sepolia: `celo-sepolia`

**Solana**:
- Solana Mainnet
- Solana Devnet
