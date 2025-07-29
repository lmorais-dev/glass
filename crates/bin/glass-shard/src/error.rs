use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShardError {
    #[error("The path provided does not exist: {0}")]
    InexistentPath(String),

    #[error("The path provided is not a directory: {0}")]
    NotDirectory(String),

    #[error("The path provided was invalid: {0}")]
    InvalidPath(String),

    #[error("An IO error occurred: {0}")]
    GeneralIo(#[from] std::io::Error),

    #[error("A parser error occurred: {0}")]
    Parser(#[from] glass_codegen::prelude::ParserError),
}
