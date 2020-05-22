FROM rust:1.43.0-slim-stretch AS builder

ARG LINDERA_VERSION

WORKDIR /repo

RUN set -ex \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
       build-essential \
       cmake \
       jq \
       pkg-config \
       libssl-dev \
       curl \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install lindera --root=./ --vers=${LINDERA_VERSION}


FROM debian:stretch-slim

WORKDIR /

RUN set -ex \
    && apt-get update \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /repo/bin /usr/local/bin

ENTRYPOINT [ "lindera" ]
