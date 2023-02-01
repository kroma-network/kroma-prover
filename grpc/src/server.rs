use super::proof::proof_server::ProofServer;
use super::server_config::ServerConfig;
use super::service::ProofService;
use anyhow::Result;
use log::info;
use signal_hook::{consts::SIGHUP, consts::SIGINT, consts::SIGTERM, iterator::Signals};

pub struct GrpcServer {}

impl GrpcServer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn start(&self, server_config: ServerConfig) -> Result<()> {
        let mut server = server_config.server;
        let addr_str = format!("{}:{}", server_config.ip, server_config.port);

        let addr = addr_str.parse().unwrap();
        let proof_service = ProofService::default();

        info!("Grpc server running on {}", addr_str);
        server
            .add_service(ProofServer::new(proof_service))
            .serve_with_shutdown(addr, self.watch_for_shutdown())
            .await?;

        Ok(())
    }

    async fn watch_for_shutdown(&self) -> () {
        let mut signals = Signals::new(&[SIGINT, SIGTERM, SIGHUP]).unwrap();
        let join_handle = tokio::spawn(async move {
            for sig in signals.forever() {
                info!("Received signal {:?}. Shutting down grpc server", sig);
                return ();
            }
        });
        join_handle.await.unwrap();
    }
}
