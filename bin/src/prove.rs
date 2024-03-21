use clap::Parser;
use halo2_proofs::{consts::SEED, halo2curves::bn256::Bn256, poly::kzg::commitment::ParamsKZG};
use std::{
    collections::HashMap,
    ffi::OsString,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};
use types::eth::BlockTrace;
use utils::{check_chain_id, Measurer};
use zkevm::{
    circuit::{EvmCircuit, StateCircuit, AGG_DEGREE, DEGREE, MAX_TXS},
    io::write_file,
    prover::Prover,
    utils::{get_block_trace_from_file, load_kzg_params},
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Specify directory which params have stored in. (default: ./kzg_params)
    #[clap(short, long = "params_dir")]
    params_dir: Option<String>,
    /// Specify path to block trace (json file)
    #[clap(short, long = "trace")]
    trace_path: String,
    /// Specify circuit type in [evm, state, agg]
    #[clap(short, long = "circuit")]
    circuit: String,
}

enum CircuitType {
    EVM,
    STATE,
    AGG,
}

impl Args {
    fn is_tachyon() -> bool {
        #[cfg(not(feature = "tachyon"))]
        return false;
        #[cfg(feature = "tachyon")]
        return true;
    }

    fn get_params_dir(&self) -> String {
        match &self.params_dir {
            Some(dir) => dir.clone(),
            None => {
                let dir = String::from("kzg_params");
                fs::create_dir_all(&dir).unwrap();
                dir
            }
        }
    }

    fn get_seed() -> [u8; 16] {
        #[cfg(not(feature = "tachyon"))]
        return [
            0x59, 0x62, 0xbe, 0x5d, 0x76, 0x3d, 0x31, 0x8d, 0x17, 0xdb, 0x37, 0x32, 0x54, 0x06,
            0xbc, 0xe5,
        ];
        #[cfg(feature = "tachyon")]
        return SEED;
    }

    fn get_proof_type(&self) -> CircuitType {
        if self.circuit.to_lowercase() == "evm" {
            CircuitType::EVM
        } else if self.circuit.to_lowercase() == "state" {
            CircuitType::STATE
        } else if self.circuit.to_lowercase() == "agg" {
            CircuitType::AGG
        } else {
            panic!("you should specify proof type in [evm, state, agg]");
        }
    }

    fn load_params(&self) -> (ParamsKZG<Bn256>, ParamsKZG<Bn256>) {
        let params =
            load_kzg_params(&self.get_params_dir(), *DEGREE).expect("failed to load kzg params");
        let agg_params = load_kzg_params(&self.get_params_dir(), *AGG_DEGREE)
            .expect("failed to load kzg params");
        (params, agg_params)
    }

    fn panic_if_tx_too_many(trace: &BlockTrace) {
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
    }

    fn load_traces(&self) -> HashMap<OsString, BlockTrace> {
        let mut traces = HashMap::new();
        let trace_path = PathBuf::from(&self.trace_path);
        if trace_path.is_dir() {
            for entry in fs::read_dir(trace_path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_file() && path.to_str().unwrap().ends_with(".json") {
                    let block_trace = get_block_trace_from_file(path.to_str().unwrap());
                    Args::panic_if_tx_too_many(&block_trace);
                    traces.insert(path.file_stem().unwrap().to_os_string(), block_trace);
                }
            }
        } else {
            let block_trace = get_block_trace_from_file(trace_path.to_str().unwrap());
            Args::panic_if_tx_too_many(&block_trace);
            traces.insert(trace_path.file_stem().unwrap().to_os_string(), block_trace);
        }
        traces
    }
}

fn main() {
    dotenv::dotenv().ok();
    let args = Args::parse();
    let chain_id = check_chain_id();

    env_logger::init();
    let is_tachyon = Args::is_tachyon();
    log::info!("chain_id: {chain_id}, tachyon: {is_tachyon}");

    // Prepare KZG params and rng for prover
    let mut timer = Measurer::new();
    let (params, agg_params) = args.load_params();
    let mut prover = Prover::from_params_and_seed(params, agg_params, Args::get_seed());
    timer.end("finish loading params");

    // Getting traces from specific directory
    let traces = args.load_traces();

    // Generating proofs for each trace
    let mut outer_timer = Measurer::new();
    for (trace_name, trace) in traces {
        let mut out_dir = PathBuf::from(&trace_name);
        fs::create_dir_all(&out_dir).unwrap();
        prover.debug_dir = String::from(out_dir.to_str().unwrap());

        timer.start();
        match args.get_proof_type() {
            CircuitType::EVM => {
                let proof_path = PathBuf::from(&trace_name).join("evm.proof");
                let evm_proof = prover
                    .create_target_circuit_proof::<EvmCircuit>(&trace)
                    .expect("cannot generate evm_proof");
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(evm_proof.proof.as_slice()).unwrap();
            }
            CircuitType::STATE => {
                let proof_path = PathBuf::from(&trace_name).join("state.proof");
                let state_proof = prover
                    .create_target_circuit_proof::<StateCircuit>(&trace)
                    .expect("cannot generate state_proof");
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(state_proof.proof.as_slice()).unwrap();
            }
            CircuitType::AGG => {
                let mut proof_path = PathBuf::from(&trace_name).join("agg.proof");
                let agg_proof = prover
                    .create_agg_circuit_proof(&trace)
                    .expect("cannot generate agg_proof");
                fs::create_dir_all(&proof_path).unwrap();
                agg_proof.write_to_dir(&mut proof_path);

                let sol = prover.create_solidity_verifier(&agg_proof);
                write_file(
                    &mut out_dir,
                    "verifier.sol",
                    &Vec::<u8>::from(sol.as_bytes()),
                );
                log::info!("output files to {}", out_dir.to_str().unwrap());
            }
        }
        timer.end("finish generating a proof");
    }
    outer_timer.end("finish generating all");
}
