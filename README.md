# x402 Gateway Service

A high performance, multi chain payment gateway built with Rust and Axum, implementing x402 (V1 and V2) to monetize HTTP APIs using USDC across EVM and Solana networks, with integrated Oyster CVM TEE signature verification for secure, enclave backed request verification.

## Features

- **Multi-Chain Support**: Accept payments on multiple networks simultaneously (e.g., Base, Polygon, Solana).
- **Dual Protocol Support**: Fully supports both x402 V1 and V2 protocols.
- **Per-Endpoint Pricing**: Configure different payment amounts for different routes.
- **TEE Signatures**: Responses are signed using a secp256k1 key (via Oyster KMS or env var) for enclave-backed verification.

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
| `SIGNING_PRIVATE_KEY_HEX` | Hex-encoded 32-byte secp256k1 private key for signing responses | — |
| `SIGNING_KEY_DERIVE_URL` | URL to derive signing key from KMS | `http://127.0.0.1:1100/derive/secp256k1?path=signing-server` |

> If `SIGNING_PRIVATE_KEY_HEX` is set, it takes priority. Otherwise the gateway fetches the key from the KMS derive URL (used in Oyster CVM deployments).

## Running Locally

1. **Install Rust**: Ensure you have Rust and Cargo installed.
2. **Configure**: Copy `config.example.json` to `config.json` and update with your details.
3. **Set signing key** (for local dev):
   ```bash
   export SIGNING_PRIVATE_KEY_HEX="your_64_char_hex_private_key"
   ```
4. **Run**:
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
   SIGNING_PRIVATE_KEY_HEX="your_key" cargo run
   ```
5. Test a free route:
   ```bash
   curl http://localhost:3000/api/version
   ```
6. Test a protected route (returns 402):
   ```bash
   curl -v http://localhost:3000/api/chat-v2
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

### V1 Protocol
Access protected routes directly. The gateway returns `402 Payment Required` with payment details in the body if no valid payment header is present.

```bash
curl http://localhost:3000/api/chat
```

### V2 Protocol
Append `-v2` to your configured protected routes (e.g., `/api/chat-v2`). The gateway returns payment requirements in the `payment-required` header.

```bash
curl -v http://localhost:3000/api/chat-v2
```

## Supported Networks

**EVM Mainnets**: Base, Polygon, Avalanche, Sei, XDC, XRPL EVM, Peaq, IoTeX, Celo

**EVM Testnets**: Base Sepolia, Polygon Amoy, Avalanche Fuji, Sei Testnet, Celo Sepolia

**Solana**: Mainnet, Devnet

## Verifying Signatures

> Note: Set `SIGNING_PRIVATE_KEY_HEX` (for local) or deploy on Oyster CVM (for KMS-derived keys).

### Using the Verifier CLI

Recover the public key from a signed response:

```bash
cargo run --bin verifier -- http://<ENCLAVE_IP>:8888/your-endpoint
```

### Using KMS Derive

Get the expected public key directly from the KMS:

```bash
oyster-cvm kms-derive \
  --image-id <IMAGE_ID> \
  --path signing-server \
  --key-type secp256k1/public
```

### Verification

**The public key from the verifier should match the public key from `kms-derive`.** This confirms that:

1. The response was signed by a valid Oyster enclave
2. The enclave is running the expected image (identified by `image-id`)
3. The signature was created using the KMS-derived key for `signing-server` path

## Signature Format

The `X-Signature` header contains a 65-byte hex-encoded signature:

- Bytes 0–63: ECDSA signature (r, s)
- Byte 64: Recovery ID + 27 (Ethereum-style)

The signed message is the Keccak256 hash of:

```text
"oyster-signature-v2\0" ||
u32be(len(request_method)) || request_method ||
u32be(len(request_path_and_query)) || request_path_and_query ||
u64be(len(request_body)) || request_body ||
u64be(len(response_body)) || response_body
```
