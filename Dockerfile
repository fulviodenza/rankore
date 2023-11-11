FROM rust:1.73.0-bookworm

WORKDIR rankore

COPY ./Cargo.toml .
COPY ./src ./src
COPY .sqlx/ ./.sqlx/
COPY ./migrations ./migrations
COPY README.md ./README.md

RUN cargo install sqlx-cli
ARG DATABASE_URL_EXTERNAL
ARG DATABASE_URL
RUN sqlx migrate run --database-url "${DATABASE_URL_EXTERNAL}"

RUN SQLX_OFFLINE=true cargo build
CMD ["target/release/rankore"]
