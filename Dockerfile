FROM rust:1-bookworm AS chef

ARG CARGO_CHEF_VERSION=0.1.78
ARG CARGO_LEPTOS_VERSION=0.3.12

# Install cargo-binstall, which makes it easier to install other
# cargo extensions like cargo-leptos
RUN wget --progress=dot:giga -O cargo-binstall.tgz \
  "https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-$(uname -m)-unknown-linux-musl.tgz" \
  && tar -xvf cargo-binstall.tgz \
  && cp cargo-binstall /usr/local/cargo/bin

# Install required tools
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends clang lld

# Keep the build tools identical in the planner, dependency, and application stages.
RUN cargo binstall cargo-leptos --version "${CARGO_LEPTOS_VERSION}" -y \
  && cargo install cargo-chef --version "${CARGO_CHEF_VERSION}" --locked \
  # Add the WASM target
  && rustup target add wasm32-unknown-unknown


WORKDIR /app
ENV RUSTFLAGS="--cfg erase_components"
# Match cargo-leptos's compile-time setting when warming Leptos dependencies.
ENV LEPTOS_OUTPUT_NAME="leptos-website-maj"

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# SSR and hydration use different features, profiles, and target directories.
RUN cargo chef cook --recipe-path recipe.json --locked --release \
  --no-default-features --features ssr --bin leptos-website-maj
RUN cargo chef cook --recipe-path recipe.json --locked \
  --no-default-features --features hydrate --profile wasm-release \
  --target wasm32-unknown-unknown --target-dir /app/target/front

COPY . .
# Build real sources, embed migrations, and generate the browser/CSS assets.
RUN cargo leptos build --release --cargo-locked -vv

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
COPY --from=builder /app/target/release/leptos-website-maj /app/leptos-website-maj
# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site
# Initial offline import sources are shipped separately from served assets.
COPY --from=builder /app/seed /app/seed
COPY --from=builder /app/Cargo.toml /app/Cargo.toml

# Set any required env variables and
ENV RUST_LOG="info"
ENV APP_ENVIRONMENT="production"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 8080

# Run the server
ENTRYPOINT [ "/app/leptos-website-maj" ]
