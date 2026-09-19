# syntax=docker/dockerfile:1

FROM rust:1-slim-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli --version 0.7.10 --locked
WORKDIR /app
COPY . .

# Baked into the WASM bundle at compile time — the browser running this code
# can't resolve a runtime env var the way a server process would. See src/api.rs.
ARG API_BASE_URL
ENV API_BASE_URL=${API_BASE_URL}

RUN dx build --release

FROM caddy:2-alpine AS prod
COPY --from=builder /app/target/dx/leitsys_web/release/web/public /srv
COPY Caddyfile /etc/caddy/Caddyfile
EXPOSE 80
