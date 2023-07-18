use crate::prover::prover_server::Prover as GrpcProver;
use crate::prover::{ProveRequest, ProveResponse, ProverSpecRequest, ProverSpecResponse};
use crate::server::RawConfig;
use crate::utils::{kroma_info, kroma_msg, write_agg_proof, write_solidity, write_target_proof};
use anyhow::Result;
use core::panic;
use enum_iterator::{all, Sequence};
use once_cell::sync::Lazy;
use rand_core::SeedableRng;
use rand_xorshift::XorShiftRng;
use serde_json;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::path::Path;
use std::{fmt, fs::create_dir_all, path::PathBuf};
use tonic::{Request, Response, Status};
use types::eth::BlockTrace;
use utils::Measurer;
use zkevm::circuit::CHAIN_ID;
use zkevm::{
    circuit::{EvmCircuit, StateCircuit, SuperCircuit, AGG_DEGREE, DEGREE, MAX_CALLDATA, MAX_TXS},
    prover::Prover,
    utils::{load_or_create_params, load_or_create_seed},
};

pub static DEFAULT_PARAMS_DIR: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./test_params"));
pub static DEFAULT_SEED_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./test_seed"));
pub static DEFAULT_OUT_DIR: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("./out_proof"));

#[derive(Debug, Sequence)]
pub enum ProofType {
    NONE,
    EVM,
    STATE,
    SUPER,
    AGG,
}

impl Display for ProofType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProofType::EVM => write!(f, "evm"),
            ProofType::STATE => write!(f, "state"),
            ProofType::SUPER => write!(f, "super"),
            ProofType::AGG => write!(f, "agg"),
            ProofType::NONE => write!(f, "none"),
        }
    }
}

impl ProofType {
    /// enum selector by u32-value
    pub fn from_value(val: i32) -> Self {
        match val {
            1 => ProofType::EVM,
            2 => ProofType::STATE,
            3 => ProofType::SUPER,
            4 => ProofType::AGG,
            _ => ProofType::NONE,
        }
    }

    pub fn to_value(&self) -> i32 {
        match self {
            ProofType::EVM => 1,
            ProofType::STATE => 2,
            ProofType::SUPER => 3,
            ProofType::AGG => 4,
            ProofType::NONE => 0,
        }
    }

    /// it returns enum-value mapping as a String
    pub fn desc() -> String {
        let mut mapping = HashMap::new();
        let proof_type_vec = all::<ProofType>().collect::<Vec<_>>();
        for i in proof_type_vec {
            let key = i.to_string();
            let value = i.to_value();
            mapping.insert(key, value);
        }
        serde_json::to_string(&mapping).unwrap()
    }
}

#[derive(Debug, Default)]
pub struct ProofResult {
    pub final_pair: Option<Vec<u8>>,
    pub proof: Vec<u8>,
}

#[derive(Clone)]
pub struct ProverLib {
    params_dir: PathBuf,
    seed_path: PathBuf,
    out_proof_dir: PathBuf,
    verifier_name: String,
}

impl Default for ProverLib {
    fn default() -> Self {
        Self {
            params_dir: DEFAULT_PARAMS_DIR.clone(),
            seed_path: DEFAULT_SEED_PATH.clone(),
            out_proof_dir: DEFAULT_OUT_DIR.clone(),
            verifier_name: "verifier.sol".to_string(),
        }
    }
}

