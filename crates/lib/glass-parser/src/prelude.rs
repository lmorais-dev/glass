pub use crate::ast::File;
pub use crate::error::*;
pub use crate::validator::ValidatedFile;

pub type ParserResult<T> = Result<T, ParserError>;
