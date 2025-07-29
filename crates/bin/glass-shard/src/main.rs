use crate::cli::Cli;
use crate::error::ShardError;
use clap::Parser;

mod cli;
mod error;
mod transpiler;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    let result = transpiler::Transpiler::transpile_from_directory(&cli.sources, &cli.output);
    match result {
        Ok(()) => println!("🚀 Transpilation successful!"),
        Err(error) => match error {
            ShardError::InvalidPath(path) => {
                eprintln!("😢 Invalid path detected: {path}");
            }
            ShardError::InexistentPath(path) => {
                eprintln!("😢 Inexistent path detected: {path}");
            }
            ShardError::NotDirectory(path) => {
                eprintln!("😢 Path is not a directory: {path}");
            }
            ShardError::GeneralIo(_) => {
                eprintln!("😭 Unexpected IO error");
            }
            ShardError::Parser(error) => {
                eprintln!("😭 Unexpected Parser error: {error}");
            }
        },
    }

    Ok(())
}
