use crate::ast::schema::SchemaRef;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidatorError {
    #[error("A duplicate schema was found: `{0}`")]
    DuplicateSchema(String),

    #[error("A duplicate interface was found: `{0}`")]
    DuplicateInterface(String),

    #[error("Schema `{schema}` contains a duplicate field: `{field}`")]
    DuplicateField { schema: String, field: String },

    #[error("Interface `{interface}` contains a duplicate function: `{function}`")]
    DuplicateFunction { interface: String, function: String },

    #[error("A reference to an unknown schema was found: `{0:?}`")]
    SchemaNotFound(SchemaRef),
}

pub type ValidatorResult<T> = Result<T, ValidatorError>;
