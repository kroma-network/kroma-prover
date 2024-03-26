FROM ubuntu:jammy AS kroma-prover-builder
LABEL maintainer="TA <ta@lightscale.io>"

RUN apt-get update && apt-get install curl build-essential axel -y && \
    rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly-2022-12-10 -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN curl -LO "https://golang.org/dl/go1.20.3.linux-amd64.tar.gz" && \
    rm -rf /usr/local/go && tar -C /usr/local -xzf go1.20.3.linux-amd64.tar.gz
ENV PATH="${PATH}:/usr/local/go/bin"

COPY . /usr/src/kroma-prover
WORKDIR /usr/src/kroma-prover

RUN cargo build --release --bin prover-server
RUN mkdir -p /temp-lib \
    && cp $(find ./target/release/ -name libzktrie.so) /temp-lib/

FROM tachyon:v3 AS tachyon

FROM ubuntu:jammy AS kroma-prover
LABEL maintainer="TA <ta@lightscale.io>"

COPY --from=kroma-prover-builder /usr/src/kroma-prover/target/release/prover-server .
COPY --from=kroma-prover-builder /temp-lib/libzktrie.so /usr/local/lib/

COPY --from=tachyon /usr/src/tachyon/bazel-bin/scripts/packages/debian/runtime/libtachyon_0.0.1_amd64.deb /usr/src/tachyon/bazel-bin/scripts/packages/debian/runtime/libtachyon_0.0.1_amd64.deb
COPY --from=tachyon /usr/src/tachyon/bazel-bin/scripts/packages/debian/dev/libtachyon-dev_0.0.1_amd64.deb /usr/src/tachyon/bazel-bin/scripts/packages/debian/dev/libtachyon-dev_0.0.1_amd64.deb

RUN apt update && \
    apt install -y --no-install-recommends \
    libgmp-dev \
    libomp-dev && \
    rm -rf /var/lib/apt/lists/*

RUN dpkg -i /usr/src/tachyon/bazel-bin/scripts/packages/debian/runtime/libtachyon_0.0.1_amd64.deb && \
    dpkg -i /usr/src/tachyon/bazel-bin/scripts/packages/debian/dev/libtachyon-dev_0.0.1_amd64.deb

RUN apt update && \
    apt install -y --no-install-recommends \
    axel && \
    rm -rf /var/lib/apt/lists/*
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
