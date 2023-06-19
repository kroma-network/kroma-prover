use anyhow::{Ok, Result};
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
};
use thiserror::Error;
use types::eth::BlockTrace;

/// It will be available after typing `make devnet-up` at root of kroma-network/kroma@dev)
pub static DEFAULT_RPC_URL: &str = "http://localhost:9545";

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("l2 client was not initiated")]
    Init,

    #[error("failed to get block trace")]
    BlockTrace,
}

/// An Ethereum rpc client
pub struct L2Client {
    http_client: HttpClient,
}

impl Default for L2Client {
    fn default() -> Self {
        let http_client = HttpClientBuilder::default()
            .build(DEFAULT_RPC_URL.clone())
            .unwrap();
        Self { http_client }
    }
}

impl L2Client {
    pub fn new(http_client: HttpClient) -> Self {
        Self { http_client }
    }

    pub async fn get_trace_by_block_number_hex(
        &self,
        block_number_hex: String,
    ) -> Result<BlockTrace> {
        // build rpc params
        let params = rpc_params![block_number_hex.clone()];

        // TODO: Add specific error handling (e.g., for timeout errors)
        let trace_result = self
            .http_client
            .request("kroma_getBlockTraceByNumberOrHash", params)
            .await;
        Ok(trace_result.unwrap())
    }
}

#[cfg(test)]
mod l2_client_test {
    use crate::l2_client::L2Client;

    #[tokio::test]
    async fn get_trace_test() {
        dotenv::dotenv().ok();
        env_logger::init();
        // the client will try to send request to dev-net
        let client = L2Client::default();

        // try to send request for getting block trace
        let result = client
            .get_trace_by_block_number_hex("0x10".to_string())
            .await
            .unwrap();

        assert_eq!(result.chain_id.as_u32(), 901);
        assert_eq!(result.transactions[0].type_, 126);
        assert_eq!(result.transactions[0].mint.unwrap().as_u32(), 0);
    }
}
