use super::proof::{proof_server::Proof, ProofRequest, ProofResponse};
use clap::Parser;
use prover_lib::args::Args;
use prover_lib::prover_lib::ProverLib;
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct ProofService {}

#[tonic::async_trait]
impl Proof for ProofService {
    async fn prove(
        &self,
        request: Request<ProofRequest>,
    ) -> Result<Response<ProofResponse>, Status> {
        let block_number_hex = request.into_inner().block_number_hex;

        let mut prover_lib = ProverLib::new();

        // get block trace
        let trace = prover_lib
            .make_trace_from_chain(block_number_hex)
            .await
            .unwrap();

        // load params and seed
        prover_lib.load_params_and_seed(Args::parse());

        // start creating proof
        let created_proof = prover_lib.create_proof(trace).await.unwrap();

        // create grpc response
        let message = ProofResponse {
            final_pair: created_proof.final_pair,
            proof: created_proof.proof,
        };

        Ok(Response::new(message))
    }
}
