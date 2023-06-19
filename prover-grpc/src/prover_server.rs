pub mod l2_client;
pub mod prover_lib;
pub mod utils;
pub mod proof {
    tonic::include_proto!("proof");
}
use anyhow::Result;
use clap::Parser;

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

    Ok(())
}
