# vidformer-igni

The next generation scale-out vidformer server.

## Development Setup

```bash
docker-compose -f docker-compose-db.yaml up
export 'IGNI_DB=postgres://igni:igni@localhost:5432/igni'
cargo run -- user add --name test --api-key test --permissions test
cargo run -- server --config igni.toml
```

## Deployment

```bash
# From vidformer project root
docker build -t igni -f vidformer-igni/Dockerfile .
cd vidformer-igni
docker-compose -f docker-compose-prod.yaml up
```