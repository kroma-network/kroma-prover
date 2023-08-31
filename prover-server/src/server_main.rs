mod prove;
mod spec;
pub mod utils;

use crate::prove::{create_proof, ProofResult};
use crate::spec::ProofType;
use crate::utils::{kroma_err, kroma_info};
use ::utils::check_chain_id;
use clap::Parser;
use jsonrpc_derive::rpc;
use jsonrpc_http_server::jsonrpc_core::Result;
use jsonrpc_http_server::ServerBuilder;
use spec::ZkSpec;
use types::eth::BlockTrace;
use zkevm::circuit::{CHAIN_ID, MAX_TXS};
#[rpc]
pub trait Rpc {
    #[rpc(name = "spec")]
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
    fn spec(&self) -> Result<ZkSpec> {
        let spec = ZkSpec::new(*CHAIN_ID as u32);
        Ok(spec)
    }

    #[rpc(name = "prove")]
    /// return proof related to the trace.
    fn prove(&self, trace: String, proof_type: i32) -> Result<ProofResult>;
}

pub struct RpcImpl;

impl Rpc for RpcImpl {
    /// return zk-proof generated with the trace as an input.
    ///
    /// # Arguments
    /// * `trace` - A trace of the specific block as a JSON String.
    /// * `proof_type` - An identifier of proof type (1: Evm, 2: State, 3: Super, 4: Agg)
    ///
    /// # Returns
    /// ProofResult instance which includes proof and final pair.
    fn prove(&self, trace: String, proof_type_val: i32) -> Result<ProofResult> {
        // initiate ProofType
        let proof_type = ProofType::from_value(proof_type_val);
        if let ProofType::None = proof_type {
            let msg = format!(
                "invalid prove param: expected param from 1 to 4, but {:?}",
                proof_type_val
            );
            kroma_err(&msg);
            let err = jsonrpc_core::Error::invalid_params(msg);
            return Err(err);
        }

        // initiate BlockTrace
        let block_trace: BlockTrace = match serde_json::from_slice(trace.as_bytes()) {
            Ok(trace) => trace,
            Err(_) => {
                kroma_err("invalid block trace.");
                let err = jsonrpc_core::Error::invalid_params("invalid format trace");
                return Err(err);
            }
        };

        // check number of txs in the trace
        let tx_count = block_trace.transactions.len();
        if tx_count > MAX_TXS {
            let msg = format!(
                "too many transactions. MAX_TXS: {}, given transactions: {}",
                MAX_TXS, tx_count
            );
            kroma_err(&msg);
            let err = jsonrpc_core::Error::invalid_params(msg);
            return Err(err);
        }

        // check chain id
        let trace_chain_id = block_trace.chain_id;
        if *CHAIN_ID != trace_chain_id.as_u64() {
            let msg = format!(
                "not matched chain ids: expected({:?}), requested({:?})",
                *CHAIN_ID, trace_chain_id
            );
            kroma_err(&msg);
            let err = jsonrpc_core::Error::invalid_params(msg);
            return Err(err);
        }

        create_proof(block_trace, proof_type)
    }
}

pub struct MockRpcImpl;

impl Rpc for MockRpcImpl {
    /// Regardless of the received trace, it returns a zero proof.
    fn prove(&self, _trace: String, _proof_type: i32) -> Result<ProofResult> {
        kroma_info("return zero proof");
        Ok(ProofResult::new(vec![0; 4640], Some(vec![0; 128])))
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

    let chain_id = check_chain_id();
    let args = Args::parse();
    let endpoint = args.endpoint.unwrap_or("127.0.0.1:3030".to_string());

    let mut io = jsonrpc_core::IoHandler::new();
    #[cfg(not(feature = "mock-server"))]
    io.extend_with(RpcImpl.to_delegate());
    #[cfg(feature = "mock-server")]
    io.extend_with(MockRpcImpl.to_delegate());

    kroma_info(format!(
        "Prover server starting on {endpoint}. CHAIN_ID: {chain_id}"
    ));
    let server = ServerBuilder::new(io)
        .threads(3)
        .max_request_body_size(32_000_000)
        .start_http(&endpoint.parse().unwrap())
        .unwrap();

    server.wait();
}
