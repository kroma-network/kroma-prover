use super::proof::proof_server::ProofServer;
use crate::prover_lib::ProverLib;
use crate::utils::{kroma_info, kroma_msg};
use anyhow::Result;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use serde_derive::{Deserialize, Serialize};
use signal_hook::{consts::SIGHUP, consts::SIGINT, consts::SIGTERM, iterator::Signals};
use std::fs::{create_dir_all, File};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tonic::transport::Server;

pub static DEFAULT_GRPC_IP: &str = "0.0.0.0";
pub static DEFAULT_GRPC_PORT: u16 = 50051;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// For config.json
pub struct RawConfig {
    pub grpc_port: u16,
    pub grpc_ip: String,
    pub params_dir: PathBuf,
    pub seed_path: PathBuf,
    pub proof_out_dir: Option<PathBuf>,
    pub l2_rpc_endpoint: String,
    pub verifier_name: Option<String>,
}

pub struct GrpcServer {
    pub grpc_addr: SocketAddr,
    pub l2_rpc_endpoint: HttpClient,
    pub params_dir: PathBuf,
    pub seed_path: PathBuf,
    pub proof_out_dir: PathBuf,
    pub verifier_name: String,
}

impl GrpcServer {
    pub fn new(
        grpc_addr: SocketAddr,
        l2_rpc_endpoint: HttpClient,
        params_dir: &Path,
        seed_path: &Path,
        proof_out_dir: &Path,
        verifier_name: &String,
    ) -> Self {
        Self {
            grpc_addr,
            l2_rpc_endpoint,
            params_dir: params_dir.to_path_buf(),
            seed_path: seed_path.to_path_buf(),
            proof_out_dir: proof_out_dir.to_path_buf(),
            verifier_name: verifier_name.to_string(),
        }
    }

    pub fn from_config_file(config_path: &PathBuf) -> Self {
        let file = File::open(config_path)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("fail to open config file.")));
        let deserialized: RawConfig = serde_json::from_reader(file).unwrap_or_else(|_| {
            panic!("{}", kroma_msg("config file was not well-formatted json."))
        });

        // params dir is specified, but not exists.
        if !deserialized.params_dir.is_dir() & !deserialized.params_dir.is_file() {
            let _ = create_dir_all(deserialized.params_dir.clone());
        }

        // init socket to be used this server.
        let grpc_socket: SocketAddr =
            format!("{}:{}", deserialized.grpc_ip, deserialized.grpc_port)
                .parse()
                .unwrap();

        // init HttpClient connecting to l2 geth
        let l2_endpoint = HttpClientBuilder::default()
            .build(deserialized.l2_rpc_endpoint)
            .unwrap();

        // the proof-result will be stored in `proof_out_dir` (verifier contract too.)
        // if config has no `proof_out_dir`, prover does not store proof.
        let out_dir = if let Some(dir) = deserialized.proof_out_dir {
            dir
        } else {
            PathBuf::default()
        };

        // set verifier contract's name (use default name if config has no `verifier_name`
        let verifier_name = if let Some(dir) = deserialized.verifier_name {
            dir
        } else {
            "verifier.sol".to_string()
        };

        Self::new(
            grpc_socket,
            l2_endpoint,
            &deserialized.params_dir,
            &deserialized.seed_path,
            &out_dir,
            &verifier_name,
        )
    }

    pub async fn start(&mut self) -> Result<()> {
        let service = ProverLib::new(
            &self.params_dir,
            &self.seed_path,
            self.l2_rpc_endpoint.clone(),
            &self.proof_out_dir,
            self.verifier_name.clone(),
        );

        kroma_info(format!("Grpc server running on {}", self.grpc_addr));
        let mut server = Server::builder();
        server
            .add_service(ProofServer::new(service))
            .serve_with_shutdown(self.grpc_addr, self.watch_for_shutdown())
            .await?;

        Ok(())
    }

    async fn watch_for_shutdown(&self) {
        let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP]).unwrap();
        let join_handle = tokio::spawn(async move {
            if let Some(sig) = signals.forever().next() {
                kroma_info(format!(
                    "Received signal {sig:?}. Shutting down grpc server"
                ));
            }
        });
        join_handle.await.unwrap();
    }
}
