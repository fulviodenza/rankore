FROM rust:latest

WORKDIR rankore
COPY ./Cargo.toml .

COPY ./src ./src
COPY .sqlx/ ./.sqlx/
RUN mkdir ./tmp
COPY ./assets ./assets
COPY README.md ./README.md
RUN SQLX_OFFLINE=true cargo build --release

CMD ["target/release/rankore"]
