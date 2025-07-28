pub use crate::error::*;
pub use crate::ast::File;

pub type ParserResult<T> = Result<T, ParserError>;