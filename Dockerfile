FROM rust:1-bookworm AS builder

# Install cargo-binstall, which makes it easier to install other
# cargo extensions like cargo-leptos
RUN wget --progress=dot:giga -O cargo-binstall.tgz \
  "https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-$(uname -m)-unknown-linux-musl.tgz" \
  && tar -xvf cargo-binstall.tgz \
  && cp cargo-binstall /usr/local/cargo/bin

# Install required tools
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends clang lld

# Install cargo-leptos
RUN cargo binstall cargo-leptos -y \
  # Add the WASM target
  && rustup target add wasm32-unknown-unknown


# Make an /app dir, which everything will eventually live in
RUN mkdir -p /app
WORKDIR /app
COPY . .

ENV RUSTFLAGS="--cfg erase_components"
# Build the app
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
