# Kroma Prover
[Kroma](https://github.com/kroma-network/kroma) is an Ethereum layer 2 network consisting of entities such as Proposer, Validators, Challengers, and more. The detailed explanations about our network and its entities can be found [here](https://github.com/kroma-network/kroma/blob/dev/specs/introduction.md).

This [Prover](https://github.com/kroma-network/kroma/blob/dev/specs/zkevm-prover.md) is a component of the Kroma challenger that generates zkevm-proof for the Kroma blockchain. And the zkevm-proof is used as a fault proof to rectify the validator's misbehavior. The detailed explanations abount Challenge process can be found [here](https://github.com/kroma-network/kroma/blob/dev/specs/challenge.md).

## Requirements

- Rust(rustc 1.68.0-nightly)
- Golang (go1.20.3)
- `axel` installed for downloading KZG params

## Download KZG Params
```shell
# Download Params for KZG
> sh download_params.sh
```

## Kroma Prover Binary

Prover server (entry: prover-server/src/server_main.rs)

```shell
# build
> cargo build --release --bin prover-server

# run
> ./target/release/prover-server --endpoint "127.0.0.1:3030"
```

Mock Prover server (which always return zero proof for test)

```shell
# build
> cargo build --release --bin prover-server --features mock-server

# run
> ./target/release/prover-server --endpoint "127.0.0.1:3030"
```

Mock client for test (entry: prover-grpc/src/mock_client.rs)

```shell
# build
> cargo build --release --bin mock-client

# run with proof type: EVM(1), STATE(2), SUPER(3), AGG(4)
> ./target/release/client-mock --prove <proof_type_int>
# or
> ./target/release/client-mock --spec true
```

## Legacy Binaries

Setup (entry: bin/src/setup.rs)

```shell
> cargo run --release --bin setup -- --help
```

If you run into linking issues during setup you may need to run

```shell
> cp `find ./target/release/ | grep libzktrie.so` /usr/local/lib/
```

to move the zktrielib into a path where your linker can locate it

Prove (entry: bin/src/prove.rs)

```shell
> cargo build --release --bin prove

> ./target/release/prove --help
```

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
