# Build stage
FROM rust:1-bookworm AS builder

WORKDIR /usr/src/x402-gateway
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/local/bin
COPY --from=builder /usr/src/x402-gateway/target/release/x402-gateway .
COPY --from=builder /usr/src/x402-gateway/config.docker.json .

# Expose the gateway port
EXPOSE 3000

# Run the binary
CMD ["./x402-gateway"]
