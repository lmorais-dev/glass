use std::net::SocketAddr;
use std::path::PathBuf;

pub struct ServerConfig {
    pub http: ServerHttpConfig,
    pub security: ServerSecurityConfig,
}

pub struct ServerHttpConfig {
    pub bind_address: SocketAddr,
}

pub struct ServerSecurityConfig {
    pub tls_certificate: PathBuf,
    pub tls_private_key: PathBuf,
}
