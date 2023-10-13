FROM rust:1.70.0
WORKDIR /usr/src/rankore
COPY . .
ENV DISCORD_TOKEN my_fantastic_discord_token
RUN cargo build --release
RUN chmod +x ./target/release/rankore
CMD ["./target/release/rankore"]
