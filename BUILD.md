# Building Instructions

Notes on building for development and publishing to production.

1. You need rust installed, including the WASM target (`rustup target add wasm32-unknown-unknown`)
2. Install:

- mold
- cargo-leptos
- flyctl
- docker

3. Make sure docker is running or `sudo systemctl start docker`

## Local Dev

1. Build the docker image with: `sudo docker build -t maj-web .`
2. Run the docker image with: `sudo docker run -p 8080:8080 maj-web`

## Publishing to Fly.io

1. `fly deploy` to build on a Fly machinge or add `--local-only` to build locally