impl ProverLib {
    pub fn from_config_path(config_path: &Path) -> Self {
        let file = File::open(config_path)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("fail to open config file.")));
        let config: RawConfig = serde_json::from_reader(file).unwrap_or_else(|_| {
            panic!("{}", kroma_msg("config file was not well-formatted json."))
        });

        Self::new(
            config.params_dir,
            config.seed_path,
            config.proof_out_dir,
            config.verifier_name,
        )
    }

    pub fn new(
        params_dir: PathBuf,
        seed_path: PathBuf,
        proof_out_dir: Option<PathBuf>,
        verifier_name: Option<String>,
    ) -> Self {
        // ensure `params_dir` is directory
        if params_dir.is_file() {
            panic!("params_dir is not allowed a file.")
        }
        if !params_dir.is_dir() {
            let _ = create_dir_all(params_dir.clone());
        }

        // ensure seed_path is a file
        if !seed_path.is_file() {
            panic!("seed_dir must be a file")
        }

        // the proof and verifier contract will be stored in `proof_out_dir`
        let ensured_out_dir = proof_out_dir.unwrap_or(PathBuf::from("out_proof"));
        let _ = create_dir_all(&ensured_out_dir);

        // set verifier contract's name.
        let ensured_verifier_name = verifier_name.unwrap_or("verifier.sol".to_string());

        Self {
            params_dir,
            seed_path,
            out_proof_dir: ensured_out_dir,
            verifier_name: ensured_verifier_name,
        }
    }

    pub async fn create_agg_proof(
        &self,
        mut prover: Prover,
        trace: BlockTrace,
    ) -> Result<ProofResult> {
        // (common) proof result to be returned.
        let mut proof_result = ProofResult::default();

        kroma_info("start creating proof");
        let mut timer = Measurer::new();

        let proof = prover
            .create_agg_circuit_proof(&trace)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("cannot generate agg_proof")));

        timer.end(&kroma_msg("finish generating a proof"));

        proof_result.proof = proof.proof.clone();
        proof_result.final_pair = Some(proof.final_pair.clone());

        // write proof to file (opt)
        let dir = PathBuf::from(prover.debug_dir.clone());
        write_agg_proof(&dir, &proof);
        // always export verifier.sol when proof_type is AggProof
        write_solidity(&prover, &proof, &dir, &self.verifier_name);
        kroma_info(format!("output files to {}", dir.to_str().unwrap()));

        Ok(proof_result)
    }

    pub async fn create_target_proof(
        &self,
        mut prover: Prover,
        trace: BlockTrace,
        proof_type: ProofType,
    ) -> Result<ProofResult> {
        // (common) proof result to be returned.
        let mut proof_result = ProofResult::default();

        kroma_info("start creating proof");
        let mut timer = Measurer::new();

        let proof = match proof_type {
            ProofType::EVM => prover
                .create_target_circuit_proof::<EvmCircuit>(&trace)
                .unwrap_or_else(|_| panic!("{}", kroma_msg("cannot generate evm_proof"))),
            ProofType::STATE => prover
                .create_target_circuit_proof::<StateCircuit>(&trace)
                .unwrap_or_else(|_| panic!("{}", kroma_msg("cannot generate state_proof"))),
            ProofType::SUPER => prover
                .create_target_circuit_proof::<SuperCircuit>(&trace)
                .unwrap_or_else(|_| panic!("{}", kroma_msg("cannot generate super_proof"))),
            _ => {
                panic!("Invalid proof type");
            }
        };
        timer.end(&kroma_msg("finish generating a proof"));

        proof_result.proof = proof.proof.clone();

        // write proof to file (opt)
        let proof_dir = PathBuf::from(&prover.debug_dir);
        write_target_proof(&proof_dir, proof, proof_type);

        Ok(proof_result)
    }

    /// Create proof and return it, optionally exporting it to file.
    pub async fn create_proof(
        &self,
        trace: BlockTrace,
        proof_type: ProofType,
    ) -> Result<ProofResult> {
        // build prover
        // load and create material for prover
        let params = load_or_create_params(self.params_dir.to_str().unwrap(), *DEGREE)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("fail to load or create params")));
        let agg_params = load_or_create_params(self.params_dir.to_str().unwrap(), *AGG_DEGREE)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("fail to load or create params")));
        let seed = load_or_create_seed(self.seed_path.to_str().unwrap())
            .unwrap_or_else(|_| panic!("{}", kroma_msg("fail to load or create seed")));
        let rng = XorShiftRng::from_seed(seed);

        let mut prover = Prover::from_params_and_rng(params, agg_params, rng);

        // prepare dir to store proof. (i.e., self.OUT_DIR/<block_number>/)
        let block_num_str = trace.header.number.unwrap().to_string();
        let proof_dir = self.out_proof_dir.join(block_num_str);
        let _ = create_dir_all(&proof_dir);
        prover.debug_dir = self.out_proof_dir.to_str().unwrap().to_string();

        match proof_type {
            ProofType::NONE => {
                panic!("Invalid proof type");
            }
            ProofType::AGG => self.create_agg_proof(prover, trace).await,

            // For target circuit cases
            _ => self.create_target_proof(prover, trace, proof_type).await,
        }
    }
}

