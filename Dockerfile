FROM rust:latest

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

COPY src/ ./src/
COPY .sqlx/ ./.sqlx/
RUN NIXPACKS_NO_MUSL=1 SQLX_OFFLINE=true cargo build --release

CMD ["target/release/rankore"]
