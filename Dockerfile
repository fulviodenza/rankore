FROM rust:latest

WORKDIR rankore

COPY ./Cargo.toml .
COPY ./src ./src
COPY .sqlx/ ./.sqlx/
COPY ./migrations ./migrations
COPY README.md ./README.md

RUN cargo install sqlx-cli
ARG DATABASE_URL
RUN sqlx migrate run --database-url "${DATABASE_URL_EXTERNAL}"

RUN SQLX_OFFLINE=true cargo build
CMD ["target/release/rankore"]
