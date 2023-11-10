if [ -f .env ]
then
    export $(cat .env | xargs)
fi
docker compose up -d
cargo run
