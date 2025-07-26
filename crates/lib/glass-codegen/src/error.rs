use glass_parser::type_tree::TypeTreeError;
use thiserror::Error;

/// Error type for code generators
#[derive(Error, Debug)]
pub enum CodeGeneratorError {
    #[error("Type tree error: {0}")]
    TypeTree(#[from] TypeTreeError),

    #[error("Type not found: {name}")]
    TypeNotFound { name: String },

    #[error("Invalid type reference: {reference}")]
    InvalidTypeReference { reference: String },

    #[error("Formatting error: {0}")]
    Formatting(String),

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Syntax error: {0}")]
    SyntaxError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
