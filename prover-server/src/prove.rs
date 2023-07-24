use crate::spec::ProofType;
use crate::utils::{kroma_info, kroma_msg};
use jsonrpc_core::Result;
use rand_core::SeedableRng;
use rand_xorshift::XorShiftRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::create_dir_all;
use std::io::Write;
use std::path::{Path, PathBuf};
use types::eth::BlockTrace;
use utils::Measurer;
use zkevm::circuit::{EvmCircuit, StateCircuit, SuperCircuit, AGG_DEGREE, DEGREE};
use zkevm::io::write_file;
use zkevm::prover::{AggCircuitProof, Prover, TargetCircuitProof};
use zkevm::utils::{load_or_create_params, load_or_create_seed};

const PARAMS_DIR: &str = "./test_params/";
const SEED_FILE: &str = "./test_seed";
const OUT_PROOF_DIR: &str = "./out_proof/";
const VERIFIER_NAME: &str = "zk-verifier.sol";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProofResult {
    pub final_pair: Option<Vec<u8>>,
    pub proof: Vec<u8>,
}

impl ProofResult {
    pub fn new(proof: Vec<u8>, final_pair: Option<Vec<u8>>) -> Self {
        Self { proof, final_pair }
    }
}

pub fn create_proof(trace: BlockTrace, proof_type: ProofType) -> Result<ProofResult> {
    // load or create material for prover
    let params = load_or_create_params(PARAMS_DIR, *DEGREE)
        .unwrap_or_else(|_| panic!("{}", kroma_msg("failed to load or create params")));
    let agg_params = load_or_create_params(PARAMS_DIR, *AGG_DEGREE)
        .unwrap_or_else(|_| panic!("{}", kroma_msg("failed to load or create agg params")));
    let seed = load_or_create_seed(SEED_FILE)
        .unwrap_or_else(|_| panic!("{}", kroma_msg("failed to load or create seed")));
    let rng = XorShiftRng::from_seed(seed);

    // prepare directory to store proof. (i.e., ./out_proof/<block_number>/)
    let height_hex = trace.header.number.unwrap().to_string();
    let out_dir = PathBuf::from(OUT_PROOF_DIR).join(height_hex);
    let _ = create_dir_all(&out_dir);

    // build prover
    let mut prover = Prover::from_params_and_rng(params, agg_params, rng);
    // specify the dir to store the vk and proof of the intermediate circuit.
    prover.debug_dir = out_dir.to_str().unwrap().to_string();

    match proof_type {
        ProofType::None => {
            panic!("invalid proof type");
        }
        ProofType::Agg => create_agg_proof(prover, trace),
        _ => create_target_proof(prover, trace, proof_type),
    }
}

pub fn create_target_proof(
    mut prover: Prover,
    trace: BlockTrace,
    proof_type: ProofType,
) -> Result<ProofResult> {
    kroma_info("start creating proof");

    // generate proof
    let mut timer = Measurer::new();
    let proof = match proof_type {
        ProofType::Evm => prover
            .create_target_circuit_proof::<EvmCircuit>(&trace)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("cannot generate evm_proof"))),
        ProofType::State => prover
            .create_target_circuit_proof::<StateCircuit>(&trace)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("cannot generate state_proof"))),
        ProofType::Super => prover
            .create_target_circuit_proof::<SuperCircuit>(&trace)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("cannot generate super_proof"))),
        _ => {
            panic!("invalid proof type");
        }
    };
    timer.end(&kroma_msg("finish generating a proof"));

    // store the proof as a file
    let proof_dir = PathBuf::from(&prover.debug_dir);
    write_target_proof(&proof_dir, proof.clone(), &proof_type.to_string());

    let proof_result = ProofResult::new(proof.proof, None);
    Ok(proof_result)
}

pub fn create_agg_proof(mut prover: Prover, trace: BlockTrace) -> Result<ProofResult> {
    kroma_info("start creating proof");

    // generate proof
    let mut timer = Measurer::new();
    let proof = prover
        .create_agg_circuit_proof(&trace)
        .unwrap_or_else(|_| panic!("{}", kroma_msg("cannot generate agg_proof")));
    timer.end(&kroma_msg("finish generating a proof"));

    // store proof and verifier contract as files
    let dir = PathBuf::from(prover.debug_dir.clone());
    write_agg_proof(&dir, &proof);
    write_solidity(&prover, &proof, &dir, VERIFIER_NAME);
    kroma_info(format!("output files to {}", dir.to_str().unwrap()));

    let proof_result = ProofResult::new(proof.proof.clone(), Some(proof.final_pair));
    Ok(proof_result)
}

pub fn write_target_proof(dir: &Path, proof: TargetCircuitProof, proof_type: &str) {
    let proof_path = dir.join(proof_type.to_string() + ".proof");
    let mut f = fs::File::create(proof_path).unwrap();
    f.write_all(proof.proof.as_slice()).unwrap();
}

pub fn write_agg_proof(dir: &Path, proof: &AggCircuitProof) {
    let mut proof_path = dir.join("agg.proof");
    let _ = fs::create_dir_all(&proof_path);

    proof.write_to_dir(&mut proof_path);
}

pub fn write_solidity(
    prover: &Prover,
    proof: &AggCircuitProof,
    dir: &PathBuf,
    verifier_name: &str,
) {
    let sol = prover.create_solidity_verifier(proof);
    let _ = fs::create_dir_all(dir);

    let mut dir = dir.clone();
    write_file(&mut dir, verifier_name, &Vec::<u8>::from(sol.as_bytes()));
}
