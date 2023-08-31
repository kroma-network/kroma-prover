use clap::Parser;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use utils::check_chain_id;
use utils::Measurer;
use zkevm::{
    circuit::{EvmCircuit, StateCircuit, AGG_DEGREE, DEGREE, MAX_TXS},
    io::write_file,
    prover::Prover,
    utils::{get_block_trace_from_file, load_kzg_params, load_or_create_seed},
};

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

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let chain_id = check_chain_id();
    log::info!("chain_id: {chain_id}");
    let args = Args::parse();

    // Prepare KZG params and rng for prover
    let mut timer = Measurer::new();
    let params = load_kzg_params(&args.params_path.clone().unwrap(), *DEGREE)
        .expect("failed to load kzg params");
    let agg_params = load_kzg_params(&args.params_path.unwrap(), *AGG_DEGREE)
        .expect("failed to load kzg params");
    let seed =
        load_or_create_seed(&args.seed_path.unwrap()).expect("failed to load or create seed");
    let rng = XorShiftRng::from_seed(seed);

    let mut prover = Prover::from_params_and_rng(params, agg_params, rng);
    timer.end("finish loading params");

    // Getting traces from specific directory
    let mut traces = HashMap::new();
    let trace_path = PathBuf::from(&args.trace_path.unwrap());
    if trace_path.is_dir() {
        for entry in fs::read_dir(trace_path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() && path.to_str().unwrap().ends_with(".json") {
                let block_trace = get_block_trace_from_file(path.to_str().unwrap());
                traces.insert(path.file_stem().unwrap().to_os_string(), block_trace);
            }
        }
    } else {
        let block_trace = get_block_trace_from_file(trace_path.to_str().unwrap());
        traces.insert(trace_path.file_stem().unwrap().to_os_string(), block_trace);
    }

    // Generating proofs for each trace
    let mut outer_timer = Measurer::new();
    for (trace_name, trace) in traces {
        let tx_count = trace.transactions.len();
        if tx_count > MAX_TXS {
            panic!(
                "{}",
                format!(
                    "too many transactions. MAX_TXS: {}, given transactions: {}",
                    MAX_TXS, tx_count
                )
            );
        }
        let mut out_dir = PathBuf::from(&trace_name);
        fs::create_dir_all(&out_dir).unwrap();

        timer.start();
        prover.debug_dir = String::from(out_dir.to_str().unwrap());
        if args.evm_proof.is_some() {
            let proof_path = PathBuf::from(&trace_name).join("evm.proof");

            let evm_proof = prover
                .create_target_circuit_proof::<EvmCircuit>(&trace)
                .expect("cannot generate evm_proof");

            if args.evm_proof.unwrap() {
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(evm_proof.proof.as_slice()).unwrap();
            }
        }

        if args.state_proof.is_some() {
            let proof_path = PathBuf::from(&trace_name).join("state.proof");

            let state_proof = prover
                .create_target_circuit_proof::<StateCircuit>(&trace)
                .expect("cannot generate state_proof");

            if args.state_proof.unwrap() {
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(state_proof.proof.as_slice()).unwrap();
            }
        }

        if args.agg_proof.is_some() {
            let mut proof_path = PathBuf::from(&trace_name).join("agg.proof");

            let agg_proof = prover
                .create_agg_circuit_proof(&trace)
                .expect("cannot generate agg_proof");

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
        timer.end("finish generating a proof");
    }
    outer_timer.end("finish generating all");
}
