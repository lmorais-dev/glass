use crate::message::status::Status;
use crate::security::error::SecurityError;
use h3::error::StreamError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),

    #[error("Failed to resolve an H3 request")]
    Resolver,

    #[error("Failed to decode a message: {0}")]
    Decoding(ciborium::de::Error<std::io::Error>),

    #[error("Failed to encode a message: {0}")]
    Encoding(ciborium::ser::Error<std::io::Error>),

    #[error("Failed to send a message")]
    Sender,

    #[error("Failed with status: {0:#?}")]
    Status(Status),

    #[error("H3 stream error: {0}")]
    Stream(#[from] StreamError),

    #[error("IO error: {0}")]
    StdIo(#[from] std::io::Error),
}
