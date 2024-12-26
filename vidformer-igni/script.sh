#!/usr/bin/env bash
exit

# docker exec -it igni_db psql -U igni -d igni

cargo run source ls
cargo run source add --name "../tos_720p.mp4" --stream-idx 0 --storage-service fs --storage-config '{"root":"."}'
cargo run source rm ...

cargo run spec ls
cargo run spec add --width 1280 --height 720 --pix-fmt yuv420p --segment-length 2/1

curl http://localhost:8080/

curl -X POST -H "Content-Type: application/json" -d '{"name":"../tos_720p.mp4","stream_idx":0,"storage_service":"fs","storage_config":{"root":"."}}' http://localhost:8080/v2/source

curl http://localhost:8080/v2/source/<uuid>
