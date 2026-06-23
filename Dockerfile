# syntax=docker/dockerfile:1.6

FROM rust:1.83-slim-bookworm AS builder
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY src ./src
COPY migrations ./migrations
COPY assets ./assets
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin rankore

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/rankore /app/rankore
COPY --from=builder /app/assets /app/assets
ENV RUST_BACKTRACE=1
CMD ["/app/rankore"]
