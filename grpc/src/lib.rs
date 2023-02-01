pub mod server;
pub mod service;
pub mod proof {
    tonic::include_proto!("proof");
}
pub mod server_config;
