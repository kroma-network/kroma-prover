pub mod prover_lib;
pub mod server;
pub mod utils;
pub mod prover {
    tonic::include_proto!("prover");
}

use anyhow::Result;
use clap::Parser;
use server::GrpcServer;
use std::path::Path;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "config")]
    config_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

    let args = Args::parse();
    let config_path = args.config_path.unwrap();

    let config_path = Path::new(&config_path);
    let mut server = GrpcServer::from_config_file(config_path);
    let _ = server.start().await;
    Ok(())
}
