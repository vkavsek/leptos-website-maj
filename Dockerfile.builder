FROM rust:1.73-alpine3.18

RUN apk update && \
    apk add --no-cache bash binaryen gcc git g++ libc-dev make npm openssl-dev protobuf-dev protoc zlib-dev musl-dev && \ 
    apk upgrade


RUN rustup target add wasm32-unknown-unknown
RUN rustup component add clippy

# Specify library paths explicitly
ENV OPENSSL_LIB_DIR=/usr/lib/
ENV OPENSSL_INCLUDE_DIR=/usr/include/
ENV PKG_CONFIG_ALLOW_CROSS=1

RUN cargo install cargo-leptos
RUN npm install -g sass

WORKDIR /work

CMD /bin/bash
