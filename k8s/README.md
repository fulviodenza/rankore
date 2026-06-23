# Rankore — Kubernetes deployment

Single-node-friendly manifests targeting the k3s cluster in `~/.kube/config`.
Postgres is bundled as a StatefulSet in the `rankore` namespace; the bot image
is pulled from GHCR.

## Prerequisites

- A k3s/Kubernetes cluster with at least one node and the `local-path-ssd`
  storage class (substitute another class in `20-postgres.yaml` if needed).
- A Discord bot token.
- An image pushed to `ghcr.io/fulviodenza/rankore`.

## Build and push the image

From the repo root:

```sh
# Multi-arch is nice but not required for a single-node cluster.
docker build -t ghcr.io/fulviodenza/rankore:latest .

# Authenticate against GHCR (one-time):
echo "$GH_PAT" | docker login ghcr.io -u fulviodenza --password-stdin

docker push ghcr.io/fulviodenza/rankore:latest
```

Tag with a SHA or semver for production rollouts:
`ghcr.io/fulviodenza/rankore:$(git rev-parse --short HEAD)`.

## Apply

```sh
kubectl apply -f k8s/00-namespace.yaml

# Fill in the secrets template, then apply (do not commit the filled copy):
cp k8s/10-secrets.example.yaml k8s/10-secrets.yaml
$EDITOR k8s/10-secrets.yaml
kubectl apply -f k8s/10-secrets.yaml

# If the GHCR image is private, also create an image-pull secret:
kubectl -n rankore create secret docker-registry ghcr-pull \
  --docker-server=ghcr.io \
  --docker-username=fulviodenza \
  --docker-password=$GH_PAT_READ_PACKAGES
# then uncomment `imagePullSecrets` in 30-bot.yaml.

kubectl apply -f k8s/20-postgres.yaml
kubectl -n rankore rollout status statefulset/rankore-postgres --timeout=120s

kubectl apply -f k8s/30-bot.yaml
kubectl -n rankore rollout status deployment/rankore --timeout=120s
```

## Verify

```sh
kubectl -n rankore get pods
kubectl -n rankore logs deploy/rankore -f
```

The bot runs migrations on startup via `sqlx::migrate!()`, so the first
deployment creates the schema automatically. Subsequent deploys re-run only
pending migrations.

## Upgrade

```sh
docker build -t ghcr.io/fulviodenza/rankore:$(git rev-parse --short HEAD) .
docker push ghcr.io/fulviodenza/rankore:$(git rev-parse --short HEAD)
kubectl -n rankore set image deployment/rankore \
  rankore=ghcr.io/fulviodenza/rankore:$(git rev-parse --short HEAD)
kubectl -n rankore rollout status deployment/rankore
```

## Tear down

```sh
kubectl delete namespace rankore
# Note: the PVC for Postgres is in the namespace and will be deleted too.
# Back up the data first if you care about it.
```

## Notes

- The bot runs as a single replica (`strategy: Recreate`). Discord bots cannot
  shard from a single token without explicit sharding setup, and this bot does
  not.
- No HTTP liveness probe. The container is alive iff the Rust process is
  running; k8s will restart on crash.
- `DATABASE_URL` lives in the `rankore-postgres` secret rather than the bot's
  own secret, so the password is defined exactly once.
