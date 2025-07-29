use crate::error::ShardError;
use glass_codegen::prelude::{File, ValidatedFile, generate};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Transpiler;

impl Transpiler {
    pub fn transpile_from_directory(
        input_path: &Path,
        output_path: &Path,
    ) -> Result<(), ShardError> {
        // Validate the input path and output path, then extract the flat file hash map.
        crate::cli::check_path(input_path)?;
        Self::prepare_output_directory(output_path)?;

        let file_map = Self::build_file_map(input_path)?;

        // Try to parse each file, skipping with a warning any that failed.
        let validated_files = Self::parse_and_validate_files(&file_map)?;

        // Generate sources and output a HashMap which
        // contains the output path and the content to be outputted.
        let outputs = Self::generate_outputs(output_path, &validated_files, &file_map);

        // Save generated sources to disk
        for (output_path, content) in outputs {
            std::fs::write(output_path, content)?;
        }

        Ok(())
    }

    fn prepare_output_directory(output_path: &Path) -> Result<(), ShardError> {
        let is_valid_dir = crate::cli::check_path(output_path).is_ok();

        if !is_valid_dir {
            std::fs::create_dir_all(output_path)?;
        } else {
            std::fs::remove_dir_all(output_path)?;
            std::fs::create_dir_all(output_path)?;
        }

        Ok(())
    }

    fn build_file_map(input_path: &Path) -> Result<HashMap<String, PathBuf>, ShardError> {
        let mut file_map = HashMap::new();
        Self::get_file_paths(input_path, &mut file_map)?;

        Ok(file_map)
    }

    fn get_file_paths(
        input_path: &Path,
        file_map: &mut HashMap<String, PathBuf>,
    ) -> Result<(), ShardError> {
        // This is safe to unwrap as we previously validated this path exists
        // and is a directory.
        let read_dir = std::fs::read_dir(input_path)?;

        for entry in read_dir {
            let entry = entry?;

            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_name = if file_name.ends_with(".glass") {
                file_name.replace(".glass", ".rs")
            } else {
                continue;
            };

            let canonical_path = entry
                .path()
                .canonicalize()
                .map_err(|error| ShardError::InvalidPath(error.to_string()))?;

            file_map.insert(file_name, canonical_path);
        }

        Ok(())
    }

    fn parse_and_validate_files(
        file_map: &HashMap<String, PathBuf>,
    ) -> Result<Vec<ValidatedFile>, ShardError> {
        let mut validated_files = vec![];

        for file_path in file_map.values() {
            let mut file = match File::try_new(file_path.clone()) {
                Ok(file) => file,
                Err(_) => {
                    println!("🤔 The file '{file_path:#?}' failed to be parsed. Skipping...");
                    continue;
                }
            };

            file.try_parse()?;

            let validated_file = match ValidatedFile::validate(file) {
                Ok(validated) => validated,
                Err(_) => {
                    println!("🤔 The file '{file_path:#?}' failed to be validated. Skipping...");
                    continue;
                }
            };

            validated_files.push(validated_file);
        }

        Ok(validated_files)
    }

    fn generate_outputs(
        output_path: &Path,
        validated_files: &[ValidatedFile],
        file_map: &HashMap<String, PathBuf>,
    ) -> HashMap<PathBuf, String> {
        let mut output_files = HashMap::new();

        for validated_file in validated_files {
            for (name, path) in file_map {
                if validated_file.file.path.eq(path) {
                    let content = generate(validated_file);
                    let output_path = output_path.join(name);

                    output_files.insert(output_path, content);

                    break;
                }
            }
        }

        output_files
    }
}
