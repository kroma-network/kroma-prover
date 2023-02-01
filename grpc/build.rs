fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../kanvas-grpc-proto/proof.proto")?;
    Ok(())
}
