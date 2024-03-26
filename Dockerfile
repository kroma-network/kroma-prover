FROM ubuntu:jammy AS builder
LABEL maintainer="Kroma <kroma@kroma.network>"

ARG RUST_TOOLCHAIN_VERSION=nightly-2022-12-10

RUN apt update && \
    apt install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl && \
    rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain ${RUST_TOOLCHAIN_VERSION} -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN curl -LO "https://golang.org/dl/go1.20.3.linux-amd64.tar.gz" && \
    rm -rf /usr/local/go && tar -C /usr/local -xzf go1.20.3.linux-amd64.tar.gz
ENV PATH="${PATH}:/usr/local/go/bin"

COPY . /usr/src/kroma-prover
WORKDIR /usr/src/kroma-prover

RUN cargo build --release --bin prover-server
RUN mkdir -p /temp-lib && \
    cp $(find ./target/release/ -name libzktrie.so) /temp-lib/

FROM halo2:jammy AS kroma-prover
LABEL maintainer="Kroma <kroma@kroma.network>"

COPY --from=builder /usr/src/kroma-prover/target/release/prover-server .
COPY --from=builder /temp-lib/libzktrie.so /usr/local/lib/

COPY . /usr/src/kroma-prover
WORKDIR /usr/src/kroma-prover

ENV LD_LIBRARY_PATH=/usr/local/lib
ENV CHAIN_ID=2358
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV PROVE_TYPE=block
ENV RUST_MIN_STACK=100000000

EXPOSE 3030
CMD ["/bin/sh","-c","./prover-server"]
