use crate::l2_client::{L2Client, DEFAULT_RPC_URL};
use crate::proof::proof_server::Proof;
use crate::proof::{ProofRequest, ProofResponse, ProverSpecRequest, ProverSpecResponse};
use crate::utils::{kroma_info, kroma_msg, write_agg_proof, write_solidity, write_target_proof};
use anyhow::Result;
use core::panic;
use enum_iterator::{all, Sequence};
use halo2_proofs::halo2curves::bn256::Bn256;
use halo2_proofs::poly::kzg::commitment::ParamsKZG;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use once_cell::sync::Lazy;
use rand_core::SeedableRng;
use rand_xorshift::XorShiftRng;
use serde_json;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
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
    pub fn from_value(val: u32) -> Self {
        match val {
            1 => ProofType::EVM,
            2 => ProofType::STATE,
            3 => ProofType::SUPER,
            4 => ProofType::AGG,
            _ => ProofType::NONE,
        }
    }

    pub fn to_value(&self) -> u32 {
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

pub struct ProverLib {
    params: ParamsKZG<Bn256>,
    agg_params: ParamsKZG<Bn256>,
    seed: [u8; 16],
    l2_client: L2Client,
    out_proof_dir: PathBuf,
    verifier_name: String,
}

impl Default for ProverLib {
    fn default() -> Self {
        Self::new(
            &DEFAULT_PARAMS_DIR,
            &DEFAULT_SEED_PATH,
            HttpClientBuilder::default().build(DEFAULT_RPC_URL).unwrap(),
            &DEFAULT_OUT_DIR,
            "verifier.sol".to_string(),
        )
    }
}

impl ProverLib {
    pub fn new(
        params_dir: &Path,
        seed_path: &Path,
        l2_rpc_endpoint: HttpClient,
        proof_out_dir: &Path,
        verifier_name: String,
    ) -> Self {
        // ensure params_dir is a directory
        if !params_dir.is_dir() {
            panic!("params_dir must be a directory");
        }

        // ensure seed_path is a file
        if !seed_path.is_file() {
            panic!("seed_dir must be a file")
        }

        // create dir and check whether proof_out_dir is a directory.
        let _ = create_dir_all(proof_out_dir);
        if !proof_out_dir.is_dir() {
            panic!("out_dir must be a directory");
        }

        // load and create material for prover
        let params = load_or_create_params(params_dir.to_str().unwrap(), *DEGREE)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("fail to load or create params")));
        let agg_params = load_or_create_params(params_dir.to_str().unwrap(), *AGG_DEGREE)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("fail to load or create params")));
        let seed = load_or_create_seed(seed_path.to_str().unwrap())
            .unwrap_or_else(|_| panic!("{}", kroma_msg("fail to load or create seed")));

        Self {
            params,
            agg_params,
            seed,
            l2_client: L2Client::new(l2_rpc_endpoint),
            out_proof_dir: proof_out_dir.to_path_buf(),
            verifier_name,
        }
    }

    pub async fn get_block_trace_from_l2(&self, block_number_hex: String) -> Result<BlockTrace> {
        let mut timer = Measurer::new();

        let block_trace = self
            .l2_client
            .get_trace_by_block_number_hex(block_number_hex.clone())
            .await?;

        timer.end(&kroma_msg("finish getting block_trace"));
        Ok(block_trace)
    }

    pub fn file_out_flag(&self) -> bool {
        let msg = self.out_proof_dir.to_str().unwrap();
        kroma_info(format!("check exporting flag: {msg}"));
        self.out_proof_dir.is_dir()
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
        if self.file_out_flag() {
            let dir = PathBuf::from(prover.debug_dir.clone());
            write_agg_proof(&dir, &proof);
            // always export verifier.sol when proof_type is AggProof
            write_solidity(&prover, &proof, &dir, &self.verifier_name);
            kroma_info(format!("output files to {}", dir.to_str().unwrap()));
        }

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
        if self.file_out_flag() {
            let proof_dir = PathBuf::from(&prover.debug_dir);
            write_target_proof(&proof_dir, proof, proof_type);
        }

        Ok(proof_result)
    }

    /// Create proof and return it, optionally exporting it to file.
    pub async fn create_proof(
        &self,
        trace: BlockTrace,
        proof_type: ProofType,
    ) -> Result<ProofResult> {
        // build prover and set dir to export output
        let rng = XorShiftRng::from_seed(self.seed);
        let mut prover =
            Prover::from_params_and_rng(self.params.clone(), self.agg_params.clone(), rng);

        if self.file_out_flag() {
            // prepare dir to store proof. (i.e., self.OUT_DIR/<block_number>/)
            let block_num_str = trace.header.number.unwrap().to_string();
            let proof_dir = self.out_proof_dir.join(block_num_str);
            let _ = create_dir_all(&proof_dir);

            // specify the dir to store the vk and proof of the intermediate circuit.
            prover.debug_dir = proof_dir.to_str().unwrap().to_string();
        }

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
impl Proof for ProverLib {
    async fn prove(
        &self,
        request: Request<ProofRequest>,
    ) -> Result<Response<ProofResponse>, Status> {
        let block_number_hex = &request.get_ref().block_number_hex;
        let proof_type = ProofType::from_value(request.get_ref().proof_type);

        // get block trace from l2 geth
        let trace = self
            .get_block_trace_from_l2(block_number_hex.clone())
            .await
            .unwrap();

        // generate proof
        // note that, in case of non-agg-proof, proof_result.final_pair MUST be `None`
        let proof_result = self.create_proof(trace, proof_type).await.unwrap();

        // build grpc response
        let message = ProofResponse {
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
