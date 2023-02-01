use anyhow::Result;
use grpc::server::GrpcServer;
use grpc::server_config::ServerConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

    let server = GrpcServer::new();
    server.start(ServerConfig::default()).await?;

    Ok(())
}