#[tonic::async_trait]
impl GrpcProver for ProverLib {
    async fn prove(
        &self,
        request: Request<ProveRequest>,
    ) -> Result<Response<ProveResponse>, Status> {
        let trace_str: &String = &request.get_ref().trace_string;
        let trace: BlockTrace = match serde_json::from_slice(trace_str.as_bytes()) {
            Ok(trace) => trace,
            Err(e) => {
                kroma_info("Trace parsing failed");
                return Err(Status::from_error(e.into()));
            }
        };

        let proof_type = ProofType::from_value(request.get_ref().proof_type);

        // generate proof
        // NOTE(dongchangYoo): in case of non-agg-proof, proof_result.final_pair MUST be `None`
        let proof_result = match self.create_proof(trace, proof_type).await {
            Ok(result) => result,
            Err(e) => {
                kroma_info("Proof creation failed");
                return Err(Status::from_error(e.into()));
            }
        };

        // build grpc response
        let message = ProveResponse {
            final_pair: match proof_result.final_pair {
                Some(final_pair) => final_pair,
                None => vec![],
            },
            proof: proof_result.proof,
        };

        Ok(Response::new(message))
    }

    /// returns prover's spec.
    async fn spec(
        &self,
        _request: Request<ProverSpecRequest>,
    ) -> Result<Response<ProverSpecResponse>, Status> {
        let message = ProverSpecResponse {
            proof_type_desc: ProofType::desc(),
            degree: *DEGREE as u32,
            agg_degree: *AGG_DEGREE as u32,
            chain_id: *CHAIN_ID as u32,
            max_txs: MAX_TXS as u32,
            max_call_data: MAX_CALLDATA as u32,
        };

        Ok(Response::new(message))
    }
}

#[cfg(test)]
mod prover_lib_tests {
    use crate::prover_lib::{ProofType, ProverLib};
    use zkevm::utils::get_block_trace_from_file;

    pub static DEFAULT_TRACE_PATH: &str = "../zkevm/tests/traces/impure/simple/impure.json";

    async fn target_proof_test(proof_type: ProofType) {
        dotenv::dotenv().ok();
        env_logger::init();

        let prover = ProverLib::default();
        let block_trace = get_block_trace_from_file(DEFAULT_TRACE_PATH);

        let proof_result = prover.create_proof(block_trace, proof_type).await.unwrap();
        if let Some(_) = proof_result.final_pair {
            panic!("target proof result has Some value as final_pair")
        }
        assert!(proof_result.proof.len() > 0)
    }

    #[tokio::test]
    #[ignore]
    // it takes about 671.16s on AWS r6i.32xlarge instance
    async fn evm_proof_test() {
        target_proof_test(ProofType::EVM).await;
    }

    #[tokio::test]
    #[ignore]
    // TODO(dongchangYoo) this test fails: assign something with negative offset where init_pk() -> StateCircuit::synthesize()
    async fn state_proof_test() {
        target_proof_test(ProofType::STATE).await;
    }

    #[tokio::test]
    #[ignore]
    // it takes about 1999.64s on AWS r6i.32xlarge instance
    async fn super_proof_test() {
        target_proof_test(ProofType::SUPER).await;
    }
}
