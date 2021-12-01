# Standalone build
FROM rust:bullseye as build

ENV DEBIAN_FRONTEND=noninteractive

ARG PROFILE=release
ARG BINARY=interbtc-standalone

WORKDIR /src
COPY . /src

RUN apt-get update && \
    apt-get dist-upgrade -y -o Dpkg::Options::="--force-confold" && \
    apt-get install -y cmake pkg-config libssl-dev git llvm-dev libclang-dev clang gcc-multilib

RUN cargo build "--$PROFILE"

FROM bitnami/minideb:buster

ARG PROFILE=release
ARG BINARY=interbtc-standalone

COPY --from=build /src/target/$PROFILE/${BINARY} /usr/local/bin

# Checks
RUN chmod +x /usr/local/bin/${BINARY} && \
    ldd /usr/local/bin/${BINARY} && \
    /usr/local/bin/${BINARY} --version

RUN /usr/local/bin/${BINARY} export-genesis-state --chain staging --parachain-id 21 > /var/lib/genesis-state
RUN /usr/local/bin/${BINARY} export-genesis-wasm --chain staging > /var/lib/genesis-wasm

EXPOSE 30333 9933 9944
VOLUME ["/data"]

CMD ["/usr/local/bin/${BINARY}"]
