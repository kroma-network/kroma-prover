use once_cell::sync::Lazy;
use tonic::transport::Server;
use zkevm::utils::read_env_var;

pub static GRPC_PORT: Lazy<u16> = Lazy::new(|| read_env_var("GRPC_PORT", 50051));
pub static GRPC_IP: Lazy<String> = Lazy::new(|| read_env_var("GRPC_IP", "0.0.0.0".to_string()));

#[derive(Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub ip: String,
    pub server: Server,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig::validate_ip(&GRPC_IP);
        Self {
            port: *GRPC_PORT,
            ip: GRPC_IP.to_string(),
            server: Server::builder(),
        }
    }
}

impl ServerConfig {
    pub fn new(port: u16, ip: String, server: Server) -> Self {
        ServerConfig::validate_ip(&ip);
        Self {
            port: port,
            ip: ip,
            server: server,
        }
    }

    fn validate_ip(ip: &str) {
        let allowed_ip = ["0.0.0.0", "127.0.0.1"];
        if !(allowed_ip.contains(&ip)) {
            panic!(
                "{} is not allowed IP address. Choose either 0.0.0.0 or 127.0.0.1",
                ip
            );
        }
    }
}
