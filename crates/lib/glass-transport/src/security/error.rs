use quinn::crypto::rustls::NoInitialCipherSuite;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("Certificate isn't found at path: {0}")]
    CertificateNotFound(String),

    #[error("Key isn't found at path: {0}")]
    KeyNotFound(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("IO error: {0}")]
    StdIo(#[from] std::io::Error),

    #[error("TLS error: {0}")]
    Rustls(#[from] rustls::Error),

    #[error("TLS error: {0}")]
    CipherSuite(#[from] NoInitialCipherSuite),
}
