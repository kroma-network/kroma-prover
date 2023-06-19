# Kroma Prover

Users can obtain zkevm proofs for a specific block height through the kroma-prover.  

## Setup

```shell
> git clone https://github.com/kroma-network/kroma-prover.git
> cd kroma-prover
> git submodule update --init
```

## Kroma Prover Binary

Prover server (entry: prover-grpc/src/prover_server.rs)

```shell
# build
> cargo build --release --bin prover-server

# run
> ./target/release/prover-sever --config <config-file-path>
```

Mock client for test (entry: prover-grpc/src/client_mock.rs)

```shell
# build
> cargo build --release --bin client-mock 

# run with proof type: EVM(1), STATE(2), SUPER(3), AGG(4)
> ./target/release/client-mock --prove <proof_type_int>
# or 
> ./target/release/client-mock --spec true
```

config file template for launching prover-server (config.json)

```json
{
  "grpc_port": "<prover's-port-as-integer>",
  "grpc_ip": "<prover's-ip-as-string>",
  "params_dir": "<directory-of-KZG-params>",
  "seed_path": "<seed-file-path-to-used-for-prover's-rng>",
  "proof_out_dir": "<directory-for-proof-as-output>",
  "l2_rpc_endpoint": "<rpc-endpoint-of-kroma-geth>",
  "verifier_name": "<verifier-sol-file-name>"
}
```

## Legacy Binaries

Setup (entry: bin/src/setup.rs)

```shell
> cargo build --release --bin setup   

> ./target/release/setup --params <params-file-path> --seed <seed-file-path>
```

If you run into linking issues during setup you may need to run

```shell
> cp `find ./target/release/ | grep libzktrie.so` /usr/local/lib/
```

to move the zktrielib into a path where your linker can locate it

Prove

```shell
> cargo build --release --bin prove

> ./target/release/prove --help
```

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
