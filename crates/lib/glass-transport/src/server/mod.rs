use crate::security::error::SecurityError;
use crate::security::tls::TlsStore;
use crate::server::error::ServerError;
use crate::server::handler::RouterFn;
use quinn::VarInt;
use quinn::crypto::rustls::QuicServerConfig;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

pub mod config;
pub mod error;
pub mod handler;

pub struct Server;

impl Server {
    pub async fn serve(
        server_config: &config::ServerConfig,
        router: RouterFn,
    ) -> Result<(), ServerError> {
        let (certificate, key) = TlsStore::try_load(
            &server_config.security.tls_certificate,
            &server_config.security.tls_private_key,
        )
        .await?;

        let mut tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certificate], key)
            .map_err(SecurityError::Rustls)?;

        tls_config.max_early_data_size = u32::MAX;
        tls_config.alpn_protocols = vec![
            b"h3".to_vec(),
            b"h3-32".to_vec(),
            b"h3-31".to_vec(),
            b"h3-30".to_vec(),
            b"h3-29".to_vec(),
        ];

        let quic_server_config =
            QuicServerConfig::try_from(tls_config).map_err(SecurityError::CipherSuite)?;

        let mut quinn_server_config =
            quinn::ServerConfig::with_crypto(Arc::new(quic_server_config));

        let mut quinn_transport_config = quinn::TransportConfig::default();
        quinn_transport_config.keep_alive_interval(Some(Duration::from_secs(2)));
        quinn_transport_config.stream_receive_window(VarInt::from(128_u8));
        quinn_transport_config.receive_window(VarInt::from(4096_u16));
        quinn_transport_config.send_window(4096);
        quinn_transport_config.send_fairness(true);
        quinn_server_config.transport = Arc::new(quinn_transport_config);

        let quinn_endpoint =
            quinn::Endpoint::server(quinn_server_config, server_config.http.bind_address)?;

        let handler = handler::SessionHandler::new(router);

        while let Some(incoming_connection) = quinn_endpoint.accept().await {
            // We move the QUIC connection to its own task so to not block when waiting
            // for the handshake to finish and actually return the connection object
            let handler_clone = handler.clone();
            tokio::spawn(async move {
                match incoming_connection.await {
                    Ok(connection) => {
                        // We upgrade a raw QUIC connection to an H3 connection.
                        //
                        // Although the name of the module is a bit deceiving, we aren't starting
                        // another server, just upgrading the connection.
                        let h3_connection = h3::server::builder()
                            .enable_webtransport(true)
                            .enable_extended_connect(true)
                            .enable_datagram(true)
                            .send_grease(true)
                            .max_webtransport_sessions(u32::MAX as u64)
                            .build(h3_quinn::Connection::new(connection))
                            .await;

                        let h3_connection = match h3_connection {
                            Ok(h3_connection) => h3_connection,
                            Err(error) => {
                                debug!(?error, "Failed to upgrade the connection to h3");
                                return;
                            }
                        };

                        if let Err(error) = handler_clone.handle_h3(h3_connection).await {
                            debug!(?error, "Failed to handle a connection");
                        }
                    }
                    Err(error) => {
                        debug!(?error, "Failed to accept a connection");
                    }
                }
            });
        }

        quinn_endpoint.wait_idle().await;

        Ok(())
    }
}
