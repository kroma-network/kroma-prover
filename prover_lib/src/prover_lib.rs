use super::args::Args;
use super::params_seed::ParamsSeed;
use anyhow::Result;
use l2client::L2Client;
use log::info;
use once_cell::sync::Lazy;
use rand_core::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};
use types::eth::BlockTrace;
use utils::Measurer;
use zkevm::{
    circuit::{EvmCircuit, StateCircuit, AGG_DEGREE, DEGREE},
    io::write_file,
    prover::Prover,
    utils::read_env_var,
    utils::{load_or_create_params, load_or_create_seed},
};

pub static OUT_TO_FILES: Lazy<bool> = Lazy::new(|| read_env_var("OUT_TO_FILES", false));

#[derive(Debug, Default)]
pub struct ProofResult {
    pub block_trace_found: bool,
    pub final_pair: Vec<u8>,
    pub proof: Vec<u8>,
}

pub struct ProverLib {
    timer: Measurer,
    args: Option<Args>,
    params_seed: Option<ParamsSeed>,
    l2_client: L2Client,
    out_to_files: bool,
}

impl ProverLib {
    pub fn new() -> Self {
        let timer = Measurer::new();
        Self {
            args: None,
            timer: timer,
            params_seed: None,
            l2_client: L2Client::default(),
            out_to_files: *OUT_TO_FILES,
        }
    }

    pub async fn make_trace_from_chain(&mut self, block_number_hex: String) -> Result<BlockTrace> {
        self.timer.start();
        let block_trace_result = self
            .l2_client
            .get_trace_by_block_number_hex(block_number_hex.clone())
            .await?;
        self.timer.end("finish getting block_trace");
        Ok(block_trace_result)
    }

    fn maybe_create_dir_all(&self, path: &str) -> Result<()> {
        if self.out_to_files {
            create_dir_all(&path).unwrap();
        }
        Ok(())
    }

    pub async fn create_proof(&mut self, trace: BlockTrace) -> Result<ProofResult> {
        let mut proof_result = ProofResult::default();
        let args = self.args.take().unwrap();

        let param_seed = self.params_seed.take().unwrap();
        let params = param_seed.params;
        let agg_params = param_seed.agg_params;
        let rng = param_seed.rng;

        let mut prover = Prover::from_params_and_rng(params, agg_params, rng);

        // start creating proof
        self.timer.start();
        info!("start creating proof");
        let block_number = trace.header.number.unwrap().to_string();

        let mut out_dir = PathBuf::from(&block_number);
        prover.debug_dir = String::from(out_dir.to_str().unwrap());
        create_dir_all(&block_number)?;

        if args.evm_proof.is_some() {
            let proof_path = PathBuf::from(&block_number).join("evm.proof");

            let evm_proof = prover
                .create_target_circuit_proof::<EvmCircuit>(&trace)
                .expect("cannot generate evm_proof");

            if args.evm_proof.unwrap() {
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(evm_proof.proof.as_slice()).unwrap();
            }
        }

        if args.state_proof.is_some() {
            let proof_path = PathBuf::from(&block_number).join("state.proof");

            let state_proof = prover
                .create_target_circuit_proof::<StateCircuit>(&trace)
                .expect("cannot generate state_proof");

            if args.state_proof.unwrap() {
                let mut f = File::create(&proof_path).unwrap();
                f.write_all(state_proof.proof.as_slice()).unwrap();
            }
        }

        if args.agg_proof.is_some() {
            let mut proof_path = PathBuf::from(&block_number).join("agg.proof");

            let agg_proof = prover
                .create_agg_circuit_proof(&trace)
                .expect("cannot generate agg_proof");

            if args.agg_proof.unwrap() {
                self.maybe_create_dir_all(proof_path.to_str().unwrap())
                    .unwrap();

                if *OUT_TO_FILES {
                    agg_proof.write_to_dir(&mut proof_path);

                    let sol = prover.create_solidity_verifier(&agg_proof);
                    write_file(
                        &mut out_dir,
                        "verifier.sol",
                        &Vec::<u8>::from(sol.as_bytes()),
                    );
                    log::info!("output files to {}", out_dir.to_str().unwrap());
                }

                proof_result.final_pair = agg_proof.final_pair;
                proof_result.proof = agg_proof.proof;
            }
        }

        self.timer.end("finish generating a proof");

        Ok(proof_result)
    }

    pub fn load_params_and_seed(&mut self, args: Args) {
        self.timer.start();
        info!("start loading params and seed");

        let params = load_or_create_params(&args.params_path.clone().unwrap(), *DEGREE)
            .expect("failed to load or create params");

        let agg_params = load_or_create_params(&args.params_path.unwrap(), *AGG_DEGREE)
            .expect("failed to load or create params");

        let seed =
            load_or_create_seed(&args.seed_path.unwrap()).expect("failed to load or create seed");

        let rng = XorShiftRng::from_seed(seed);

        self.params_seed = Some(ParamsSeed {
            params: params,
            agg_params: agg_params,
            seed: seed,
            rng: rng,
        });

        self.timer.end("finish loading params");
    }
}
