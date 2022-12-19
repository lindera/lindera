FROM rust:1.66.0-slim-bullseye AS builder

ARG LINDERA_VERSION

WORKDIR /repo

RUN set -ex \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
       build-essential \
       cmake \
       pkg-config \
       libssl-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN rustup component add rustfmt --toolchain 1.66.0-x86_64-unknown-linux-gnu

RUN cargo build --release --features="all-dictionaries"

FROM debian:bullseye-slim

COPY --from=builder /repo/target/release/lindera /usr/local/bin

ENTRYPOINT [ "lindera" ]
