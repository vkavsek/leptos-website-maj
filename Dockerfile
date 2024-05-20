FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

RUN apt update && apt install mold clang -y 
RUN cargo install --locked cargo-leptos
RUN rustup component add rust-src
RUN rustup target add wasm32-unknown-unknown


########################################

FROM chef AS planner

# Copy contents of current DIR to the image
COPY . .
# Compute a lock-like file for our project 
RUN cargo chef prepare --recipe-path recipe.json

########################################

FROM chef AS builder 

COPY --from=planner /app/recipe.json recipe.json
# Build our project dependecies, not our app!
RUN cargo chef cook --release --recipe-path recipe.json
# If our dependency tree hasn't changed, everything should be cached up to now!
COPY . .

RUN cargo leptos build --release -vv


########################################

FROM debian:bookworm-slim AS runtime 
WORKDIR /app

# Install OpenSSL - it's dynamically linked by some of our dependencies.
# Install ca-certificates - it's needed to verify TLS certificates when establishing HTTPS con.
RUN apt-get update -y \
	&& apt-get install -y --no-install-recommends openssl ca-certificates \
	&& apt-get autoremove -y \
	&& apt-get clean -y \
	&& rm -rf /var/lib/apt/lists/*


# Copy the compiled server binary from builder to runtime.
COPY --from=builder /app/target/release/maj-fullstack maj-fullstack
# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site
COPY --from=builder /app/Cargo.toml /app/Cargo.toml
# # Set any required env variables and
ENV RUST_LOG info
ENV APP_ENVIRONMENT production
ENV LEPTOS_SITE_ADDR 0.0.0.0:8080
ENV LEPTOS_SITE_ROOT site

EXPOSE 8080
# Run the server

ENTRYPOINT [ "./maj-fullstack" ]


# # If you’re using stable, use this instead
# FROM rust:lastest-bullseye as builder
#
# RUN apt-get update && \
#   apt-get install -y binaryen npm protobuf-compiler libssl-dev pkg-config musl-tools \
#   && rm -rf /var/lib/apt/lists/*
#
# RUN cargo install --locked cargo-leptos
# RUN rustup component add rust-src
# RUN rustup target add wasm32-unknown-unknown
#
# # Make an /app dir, which everything will eventually live in
# RUN mkdir -p /app
# WORKDIR /app
# COPY . .
#
# # Build the app
# RUN cargo leptos build --release -vv
#
# FROM rust:1.75-bullseye as runner
# # Copy the server binary to the /app directory
# COPY --from=builder /app/target/release/maj-fullstack /app/
# # /target/site contains our JS/WASM/CSS, etc.
# COPY --from=builder /app/target/site /app/site
# # Copy Cargo.toml if it’s needed at runtime
# COPY --from=builder /app/Cargo.toml /app/
# WORKDIR /app
#
# # Set any required env variables and
# ENV RUST_LOG="info"
# ENV APP_ENVIRONMENT="production"
# ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
# ENV LEPTOS_SITE_ROOT="site"
# EXPOSE 8080
# # Run the server
# CMD ["/app/maj-fullstack"]
