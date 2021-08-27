FROM rust:1.54.0-slim-buster AS builder

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

RUN rustup component add rustfmt --toolchain 1.54.0-x86_64-unknown-linux-gnu

RUN cargo build --release

FROM debian:buster-slim

COPY --from=builder /repo/target/release/lindera /usr/local/bin
COPY --from=builder /repo/target/release/lindera-ipadic-builder /usr/local/bin

ENTRYPOINT [ "lindera" ]
