pub mod error;

use crate::ast::interface::{FunctionParam, FunctionReturn, Interface};
use crate::ast::schema::{Schema, SchemaRef};
use crate::ast::types::Type;
use crate::prelude::*;
use crate::validator::error::{ValidatorError, ValidatorResult};
use std::collections::{HashMap, HashSet};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct ValidatedFile {
    file: File,
    schema_map: HashMap<SchemaRef, Schema>,
    interface_map: HashMap<String, Interface>,
}

impl ValidatedFile {
    #[tracing::instrument(skip_all, fields(path = ?file.path.to_str()))]
    pub fn validate(file: File) -> ValidatorResult<Self> {
        info!("Semantic validation has begun");

        let schema_map = Self::build_schema_map(&file.schemas)?;
        let interface_map = Self::build_interface_map(&file.interfaces)?;

        Self::validate_schema_ref(&file, &schema_map)?;

        Ok(Self {
            file,
            schema_map,
            interface_map,
        })
    }

    fn build_schema_map(schemas: &[Schema]) -> ValidatorResult<HashMap<SchemaRef, Schema>> {
        let mut schema_map = HashMap::with_capacity(schemas.len());
        for schema in schemas {
            if !schema_map
                .keys()
                .filter(|&key| key == &SchemaRef(schema.name.clone()))
                .collect::<Vec<_>>()
                .is_empty()
            {
                error!(schema_name = ?schema.name, "Duplicated schema detected");
                return Err(ValidatorError::DuplicateSchema(schema.name.clone()));
            }

            let mut field_names = HashSet::new();
            for field in &schema.fields {
                if !field_names.insert(field.name.clone()) {
                    error!(schema_name = ?schema.name, field_name = ?field.name, "Duplicate field in schema detected");
                    return Err(ValidatorError::DuplicateField {
                        schema: schema.name.clone(),
                        field: field.name.clone(),
                    });
                }
            }
            schema_map.insert(SchemaRef(schema.name.clone()), schema.clone());
        }

        Ok(schema_map)
    }

    fn build_interface_map(
        interfaces: &[Interface],
    ) -> ValidatorResult<HashMap<String, Interface>> {
        let mut interface_map = HashMap::with_capacity(interfaces.len());
        for interface in interfaces {
            if !interface_map
                .keys()
                .filter(|&key| key == &interface.name)
                .collect::<Vec<_>>()
                .is_empty()
            {
                error!(interface_name = ?interface.name, "Duplicated interface detected");
                return Err(ValidatorError::DuplicateInterface(interface.name.clone()));
            }

            let mut function_names = HashSet::new();
            for function in &interface.functions {
                if !function_names.insert(function.name.clone()) {
                    error!(interface_name = ?interface.name, function_name = ?function.name, "Duplicate function in interface detected");
                    return Err(ValidatorError::DuplicateFunction {
                        interface: interface.name.clone(),
                        function: function.name.clone(),
                    });
                }
            }

            interface_map.insert(interface.name.clone(), interface.clone());
        }

        Ok(interface_map)
    }

    fn validate_schema_ref(
        file: &File,
        schema_map: &HashMap<SchemaRef, Schema>,
    ) -> ValidatorResult<()> {
        for schema in &file.schemas {
            for field in &schema.fields {
                Self::validate_type(&field.ty, schema_map)?;
            }
        }

        for interface in &file.interfaces {
            for function in &interface.functions {
                Self::validate_function_param(&function.param, schema_map)?;
                if let Some(return_type) = &function.return_type {
                    Self::validate_function_return(return_type, schema_map)?;
                }
            }
        }

        Ok(())
    }

    fn validate_type(ty: &Type, schema_map: &HashMap<SchemaRef, Schema>) -> ValidatorResult<()> {
        match ty {
            Type::Primitive(_) => Ok(()),
            Type::Schema(schema_ref) => {
                if schema_map
                    .keys()
                    .filter(|&key| key == schema_ref)
                    .collect::<Vec<_>>()
                    .is_empty()
                {
                    error!(?schema_ref, "Reference to an undefined schema defined");
                    Err(ValidatorError::SchemaNotFound(schema_ref.clone()))
                } else {
                    Ok(())
                }
            }
            Type::Option(option_type) => Self::validate_type(&option_type.inner, schema_map),
            Type::Vector(vector_type) => Self::validate_type(&vector_type.inner, schema_map),
        }
    }

    fn validate_function_param(
        param: &FunctionParam,
        schema_map: &HashMap<SchemaRef, Schema>,
    ) -> ValidatorResult<()> {
        match param {
            FunctionParam::Stream(fn_type) => Self::validate_type(fn_type, schema_map),
            FunctionParam::Simple(fn_type) => Self::validate_type(fn_type, schema_map),
        }
    }

    fn validate_function_return(
        fn_return: &FunctionReturn,
        schema_map: &HashMap<SchemaRef, Schema>,
    ) -> ValidatorResult<()> {
        match fn_return {
            FunctionReturn::Stream(return_type) => Self::validate_type(return_type, schema_map),
            FunctionReturn::Simple(return_type) => Self::validate_type(return_type, schema_map),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::validator::{ValidatedFile, ValidatorError};
    use std::fs::File as StdFile;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::Builder;

    /// Helper to create a named temporary file with specific content.
    fn create_temp_file(prefix: &str, content: &str) -> (PathBuf, impl FnOnce()) {
        let temp_dir = Builder::new().prefix(prefix).tempdir().unwrap();
        let file_path = temp_dir.path().join("test.glass");
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_fmt(format_args!("{content}")).unwrap();

        let path_buf = file_path.to_path_buf();
        let cleanup = move || temp_dir.close().unwrap();

        (path_buf, cleanup)
    }

    #[test]
    fn test_validate_success() {
        let content = r#"
            schema User {
                id: u64;
            }

            interface Greeter {
                fn say_hello(User) -> string;
            }
        "#;
        let (path, cleanup) = create_temp_file("validate_success", content);
        let mut file = File::try_new(path).unwrap();
        file.try_parse().unwrap();

        let result = ValidatedFile::validate(file);
        assert!(result.is_ok());

        cleanup();
    }

    #[test]
    fn test_validate_duplicate_schema() {
        let content = r#"
            schema User { id: u64; }
            schema User { name: string; }
        "#;
        let (path, cleanup) = create_temp_file("duplicate_schema", content);
        let mut file = File::try_new(path).unwrap();
        file.try_parse().unwrap();

        let result = ValidatedFile::validate(file);
        assert!(matches!(result, Err(ValidatorError::DuplicateSchema(_))));

        cleanup();
    }

    #[test]
    fn test_validate_duplicate_interface() {
        let content = r#"
            interface Greeter { fn a(string); }
            interface Greeter { fn b(string); }
        "#;
        let (path, cleanup) = create_temp_file("duplicate_interface", content);
        let mut file = File::try_new(path).unwrap();
        file.try_parse().unwrap();

        let result = ValidatedFile::validate(file);
        assert!(matches!(result, Err(ValidatorError::DuplicateInterface(_))));

        cleanup();
    }

    #[test]
    fn test_validate_undefined_schema_ref() {
        let content = r#"
            interface Greeter {
                fn say_hello(User) -> string;
            }
        "#;
        let (path, cleanup) = create_temp_file("undefined_schema_ref", content);
        let mut file = File::try_new(path).unwrap();
        file.try_parse().unwrap();

        let result = ValidatedFile::validate(file);
        assert!(matches!(result, Err(ValidatorError::SchemaNotFound(_))));

        cleanup();
    }
}
