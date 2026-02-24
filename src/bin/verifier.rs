use alloy_signer_local::PrivateKeySigner;
use dotenvy::dotenv;
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use reqwest::Client;
use sha3::{Digest, Keccak256};
use std::env;
use std::sync::Arc;
use x402_chain_eip155::{V1Eip155ExactClient, V2Eip155ExactClient};
use x402_reqwest::{ReqwestWithPayments, ReqwestWithPaymentsBuild, X402Client};

fn build_signing_message(
    request_method: &str,
    request_path_and_query: &str,
    request_body: &[u8],
    response_body: &[u8],
) -> Vec<u8> {
    let method = request_method.as_bytes();
    let path_and_query = request_path_and_query.as_bytes();

    let mut message = Vec::with_capacity(
        16 + method.len() + path_and_query.len() + request_body.len() + response_body.len(),
    );
    message.extend_from_slice(b"oyster-signature-v2\0");
    message.extend_from_slice(&(method.len() as u32).to_be_bytes());
    message.extend_from_slice(method);
    message.extend_from_slice(&(path_and_query.len() as u32).to_be_bytes());
    message.extend_from_slice(path_and_query);
    message.extend_from_slice(&(request_body.len() as u64).to_be_bytes());
    message.extend_from_slice(request_body);
    message.extend_from_slice(&(response_body.len() as u64).to_be_bytes());
    message.extend_from_slice(response_body);
    message
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let mut x402_client = X402Client::new();
    {
        let signer: Option<PrivateKeySigner> =
            env::var("EVM_PRIVATE_KEY").ok().and_then(|key| key.parse().ok());
        if let Some(signer) = signer {
            println!("Using EVM signer address: {:?}", signer.address());
            let signer = Arc::new(signer);
            x402_client = x402_client
                .register(V1Eip155ExactClient::new(signer.clone()))
                .register(V2Eip155ExactClient::new(signer));
            println!("Enabled eip155 exact scheme");
        }
    };

    let http_client = Client::new().with_payments(x402_client).build();

    let url = std::env::args()
        .nth(1)
        .expect("Usage: verifier <url>");
    let parsed_url: reqwest::Url = url.parse()?;
    let path_and_query = format!(
        "{}{}",
        parsed_url.path(),
        parsed_url
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default()
    );

    let response = http_client.get(url).send().await?;
    println!("Response Headers: {:?}", response.headers());

    let signature_hex = response
        .headers()
        .get("X-Signature")
        .expect("no X-Signature header")
        .to_str()
        .expect("invalid header value")
        .to_owned();
    let signature_bytes = hex::decode(&signature_hex).expect("invalid signature hex");
    if signature_bytes.len() != 65 {
        return Err("expected 65-byte signature".into());
    }

    let response_body = response.bytes().await?.to_vec();
    println!("Response: {:?}", String::from_utf8_lossy(&response_body));

    let signing_message = build_signing_message("GET", &path_and_query, b"", &response_body);

    let mut hasher = Keccak256::new();
    hasher.update(&signing_message);
    let hash = hasher.finalize();

    let signature =
        Signature::from_slice(&signature_bytes[0..64]).expect("failed to parse signature");

    let recid_byte = signature_bytes[64];
    let recovery_id = RecoveryId::from_byte(recid_byte - 27).expect("failed to parse recovery id");

    let verifying_key = VerifyingKey::recover_from_prehash(&hash, &signature, recovery_id)
        .expect("failed to recover pubkey");

    let pubkey = hex::encode(&verifying_key.to_encoded_point(false).as_bytes()[1..]);

    println!("Signature: {}", signature_hex);
    println!("Pubkey: {}", pubkey);

    Ok(())
}
