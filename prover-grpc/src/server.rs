use crate::prover::prover_server::ProverServer;
use crate::prover_lib::ProverLib;
use crate::utils::{kroma_info, kroma_msg};
use anyhow::{Ok, Result};
use serde_derive::{Deserialize, Serialize};
use signal_hook::{consts::SIGHUP, consts::SIGINT, consts::SIGTERM, iterator::Signals};
use std::fs::File;
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
    pub verifier_name: Option<String>,
}

pub struct GrpcServer {
    pub grpc_socket_addr: SocketAddr,
    pub prover_service: ProverLib,
}

impl GrpcServer {
    pub fn new(grpc_ip: String, grpc_port: u16, prover_service: ProverLib) -> Self {
        let grpc_socket_addr: SocketAddr = format!("{grpc_ip}:{grpc_port}").parse().unwrap();
        Self {
            grpc_socket_addr,
            prover_service,
        }
    }

    pub fn from_config_file(config_path: &Path) -> Self {
        let file = File::open(config_path)
            .unwrap_or_else(|_| panic!("{}", kroma_msg("fail to open config file.")));
        let config: RawConfig = serde_json::from_reader(file).unwrap_or_else(|_| {
            panic!("{}", kroma_msg("config file was not well-formatted json."))
        });

        let prover_service = ProverLib::from_config_path(config_path);

        Self::new(config.grpc_ip, config.grpc_port, prover_service)
    }

    pub async fn start(&mut self) -> Result<()> {
        kroma_info(format!("Grpc server running on {}", self.grpc_socket_addr));
        let mut server = Server::builder();
        server
            .add_service(ProverServer::new(self.prover_service.clone()))
            .serve_with_shutdown(self.grpc_socket_addr, self.watch_for_shutdown())
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
