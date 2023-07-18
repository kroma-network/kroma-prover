pub mod prove;
pub mod spec;
pub mod utils;

use crate::prove::ProofResult;
use crate::spec::{ProofType, ZkSpec};
use crate::utils::kroma_info;
use clap::Parser;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee_core::client::ClientT;
use jsonrpsee_core::rpc_params;
use std::fs;
use std::time::Duration;
use types::eth::BlockTrace;

const CLIENT_TIMEOUT_SEC: u64 = 7200;
const DEFAULT_RPC_SERVER_ENDPOINT: &str = "http://127.0.0.1:3030";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "prove")]
    prove: Option<i32>,

    #[clap(short, long = "spec")]
    spec: Option<bool>,
}

async fn test_request_proof(cli: HttpClient, proof_type: ProofType) -> bool {
    let trace_str =
        fs::read_to_string("zkevm/tests/traces/kroma/multiple_transfers_0.json").unwrap();
    let trace: BlockTrace = serde_json::from_str(&trace_str).unwrap();

    kroma_info(format!(
        "Send 'prove' request: height({}), proof_type({proof_type})",
        trace.header.number.unwrap()
    ));

    let params = rpc_params![trace_str, proof_type.to_value()];
    let proof_result: ProofResult = cli.request("prove", params).await.unwrap();

    kroma_info(format!(
        "Got:\n - final_pair: {:?}\n - proof: {:?}",
        proof_result.final_pair, proof_result.proof
    ));

    true
}

async fn test_request_spec(cli: HttpClient) -> bool {
    kroma_info("Send 'spec' request to prover-server");
    let params = rpc_params![];
    let response: String = cli.request("spec", params).await.unwrap();
    let zk_spec: ZkSpec = serde_json::from_str(&response).unwrap();

    kroma_info(format!(
        "Got: \
        \n - proof_types: {:?}\
        \n - agg_degree: {}\
        \n - degree: {}\
        \n - chain_id: {}\
        \n - max_txs: {}\
        \n - max_call_data: {}",
        zk_spec.proof_type_desc,
        zk_spec.agg_degree,
        zk_spec.degree,
        zk_spec.chain_id,
        zk_spec.max_txs,
        zk_spec.max_call_data
    ));

    true
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let args = Args::parse();

    let http_client = HttpClientBuilder::default()
        .request_timeout(Duration::from_secs(CLIENT_TIMEOUT_SEC))
        .build(DEFAULT_RPC_SERVER_ENDPOINT)
        .unwrap();

    if args.spec.is_some() {
        let _ = test_request_spec(http_client.clone()).await;
    }
    if args.prove.is_some() {
        let proof_type = ProofType::from_value(args.prove.expect("The proof type is not allowed."));
        let _ = test_request_proof(http_client, proof_type).await;
    }
}
