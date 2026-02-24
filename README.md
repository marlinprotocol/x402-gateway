# x402 Gateway Service

A high performance, multi chain payment gateway built with Rust and Axum, implementing x402 (V1 and V2) to monetize HTTP APIs using USDC across EVM and Solana networks, with integrated Oyster CVM TEE signature verification for secure, enclave backed request verification.

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

## Test and Deploy on Oyster CVM

You can test locally and deploy the gateway to an Oyster CVM enclave. The config file is provided externally via init-params.

1.  **Navigate to the gateway directory**:
    ```bash
    cd x402-gateway
    ```
2.  **Configure**: Copy `config.example.json` to `config.json` and update with your details.

3.  **Simulate locally** (for testing):
    ```bash
    oyster-cvm simulate --docker-compose docker-compose.yml --init-params "config.json:1:0:file:./config.json"
    ```

4.  **Deploy to Oyster CVM**:
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

- `config.json` — file is placed at `/init-params/config.json` inside the enclave
- `1` — included in attestation
- `0` — not encrypted (use `1` if your config contains secrets)
- `file` — read from a local file
- `./config.json` — path to the local config file

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

## Verifying Signatures

> Note : Set your EVM_PRIVATE_KEY and/or SOLANA_PRIVATE_KEY inside `.env`

### Using the Verifier CLI

Recover the public key from a signed response:

```bash
cargo run --bin verifier -- http://<ENCLAVE_IP>:8888/your-endpoint
```

Output:
```
Signature: <hex-encoded signature>
Pubkey: <recovered public key>
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

- Bytes 0-63: ECDSA signature (r, s)
- Byte 64: Recovery ID + 27 (Ethereum-style)

The message being signed is the Keccak256 hash of a structured payload containing:

- Request method
- Request path + query
- Request body
- Response body

The payload format is:

```text
"oyster-signature-v2\0" ||
u32be(len(request_method)) || request_method ||
u32be(len(request_path_and_query)) || request_path_and_query ||
u64be(len(request_body)) || request_body ||
u64be(len(response_body)) || response_body
```
