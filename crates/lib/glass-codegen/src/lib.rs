use std::path::PathBuf;

pub mod project;
pub mod rust;

#[derive(Clone)]
pub struct GeneratorOutput {
    pub path: PathBuf,
    pub content: String,
}
