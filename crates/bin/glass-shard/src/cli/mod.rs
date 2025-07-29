use crate::error::ShardError;
use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to a directory containing Glass files
    #[arg(short, long)]
    pub sources: PathBuf,

    /// Path to a directory where Rust files will be generated.
    ///
    /// This will overwrite any file inside the folder, please be sure when running.
    #[arg(short, long)]
    pub output: PathBuf,
}

/// Checks if a path exists and is a directory.
///
/// This is needed to validate the input folder is at the very least
/// a valid path to operate on.
pub fn check_path(path: &Path) -> Result<(), ShardError> {
    if !path.exists() {
        return Err(ShardError::InexistentPath(
            path.to_string_lossy().to_string(),
        ));
    }

    if !path.is_dir() {
        return Err(ShardError::NotDirectory(path.to_string_lossy().to_string()));
    }

    Ok(())
}
