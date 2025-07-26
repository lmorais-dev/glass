use thiserror::Error;

/// Error types for import validation
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ImportValidationError {
    #[error(
        "Import '{import_path}' from '{from_file}' resolves to '{resolved_path}' which was not found"
    )]
    FileNotFound {
        import_path: String,
        from_file: String,
        resolved_path: String,
    },

    #[error("Circular import detected: {cycle:?}")]
    CircularImport { cycle: Vec<String> },
}

/// Error types for type tree operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TypeTreeError {
    #[error("Circular dependency detected: {path}")]
    CircularDependency { path: String },
    #[error("Type '{name}' not found in any program")]
    TypeNotFound { name: String },
    #[error("Invalid schema reference: {reference}")]
    InvalidSchemaReference { reference: String },
    #[error("Import file '{file_path}' not found in any program")]
    ImportFileNotFound { file_path: String },
    #[error("Type '{type_name}' is not accessible from file '{from_file}'. Missing import?")]
    TypeNotAccessible {
        type_name: String,
        from_file: String,
    },
    #[error("Circular import detected: {cycle:?}")]
    CircularImport { cycle: Vec<String> },
    #[error("Import path '{import_path}' could not be resolved from '{from_file}'")]
    UnresolvableImport {
        import_path: String,
        from_file: String,
    },
}
