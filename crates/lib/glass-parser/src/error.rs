use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("The specified file was not found: {0}")]
    FileNotFound(String),

    #[error("The specified file was not a valid glass file: {0:?}")]
    UnexpectedRule(crate::parser::Rule),

    #[error("The next token was not found")]
    NoNextToken,

    #[error("An IO operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("A pest parsing error occurred: {0}")]
    Pest(#[from] Box<pest::error::Error<crate::parser::Rule>>),
}
