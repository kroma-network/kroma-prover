use anyhow::Result;
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
};
use once_cell::sync::Lazy;
use types::eth::BlockTrace;
use zkevm::utils::read_env_var;

pub static RPC_URL: Lazy<String> =
    Lazy::new(|| read_env_var("RPC_URL", "http://localhost:8545".to_string()));

pub struct L2Client {
    http_client: HttpClient,
}

impl Default for L2Client {
    fn default() -> Self {
        Self {
            http_client: HttpClientBuilder::default().build(RPC_URL.clone()).unwrap(),
        }
    }
}

impl L2Client {
    pub fn new(url: String) -> Self {
        Self {
            http_client: HttpClientBuilder::default().build(url.clone()).unwrap(),
        }
    }

    pub async fn get_trace_by_block_number_hex(
        &self,
        block_number_hex: String,
    ) -> Result<BlockTrace> {
        let params = rpc_params![block_number_hex.clone()];
        let trace_result = self
            .http_client
            .request("kanvas_getBlockResultByNumberOrHash", params)
            .await;
        Ok(trace_result.unwrap())
    }
}
