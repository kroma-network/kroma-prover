use anyhow::Result;
use clap::Parser;
use futures::future::join_all;
use log::info;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use storage::s3::S3;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let args = Args::parse();
    let params = load_or_create_params(&args.params_path.clone().unwrap(), *DEGREE)
        .expect("failed to load or create params");
    let agg_params = load_or_create_params(&args.params_path.unwrap(), *AGG_DEGREE)
        .expect("failed to load or create params");
    let seed =
        load_or_create_seed(&args.seed_path.unwrap()).expect("failed to load or create seed");
    let rng = XorShiftRng::from_seed(seed);

    let mut prover = Prover::from_params_and_rng(params, agg_params, rng);

    let mut traces = HashMap::new();
    let trace_path = PathBuf::from(&args.trace_path.unwrap());
    if trace_path.is_dir() {
        for entry in fs::read_dir(trace_path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() && path.to_str().unwrap().ends_with(".json") {
                let block_result = get_block_result_from_file(path.to_str().unwrap());
                traces.insert(path.file_stem().unwrap().to_os_string(), block_result);
            }
        }
    } else {
        let block_result = get_block_result_from_file(trace_path.to_str().unwrap());
        traces.insert(trace_path.file_stem().unwrap().to_os_string(), block_result);
    }

    let outer_now = Instant::now();
    let mut dir_name = String::from("");

    let s3 = S3::new(
        ENV_CRED_KEY_ID.to_string(),
        ENV_CRED_KEY_SECRET.to_string(),
        REGION.to_string(),
    );
    for (_trace_name, trace) in traces {
        let block_number = trace.block_trace.number.to_string();
        let block_hash = format!("{:#x}", trace.block_trace.hash);
        dir_name = format!("{}_{}", block_number, block_hash);

        let mut out_dir = PathBuf::from(&dir_name);
        prover.debug_dir = String::from(out_dir.to_str().unwrap());

        fs::create_dir_all(&dir_name)?;

        if args.evm_proof.is_some() {
            let proof_path = PathBuf::from(&dir_name).join("evm.proof");

            let now = Instant::now();
            let evm_proof = prover
                .create_target_circuit_proof::<EvmCircuit>(&trace)
                .expect("cannot generate evm_proof");
            info!(
                "finish generating evm proof of {}, elapsed: {:?}",
                &trace.block_trace.hash,
                now.elapsed()
            );

            if args.evm_proof.unwrap() {
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(evm_proof.proof.as_slice()).unwrap();
            }
        }

        if args.state_proof.is_some() {
            let proof_path = PathBuf::from(&dir_name).join("state.proof");

            let now = Instant::now();
            let state_proof = prover
                .create_target_circuit_proof::<StateCircuit>(&trace)
                .expect("cannot generate state_proof");
            info!(
                "finish generating state proof of {}, elapsed: {:?}",
                &trace.block_trace.hash,
                now.elapsed()
            );

            if args.state_proof.unwrap() {
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(state_proof.proof.as_slice()).unwrap();
            }
        }

        if args.agg_proof.is_some() {
            let mut proof_path = PathBuf::from(&dir_name).join("agg.proof");

            let now = Instant::now();
            let agg_proof = prover
                .create_agg_circuit_proof(&trace)
                .expect("cannot generate agg_proof");
            info!(
                "finish generating agg proof of {}, elapsed: {:?}",
                &trace.block_trace.hash,
                now.elapsed()
            );

            if args.agg_proof.unwrap() {
                fs::create_dir_all(&proof_path).unwrap();
                agg_proof.write_to_dir(&mut proof_path);
            }

            let sol = prover.create_solidity_verifier(&agg_proof);
            write_file(
                &mut out_dir,
                "verifier.sol",
                &Vec::<u8>::from(sol.as_bytes()),
            );
            log::info!("output files to {}", out_dir.to_str().unwrap());
        }
    }
    info!("finish generating all, elapsed: {:?}", outer_now.elapsed());

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

    Ok(())
}
