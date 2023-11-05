FROM rust:1.70.0
WORKDIR /usr/src/rankore
COPY . .
RUN cargo build --release
RUN chmod +x ./target/release/rankore
CMD ["./target/release/rankore"]
