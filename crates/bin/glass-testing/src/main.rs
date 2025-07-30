use async_trait::async_trait;
use glass_transport::message::Message;
use glass_transport::server;
use glass_transport::server::config::{ServerHttpConfig, ServerSecurityConfig};
use glass_transport::server::error::ServerError;
use glass_transport::server::handler::{Handler, TypedHandler};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_filter(env_filter);
    tracing_subscriber::registry().with(fmt_layer).init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Unable to set crypto provider");

    let server_config = server::config::ServerConfig {
        http: ServerHttpConfig {
            bind_address: "127.0.0.1:7612".parse()?,
        },
        security: ServerSecurityConfig {
            tls_certificate: PathBuf::from("tls/certificate.der"),
            tls_private_key: PathBuf::from("tls/key.der"),
        },
    };

    let router: TypedHandler = Arc::new(Box::new(Router));

    server::Server::serve(&server_config, router).await?;

    Ok(())
}

#[derive(Clone)]
pub struct Router;

#[async_trait]
impl Handler for Router {
    async fn handle(&self, message: Message) -> Result<Message, ServerError> {
        Ok(message)
    }
}
