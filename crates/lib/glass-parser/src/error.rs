#[cfg(feature = "parsing")]
use pest::error::Error as PestError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[cfg(feature = "parsing")]
    #[error("Pest parsing error: {0}")]
    PestError(#[from] Box<PestError<crate::parser::Rule>>),

    #[error("Invalid primitive type: {0}")]
    InvalidPrimitiveType(String),

    #[error("Expected {expected} rule but found {found}")]
    UnexpectedRule { expected: String, found: String },

    #[error("Missing required element: {0}")]
    MissingElement(String),

    #[error("Invalid syntax: {0}")]
    InvalidSyntax(String),

    #[error("Schema reference error: {details}")]
    SchemaReferenceError { details: String, reference: Option<String> },
}
