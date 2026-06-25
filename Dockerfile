# syntax=docker/dockerfile:1.6

FROM rust:1.88-slim-bookworm AS builder
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config libssl-dev libopus-dev cmake build-essential ca-certificates \
        clang libclang-dev \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY src ./src
COPY migrations ./migrations
COPY assets ./assets
ENV SQLX_OFFLINE=true
ENV OPUS_USE_PKG_CONFIG=1
RUN cargo build --release --bin rankore

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 libopus0 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/rankore /app/rankore
COPY --from=builder /app/assets /app/assets
ENV RUST_BACKTRACE=1
CMD ["/app/rankore"]
