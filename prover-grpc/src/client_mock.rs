pub mod l2_client;
pub mod prover_lib;
pub mod server;
pub mod utils;
pub mod proof {
    tonic::include_proto!("proof");
}

use crate::prover_lib::ProofType;
use crate::server::{DEFAULT_GRPC_IP, DEFAULT_GRPC_PORT};
use crate::utils::kroma_info;
use clap::Parser;
use proof::{proof_client::ProofClient, ProofRequest, ProverSpecRequest};
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "prove")]
    prove: Option<u32>,

    #[clap(short, long = "spec")]
    spec: Option<bool>,
}

async fn test_request_proof(addr_str: String, proof_type: ProofType) -> bool {
    let block_number_hex = "0x10".to_string();

    let request = tonic::Request::new(ProofRequest {
        block_number_hex: block_number_hex.clone(),
        proof_type: proof_type.to_value(),
    });

    kroma_info(format!(
        "Send 'prove' request: height({}), proof_type({})",
        block_number_hex, proof_type
    ));

    let mut client = ProofClient::connect(addr_str).await.unwrap();
    let response = client.prove(request).await.unwrap();

    kroma_info(format!(
        "Got:\n - final_pair: {:?}\n - proof: {:?}",
        response.get_ref().final_pair,
        response.get_ref().proof
    ));

    true
}

async fn test_request_spec(addr_str: String) -> bool {
    let request = tonic::Request::new(ProverSpecRequest {});

    kroma_info(format!("Send 'spec' request to prover-grpc"));

    let mut client = ProofClient::connect(addr_str).await.unwrap();
    let response = client.spec(request).await.unwrap();

    let proof_type_str: &String = &response.get_ref().proof_type_desc;
    let proof_type_map: HashMap<String, u32> = serde_json::from_str(&proof_type_str).unwrap();

    kroma_info(format!(
        "Got: \
        \n - proof_types: {:?}\
        \n - agg_degree: {}\
        \n - degree: {}\
        \n - chain_id: {}\
        \n - max_txs: {}\
        \n - max_call_data: {}",
        proof_type_map,
        response.get_ref().agg_degree,
        response.get_ref().degree,
        response.get_ref().chain_id,
        response.get_ref().max_txs,
        response.get_ref().max_call_data
    ));

    true
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::init();

    let addr_str = format!(
        "http://{}:{}",
        DEFAULT_GRPC_IP.to_string(),
        DEFAULT_GRPC_PORT
    );

    let args = Args::parse();
    if args.spec.is_some() {
        let _ = test_request_spec(addr_str.clone()).await;
    }
    if args.prove.is_some() {
        let proof_type = ProofType::from_value(args.prove.expect("no proof type"));
        let _ = test_request_proof(addr_str, proof_type).await;
    }

    Ok(())
}
