# vidformer-igni

The vidformer server for the cloud.

**Quick links:**
* [🧑‍💻 Source Code](https://github.com/ixlab/vidformer/tree/main/vidformer-igni/)

## Local Setup

See the [install guide](https://ixlab.github.io/vidformer/docs/install.html).

## Development Setup

```bash
docker-compose -f docker-compose-db.yaml up
export 'IGNI_DB=postgres://igni:igni@localhost:5432/igni'
cargo run -- user add --name test --api-key test --permissions test
cargo run --release -- server --config igni.toml
```

## Server Deployment

```bash
# From vidformer project root
docker build -t igni -f Dockerfile .
docker-compose -f vidformer-igni/docker-compose-prod.yaml up
```

For TLS certs:
```bash
docker-compose -f vidformer-igni/docker-compose-prod.yaml run --rm certbot certonly --webroot --webroot-path /var/www/certbot/ -d api.example.com -d cdn.example.com
```

## Guest account setup (for colab notebook)

```bash
docker ps
docker exec -it <igni container> bash
vidformer-igni user add --name guest --permissions guest --api-key VF_GUEST
vidformer-igni user ls

# ToS video
vidformer-igni source add --user-id 98f6aa2a-e622-40bc-a0cd-e05f73f7e398 --name vf-sample-media/tos_720p.mp4 --stream-idx 0 --storage-service http --storage-config '{"endpoint":"https://f.dominik.win"}'
vidformer-igni source add --user-id 98f6aa2a-e622-40bc-a0cd-e05f73f7e398 --name vf-sample-media/bbb_720p.mp4 --stream-idx 0 --storage-service http --storage-config '{"endpoint":"https://f.dominik.win"}'
vidformer-igni source add --user-id 98f6aa2a-e622-40bc-a0cd-e05f73f7e398 --name vf-sample-media/tos_720p-yolov8x-seg-masks.mkv --stream-idx 0 --storage-service http --storage-config '{"endpoint":"https://f.dominik.win"}'

# Apollo 11 videos
for i in $(seq -f "%02g" 1 23); do
  vidformer-igni source add --user-id 98f6aa2a-e622-40bc-a0cd-e05f73f7e398 --name "vf-sample-media/apollo-11-mission/Apollo 11 $i.mp4" --stream-idx 0 --storage-service http --storage-config '{"endpoint":"https://f.dominik.win"}'
done
```