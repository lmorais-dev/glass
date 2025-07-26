use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Project {
    pub root_path: PathBuf,
    pub generator_config: GeneratorConfig,
}

#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    pub rust: Option<RustGeneratorConfig>,
}

#[derive(Debug, Clone)]
pub struct RustGeneratorConfig {
    pub out_dir: PathBuf,
    pub cargo_template: Option<PathBuf>,
}
