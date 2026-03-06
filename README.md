# x402 Gateway Service

A high performance, multi chain payment gateway built with Rust and Axum, implementing x402 (V2) to monetize HTTP APIs using USDC across EVM and Solana networks.

## Features

- **Multi-Chain Support**: Accept payments on multiple networks simultaneously (e.g., Base, Polygon, Solana).
- **x402 V2 Protocol**: Payment requirements returned in the `payment-required` header.
- **Per-Endpoint Pricing**: Configure different payment amounts for different routes.

## Configuration

The service is configured via a `config.json` file. You can set the path using the `CONFIG_PATH` environment variable (defaults to `config.json`).

### Example `config.json`

```json
{
  "gateway_port": 3000,
  "facilitator_url": "https://www.x402.org/facilitator",
  "target_api_url": "http://127.0.0.1:11434",
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
      "/api/version"
    ],
    "protected": [
      {
        "path": "/api/chat",
        "usdc_amount": 1
      }
    ]
  }
}
```

### Key Fields

- `gateway_port`: The port the gateway listens on (default: 3000).
- `facilitator_url`: The x402 facilitator service URL.
- `target_api_url`: The backend API URL to proxy requests to.
- `networks`: Array of supported blockchain networks.
  - `type`: `"evm"` or `"solana"`.
  - `network`: Network identifier (e.g., `"base-sepolia"`, `"solana-devnet"`).
  - `payment_address`: Your wallet address for receiving payments.
- `routes`:
  - `free`: List of public routes that bypass payment checks.
  - `protected`: List of routes requiring payment.
    - `path`: The URL path.
    - `usdc_amount`: Cost in USDC microunits (e.g., 1000 = 0.001 USDC).

### Environment Variables

| Variable | Description | Default |
|---|---|---|
| `CONFIG_PATH` | Path to `config.json` | `config.json` |

## Running Locally

1. **Install Rust**: Ensure you have Rust and Cargo installed.
2. **Configure**: Copy `config.example.json` to `config.json` and update with your details.
3. **Run**:
   ```bash
   cargo run
   ```
   Or with custom config path:
   ```bash
   CONFIG_PATH=production.json cargo run --release
   ```

## Ollama Setup (AI Chat Example)

This setup demonstrates monetizing an Ollama LLM behind the x402 gateway.

### Prerequisites

- [Ollama](https://ollama.ai) installed locally, or use the Docker Compose setup below.

### Local (without Docker)

1. Start Ollama:
   ```bash
   ollama serve
   ```
2. Pull a model:
   ```bash
   ollama pull qwen3:0.6b
   ```
3. Update `config.json` to point to Ollama:
   ```json
   {
     "target_api_url": "http://127.0.0.1:11434",
     "routes": {
       "free": ["/api/version"],
       "protected": [{ "path": "/api/chat", "usdc_amount": 1 }]
     }
   }
   ```
4. Run the gateway:
   ```bash
   cargo run
   ```
5. Test a free route:
   ```bash
   curl http://localhost:3000/api/version
   ```
6. Test a protected route (returns 402):
   ```bash
   curl -v http://localhost:3000/api/chat
   ```

### Docker Compose

The `docker-compose.yml` bundles the gateway with Ollama and auto-pulls the `qwen3:0.6b` model:

```yaml
services:
  x402-gateway:
    image: sagarparker/x402-gateway:latest
    network_mode: host
    environment:
      - CONFIG_PATH=/init-params/config.json
    volumes:
      - /init-params:/init-params:ro

  ollama_server:
    image: alpine/ollama:0.10.1
    network_mode: host

  ollama_model:
    image: alpine/ollama:0.10.1
    command: pull qwen3:0.6b
    network_mode: host
    depends_on:
      ollama_server:
        condition: service_healthy
```

## Deploy on Oyster CVM

You can deploy the gateway to an Oyster CVM enclave. The config file is provided externally via init-params.

1. **Simulate locally** (for testing):
   ```bash
   oyster-cvm simulate --docker-compose docker-compose.yml --init-params "config.json:1:0:file:./config.json"
   ```

2. **Deploy to Oyster CVM**:
   ```bash
   oyster-cvm deploy \
     --wallet-private-key <key> \
     --duration-in-minutes 30 \
     --arch amd64 \
     --docker-compose docker-compose.yml \
     --init-params "config.json:1:0:file:./config.json"
   ```

### Init Params Format

The `--init-params` flag follows the format: `<enclave_path>:<attest>:<encrypt>:<type>:<value>`

- `config.json` — placed at `/init-params/config.json` inside the enclave
- `1` — included in attestation
- `0` — not encrypted (use `1` if your config contains secrets)
- `file` — read from a local file
- `./config.json` — path to the local config file

## Usage

### Protocol
Access protected routes directly. The gateway returns `402 Payment Required` with payment details in the `payment-required` header if no valid payment is present.

```bash
curl -v http://localhost:3000/api/chat
```

## Supported Networks

**EVM Mainnets**: Base, Polygon, Avalanche, Sei, XDC, XRPL EVM, Peaq, IoTeX, Celo

**EVM Testnets**: Base Sepolia, Polygon Amoy, Avalanche Fuji, Sei Testnet, Celo Sepolia

**Solana**: Mainnet, Devnet
