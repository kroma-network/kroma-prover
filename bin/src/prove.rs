use anyhow::Result;
use clap::Parser;
use futures::future::join_all;
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
};
use jsonrpsee_core::Error;
use log::{debug, info};
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::{
    collections::BTreeMap,
    env,
    fs::{create_dir_all, read_dir, File},
    io::Write,
    path::PathBuf,
};
use storage::s3::S3;
use types::eth::BlockResult;
use utils::Measurer;
use zkevm::{
    circuit::{EvmCircuit, StateCircuit, AGG_DEGREE, DEGREE},
    io::write_file,
    prover::Prover,
    utils::{get_block_result_from_file, load_or_create_params, load_or_create_seed},
};

const ENV_CRED_KEY_ID: &str = "";
const ENV_CRED_KEY_SECRET: &str = "";
const BUCKET_NAME: &str = "voost-proof";
const REGION: &str = "ap-northeast-2";
const RPC_URL: &str = "http://localhost:8545";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Get params and write into file.
    #[clap(short, long = "params")]
    params_path: Option<String>,
    /// Get seed and write into file.
    #[clap(long = "seed")]
    seed_path: Option<String>,
    /// Get BlockTrace from file or dir.
    #[clap(short, long = "trace")]
    trace_path: Option<String>,
    /// Option means if generates evm proof.
    /// Boolean means if output evm proof.
    #[clap(long = "evm")]
    evm_proof: Option<bool>,
    /// Option means if generates state proof.
    /// Boolean means if output state proof.
    #[clap(long = "state")]
    state_proof: Option<bool>,
    /// Option means if generates agg proof.
    /// Boolean means if output agg proof.
    #[clap(long = "agg")]
    agg_proof: Option<bool>,

    /// Indicate how to get block trace result (if false, read from chain)
    #[clap(long = "trace_from_file")]
    trace_from_file: Option<bool>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let mut timer = Measurer::new();

    timer.start();
    let args = Args::parse();
    let params = load_or_create_params(&args.params_path.clone().unwrap(), *DEGREE)
        .expect("failed to load or create params");
    let agg_params = load_or_create_params(&args.params_path.unwrap(), *AGG_DEGREE)
        .expect("failed to load or create params");
    let seed =
        load_or_create_seed(&args.seed_path.unwrap()).expect("failed to load or create seed");
    let rng = XorShiftRng::from_seed(seed);

    let mut prover = Prover::from_params_and_rng(params, agg_params, rng);
    timer.end("finish loading params");

    let s3 = if ENV_CRED_KEY_ID.is_empty() {
        None
    } else {
        Some(S3::new(
            ENV_CRED_KEY_ID.to_string(),
            ENV_CRED_KEY_SECRET.to_string(),
            REGION.to_string(),
        ))
    };

    let traces = make_traces(
        args.trace_from_file.unwrap(),
        &args.trace_path.unwrap(),
        &s3,
    )
    .await?;

    for trace in traces.iter() {
        timer.start();

        let block_number = trace.block_trace.number.to_string();
        let block_hash = format!("{:#x}", trace.block_trace.hash);
        let dir_name = format!("{}_{}", block_number, block_hash);

        let mut out_dir = PathBuf::from(&dir_name);
        prover.debug_dir = String::from(out_dir.to_str().unwrap());

        create_dir_all(&dir_name)?;

        if args.evm_proof.is_some() {
            let proof_path = PathBuf::from(&dir_name).join("evm.proof");

            let evm_proof = prover
                .create_target_circuit_proof::<EvmCircuit>(&trace)
                .expect("cannot generate evm_proof");

            if args.evm_proof.unwrap() {
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(evm_proof.proof.as_slice()).unwrap();
            }
        }

        if args.state_proof.is_some() {
            let proof_path = PathBuf::from(&dir_name).join("state.proof");

            let state_proof = prover
                .create_target_circuit_proof::<StateCircuit>(&trace)
                .expect("cannot generate state_proof");

            if args.state_proof.unwrap() {
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(state_proof.proof.as_slice()).unwrap();
            }
        }

        if args.agg_proof.is_some() {
            let mut proof_path = PathBuf::from(&dir_name).join("agg.proof");

            let agg_proof = prover
                .create_agg_circuit_proof(&trace)
                .expect("cannot generate agg_proof");

            if args.agg_proof.unwrap() {
                create_dir_all(&proof_path).unwrap();
                agg_proof.write_to_dir(&mut proof_path);
            }

            let sol = prover.create_solidity_verifier(&agg_proof);
            write_file(
                &mut out_dir,
                "verifier.sol",
                &Vec::<u8>::from(sol.as_bytes()),
            );
            log::info!("output files to {}", out_dir.to_str().unwrap());

            if let Some(s3) = s3.as_ref() {
                let current_dir = env::current_dir().unwrap();
                let futures = [
                    "verify_circuit_proof.data",
                    "verify_circuit_final_pair.data",
                ]
                .iter()
                .map(|&path| {
                    let local_path = current_dir.join(format!("{}/agg.proof/{}", &dir_name, path));
                    let remote_path = format!("{}/{}", &dir_name, path);
                    return s3.upload(local_path, BUCKET_NAME.to_string(), remote_path);
                })
                .collect::<Vec<_>>();
                let results = join_all(futures).await;
                results.iter().for_each(|result| {
                    result.as_ref().unwrap();
                });

                info!("finish uploading all");
            } else {
                info!("not uploading to s3");
            }
        }

        timer.end("finish generating a proof");
    }

    Ok(())
}

