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
            if schema_map
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
            if interface_map
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
                if !schema_map
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
