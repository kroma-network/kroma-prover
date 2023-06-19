fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("./kroma-prover-grpc-proto/proof.proto")?;
    Ok(())
}