async fn find_proven_blocks(s3: &S3) -> Result<BTreeMap<u32, String>> {
    let keys = s3.list_keys(BUCKET_NAME.to_string()).await?;

    // BTreeMap is sorted based on the key.
    let mut proven_blocks: BTreeMap<u32, String> = BTreeMap::new(); // K : blockNumber, V: blockHash

    for val in keys.iter() {
        let (dir_key, _file_name) = val.rsplit_once('/').unwrap();
        let (block_number_str, block_hash) = dir_key.rsplit_once('_').unwrap();
        let block_number_u32 = block_number_str.parse::<u32>().unwrap();

        // insert a key only if it doesn't already exist
        proven_blocks
            .entry(block_number_u32)
            .or_insert(block_hash.to_string());
    }

    for (block_number, block_hash) in &proven_blocks {
        debug!("{block_number}: {block_hash}");
    }

    Ok(proven_blocks)
}

async fn target_proving_block_num(proven_blocks: &BTreeMap<u32, String>) -> u32 {
    if proven_blocks.is_empty() {
        return 1;
    }
    let (last_proven_block_num, _block_hash) = proven_blocks.iter().next_back().unwrap();
    return last_proven_block_num + 1;
}

async fn get_block_trace(rpc_client: &HttpClient, block_number: &u32) -> Result<BlockResult> {
    let block_number_hex_str = format!("0x{:x}", block_number);
    let params = rpc_params![block_number_hex_str];
    let trace_result: Result<BlockResult, Error> = rpc_client
        .request("voost_getBlockResultByNumberOrHash", params)
        .await;
    Ok(trace_result.unwrap())
}

async fn make_trace_from_chain(s3: &S3) -> Result<Vec<BlockResult>> {
    let mut trace_vec = Vec::new();

    // get number of the block whose proof has to be created. (assume we are creating proof for a SINGLE block only)
    let proven_blocks = find_proven_blocks(&s3).await?;

    for val in proven_blocks.iter() {
        debug!("{}  {}", val.0, val.1)
    }

    // get the block trace of the target block
    let target_block_number = target_proving_block_num(&proven_blocks).await;
    let rpc_client = HttpClientBuilder::default().build(RPC_URL)?;
    let block_trace_result = get_block_trace(&rpc_client, &target_block_number).await?;

    trace_vec.push(block_trace_result);
    Ok(trace_vec)
}

async fn make_trace_from_file(trace_path: &str) -> Result<Vec<BlockResult>> {
    let mut trace_vec = Vec::new();
    let trace_path = PathBuf::from(trace_path);

    if trace_path.is_dir() {
        for entry in read_dir(trace_path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() && path.to_str().unwrap().ends_with(".json") {
                let block_result = get_block_result_from_file(path);
                trace_vec.push(block_result);
            }
        }
    } else {
        let block_result = get_block_result_from_file(trace_path);
        trace_vec.push(block_result);
    }
    Ok(trace_vec)
}

async fn make_traces(
    trace_from_file: bool,
    params_path: &str,
    s3: &Option<S3>,
) -> Result<Vec<BlockResult>> {
    if trace_from_file {
        info!("generating trace from file");
        Ok(make_trace_from_file(params_path).await?)
    } else {
        info!("generating trace from chain");
        Ok(make_trace_from_chain(s3.as_ref().unwrap()).await?)
    }
}
