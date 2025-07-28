pub use crate::ast::File;
pub use crate::error::*;

pub type ParserResult<T> = Result<T, ParserError>;
