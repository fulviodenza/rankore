FROM rust:latest

WORKDIR rankore

COPY ./Cargo.toml .
COPY ./src ./src
COPY .sqlx/ ./.sqlx/
COPY ./migrations ./migrations
COPY README.md ./README.md

RUN cargo install sqlx-cli
RUN sqlx migrate run

RUN SQLX_OFFLINE=true cargo build
CMD ["target/release/rankore"]
