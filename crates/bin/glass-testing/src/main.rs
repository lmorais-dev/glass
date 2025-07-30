use glass_transport::message::Message;
use glass_transport::server;
use glass_transport::server::config::{ServerHttpConfig, ServerSecurityConfig};
use glass_transport::server::error::ServerError;
use glass_transport::server::handler::RouterFn;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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

    let route_fn: RouterFn = Arc::new(Box::new(|message| Box::pin(route_message(message))));

    server::Server::serve(&server_config, route_fn).await?;

    Ok(())
}

async fn route_message(message: Message) -> Result<Message, ServerError> {
    info!("reached the router");
    Ok(message)
}
