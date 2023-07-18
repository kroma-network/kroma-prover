mod prove;
mod spec;
pub mod utils;

use crate::prove::{create_proof, ProofResult};
use crate::spec::ProofType;
use crate::utils::{kroma_err, kroma_msg};
use clap::Parser;
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::{ErrorCode, Result};
use jsonrpc_http_server::ServerBuilder;
use spec::ZkSpec;
use types::eth::BlockTrace;

const KROMA_CHAIN_ID: u32 = 901;

#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
    /// return the prover's specification as JSON String.
    fn spec(&self) -> Result<String>;

    #[rpc(name = "prove")]
    /// return proof related to the trace.
    fn prove(&self, trace: String, proof_type: i32) -> Result<ProofResult>;
}

pub struct RpcImpl;

impl Rpc for RpcImpl {
    /// return the prover's specification as JSON String.
    ///
    /// # Returns
    ///
    /// String of ZkSpec instance which includes below
    /// 1. proof_type_desc: String,
    /// 2. pub agg_degree: u32,
    /// 3. pub chain_id: u32,
    /// 4. pub max_txs: u32,
    /// 5. pub max_call_data: u32,
    fn spec(&self) -> Result<String> {
        let spec = ZkSpec::new(KROMA_CHAIN_ID);
        Ok(serde_json::to_string(&spec).unwrap())
    }

    /// return zk-proof generated with the trace as an input.
    ///
    /// # Arguments
    /// * `trace` - A trace of the specific block as a JSON String.
    /// * `proof_type` - An identifier of proof type (1: Evm, 2: State, 3: Super, 4: Agg)
    ///
    /// # Returns
    /// ProofResult instance which includes proof and final pair.
    fn prove(&self, trace: String, proof_type: i32) -> Result<ProofResult> {
        // initiate BlockTrace
        let block_trace: BlockTrace = match serde_json::from_slice(trace.as_bytes()) {
            Ok(trace) => trace,
            Err(_) => {
                kroma_err("invalid block trace.");
                let err = jsonrpc_core::Error::new(ErrorCode::InvalidParams);
                return Err(err);
            }
        };

        // initiate ProofType
        let proof_type = ProofType::from_value(proof_type);
        if let ProofType::None = proof_type {
            let err = jsonrpc_core::Error::new(ErrorCode::InvalidParams);
            return Err(err);
        }

        create_proof(block_trace, proof_type)
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "endpoint")]
    endpoint: Option<String>,
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let args = Args::parse();
    let endpoint = args.endpoint.unwrap_or("127.0.0.1:3030".to_string());

    let mut io = jsonrpc_core::IoHandler::new();
    io.extend_with(RpcImpl.to_delegate());

    kroma_msg(format!("Prover server running on {endpoint}"));
    let server = ServerBuilder::new(io)
        .threads(3)
        .start_http(&endpoint.parse().unwrap())
        .unwrap();

    server.wait();
}
