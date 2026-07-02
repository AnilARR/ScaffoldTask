# Multi-stage build for the Rust API.
FROM rust:1.82-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Build the sources (simple, robust single-stage compile).
COPY . .
RUN cargo build --release --bin scaffold-api

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/scaffold-api /usr/local/bin/scaffold-api
COPY backend/migrations /app/backend/migrations
ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
CMD ["scaffold-api"]
