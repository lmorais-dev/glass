use crate::GeneratorOutput;
use crate::project::Project;
use glass_parser::ast::{
    Definition, EnumDef, PrimitiveType, Program, SchemaDef, SchemaRef, Type, TypeWithSpan,
};
use glass_parser::type_tree::{TypeTree, TypeTreeError};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::path::PathBuf;
use syn::{Type as SynType, parse_quote};
use thiserror::Error;

#[derive(Clone)]
pub struct RustCodeGenerator<'a> {
    type_tree: &'a TypeTree,
    programs_with_file_names: Vec<(&'a Program, String)>,
}

#[derive(Debug, Error)]
pub enum RustCodeGeneratorError {
    #[error("Type tree error: {0}")]
    TypeTree(#[from] TypeTreeError),

    #[error("Type '{name}' not found")]
    TypeNotFound { name: String },

    #[error("Invalid type reference: {reference}")]
    InvalidTypeReference { reference: String },

    #[error("Formatting error: {0}")]
    Formatting(String),

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    #[error("Syn parsing error: {0}")]
    SynError(String),
}

#[derive(Debug, Clone)]
struct TypeLocation {
    package_segments: Vec<String>,
    file_name: String,
    qualified_path: String, // e.g., "crate::com::example::hello_world::MyType"
}

impl<'a> RustCodeGenerator<'a> {
    pub fn new(
        type_tree: &'a TypeTree,
        programs_with_file_names: Vec<(&'a Program, String)>,
    ) -> Self {
        Self {
            type_tree,
            programs_with_file_names,
        }
    }

    pub fn generate(
        &self,
        project: &Project,
    ) -> Result<Vec<GeneratorOutput>, RustCodeGeneratorError> {
        let generator_config = project.generator_config.rust.as_ref().ok_or_else(|| {
            RustCodeGeneratorError::InvalidConfig {
                message: "No Rust generator configuration found".to_string(),
            }
        })?;

        let mut outputs = Vec::new();

        let type_locations = self.build_type_location_map();

        let programs_by_package = self.group_programs_by_package();
        let lib_rs_output = self.generate_lib_rs(&programs_by_package)?;
        outputs.push(lib_rs_output);

        for (package_name, program_files) in programs_by_package {
            self.generate_package_modules(
                &mut outputs,
                &package_name,
                &program_files,
                &type_locations,
            )?;
        }

        Ok(outputs)
    }

    fn build_type_location_map(&self) -> HashMap<String, TypeLocation> {
        let mut type_locations = HashMap::new();

        for (program, filename) in &self.programs_with_file_names {
            let package_segments = program
                .package
                .as_ref()
                .map(|p| p.path.segments.clone())
                .unwrap_or_else(|| vec!["root".to_string()]);

            let file_stem = filename
                .strip_suffix(".glass")
                .unwrap_or(filename)
                .to_string();

            // Process all type definition in this program
            for definition in &program.definitions {
                let type_name = match definition {
                    Definition::Schema(schema) => &schema.name,
                    Definition::Enum(enum_def) => &enum_def.name,
                    Definition::Service(_) => continue, //TODO: handle service definitions properly
                };

                let qualified_name = if package_segments == vec!["root"] {
                    type_name.clone()
                } else {
                    format!("{}.{type_name}", package_segments.join("."))
                };

                let qualified_path = if package_segments == vec!["root"] {
                    format!("crate::{type_name}")
                } else {
                    format!(
                        "crate::{}::{file_stem}::{type_name}",
                        package_segments.join("::")
                    )
                };

                type_locations.insert(
                    qualified_name,
                    TypeLocation {
                        package_segments: package_segments.clone(),
                        file_name: file_stem.clone(),
                        qualified_path,
                    },
                );
            }
        }

        type_locations
    }

    fn group_programs_by_package(&self) -> HashMap<String, Vec<(&Program, String)>> {
        let mut packages = HashMap::new();

        for (program, filename) in &self.programs_with_file_names {
            let package_name = program
                .package
                .as_ref()
                .map(|p| p.path.to_string())
                .unwrap_or_else(|| "root".to_string());

            packages
                .entry(package_name)
                .or_insert_with(Vec::new)
                .push((program.to_owned(), filename.clone()));
        }

        packages
    }

    fn generate_lib_rs(
        &self,
        programs_by_package: &HashMap<String, Vec<(&Program, String)>>,
    ) -> Result<GeneratorOutput, RustCodeGeneratorError> {
        let mut package_names = programs_by_package.keys().collect::<Vec<_>>();
        package_names.sort();

        let mut mod_declarations = Vec::new();

        for package_name in package_names {
            if package_name == "root" {
                let root_programs = programs_by_package.get("root").unwrap();
                self.generate_mod_declarations(&mut mod_declarations, root_programs);
            } else {
                let first_segment = package_name.split('.').next().unwrap();
                let mod_name = format_ident!("{first_segment}");
                mod_declarations.push(quote! { pub mod #mod_name; });
            }
        }

        let tokens = quote! {
            #(#mod_declarations)*
        };

        Ok(GeneratorOutput {
            path: PathBuf::from("src/lib.rs"),
            content: tokens.to_string(),
        })
    }

    fn generate_package_modules(
        &self,
        outputs: &mut Vec<GeneratorOutput>,
        package_name: &str,
        program_files: &[(&Program, String)],
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<(), RustCodeGeneratorError> {
        // Handle root package by generating files directly in src/
        if package_name == "root" {
            for (program, filename) in program_files {
                let file_stem = filename.strip_suffix(".glass").unwrap_or(filename);
                let file_path = PathBuf::from(format!("src/{file_stem}.rs"));
                let content = self.generate_program_rust_code(program, type_locations)?;
                outputs.push(GeneratorOutput {
                    path: file_path,
                    content,
                });
            }
            return Ok(());
        }

        let package_segments = package_name.split('.').collect::<Vec<_>>();
        self.generate_package_hierarchy(outputs, &package_segments, program_files)?;

        for (program, filename) in program_files {
            let file_stem = filename.strip_suffix(".glass").unwrap_or(filename);
            let package_path = package_segments.join("/");
            let file_path = PathBuf::from(format!("src/{package_path}/{file_stem}.rs"));
            let content = self.generate_program_rust_code(program, type_locations)?;

            outputs.push(GeneratorOutput {
                path: file_path,
                content,
            });
        }

        Ok(())
    }

    fn generate_program_rust_code(
        &self,
        program: &Program,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<String, RustCodeGeneratorError> {
        let mut tokens = Vec::new();

        for definition in &program.definitions {
            match definition {
                Definition::Schema(schema_def) => {
                    let schema_tokens = self.generate_schema_tokens(schema_def, type_locations)?;
                    tokens.push(schema_tokens);
                }
                Definition::Enum(enum_def) => {
                    let enum_tokens = self.generate_enum_tokens(enum_def)?;
                    tokens.push(enum_tokens);
                }
                Definition::Service(_) => {
                    // TODO
                    continue;
                }
            }
        }

        let combined = quote! {
            #(#tokens)*
        };

        Ok(combined.to_string())
    }

    fn generate_schema_tokens(
        &self,
        schema_def: &SchemaDef,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<TokenStream, RustCodeGeneratorError> {
        let struct_name = format_ident!("{}", schema_def.name);
        let mut fields = Vec::new();

        for field in &schema_def.fields {
            let field_name = format_ident!("{}", field.name);
            let field_type = self.convert_type_to_syn(&field.field_type, type_locations)?;
            fields.push(quote! { pub #field_name: #field_type });
        }

        Ok(quote! {
            #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
            pub struct #struct_name {
                #(#fields,)*
            }
        })
    }

    fn convert_type_to_syn(
        &self,
        type_with_span: &TypeWithSpan,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<SynType, RustCodeGeneratorError> {
        match &type_with_span.type_value {
            Type::Primitive(primitive) => {
                let type_str = self.convert_primitive_to_rust(primitive);
                let syn_type: SynType = syn::parse_str(&type_str)
                    .map_err(|e| RustCodeGeneratorError::SynError(e.to_string()))?;
                Ok(syn_type)
            }
            Type::Option(inner) => {
                let inner_type = self.convert_type_to_syn(inner, type_locations)?;
                Ok(parse_quote! { Option<#inner_type> })
            }
            Type::Vec(inner) => {
                let inner_type = self.convert_type_to_syn(inner, type_locations)?;
                Ok(parse_quote! { Vec<#inner_type> })
            }
            Type::SchemaRef(schema_ref) => {
                let qualified_path =
                    self.resolve_schema_reference_path(schema_ref, type_locations)?;
                let syn_type: SynType = syn::parse_str(&qualified_path)
                    .map_err(|e| RustCodeGeneratorError::SynError(e.to_string()))?;
                Ok(syn_type)
            }
        }
    }

    fn resolve_schema_reference_path(
        &self,
        schema_ref: &SchemaRef,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<String, RustCodeGeneratorError> {
        let qualified_name = if let Some(package) = &schema_ref.package {
            format!("{}.{}", package, schema_ref.name)
        } else {
            self.find_unqualified_type(&schema_ref.name, type_locations)?
        };

        let location = type_locations.get(&qualified_name).ok_or_else(|| {
            RustCodeGeneratorError::TypeNotFound {
                name: qualified_name.clone(),
            }
        })?;

        Ok(location.qualified_path.clone())
    }

    fn find_unqualified_type(
        &self,
        type_name: &str,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<String, RustCodeGeneratorError> {
        // Look for all exact matches first
        if type_locations.contains_key(type_name) {
            return Ok(type_name.to_string());
        }

        let candidates = type_locations
            .keys()
            .filter(|key| key.ends_with(&format!(".{type_name}")) || key == &type_name)
            .collect::<Vec<_>>();

        match candidates.len() {
            0 => Err(RustCodeGeneratorError::TypeNotFound {
                name: type_name.to_string(),
            }),
            1 => Ok(candidates[0].clone()),
            _ => {
                let mut candidates_str = String::new();
                for candidate in candidates {
                    candidates_str.push_str(&format!("{candidate}, "));
                }
                Err(RustCodeGeneratorError::TypeNotFound {
                    name: format!("Ambiguous type reference: {candidates_str}"),
                })
            }
        }
    }

    fn generate_package_hierarchy(
        &self,
        outputs: &mut Vec<GeneratorOutput>,
        package_segments: &[&str],
        program_files: &[(&Program, String)],
    ) -> Result<(), RustCodeGeneratorError> {
        for i in 1..=package_segments.len() {
            let segments = &package_segments[..i];
            let module_path = segments.join("/");
            let mod_rs_path = PathBuf::from(format!("src/{module_path}/mod.rs"));

            // Check if this was already generated
            if outputs.iter().any(|output| output.path == mod_rs_path) {
                continue;
            }

            let mut mod_declarations = Vec::new();
            if i < package_segments.len() {
                let next_segment = package_segments[i];
                let next_mod_name = format_ident!("{next_segment}");
                mod_declarations.push(quote! { pub mod #next_mod_name; });
            } else {
                self.generate_mod_declarations(&mut mod_declarations, program_files);
            }

            let tokens = quote! {
                #(#mod_declarations)*
            };

            outputs.push(GeneratorOutput {
                path: mod_rs_path,
                content: tokens.to_string(),
            });
        }
        Ok(())
    }

    fn generate_mod_declarations(
        &self,
        mod_declarations: &mut Vec<TokenStream>,
        program_files: &[(&Program, String)],
    ) {
        for (_, filename) in program_files {
            let file_stem = filename.strip_suffix(".glass").unwrap_or(filename);
            let mod_name = format_ident!("{file_stem}");
            mod_declarations.push(quote! { pub mod #mod_name; });
        }
    }

    fn generate_enum_tokens(
        &self,
        enum_def: &EnumDef,
    ) -> Result<TokenStream, RustCodeGeneratorError> {
        let enum_name = format_ident!("{}", enum_def.name);
        let mut variants = Vec::new();

        for variant in &enum_def.variants {
            let variant_name = format_ident!("{variant}");
            variants.push(quote! { #variant_name });
        }

        Ok(quote! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
            pub enum #enum_name {
                #(#variants,)*
            }
        })
    }

    fn convert_primitive_to_rust(&self, primitive: &PrimitiveType) -> String {
        match primitive {
            PrimitiveType::String => "String".to_string(),
            PrimitiveType::U8 => "u8".to_string(),
            PrimitiveType::U16 => "u16".to_string(),
            PrimitiveType::U32 => "u32".to_string(),
            PrimitiveType::U64 => "u64".to_string(),
            PrimitiveType::U128 => "u128".to_string(),
            PrimitiveType::I8 => "i8".to_string(),
            PrimitiveType::I16 => "i16".to_string(),
            PrimitiveType::I32 => "i32".to_string(),
            PrimitiveType::I64 => "i64".to_string(),
            PrimitiveType::I128 => "i128".to_string(),
            PrimitiveType::F32 => "f32".to_string(),
            PrimitiveType::F64 => "f64".to_string(),
            PrimitiveType::Bool => "bool".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{GeneratorConfig, Project, RustGeneratorConfig};
    use glass_parser::ast::{
        Definition, EnumDef, PackageDecl, PackagePath, PrimitiveType, Program, SchemaDef,
        SchemaField, Span, Type, TypeWithSpan,
    };
    use glass_parser::parser::Parser;
    use glass_parser::type_tree::TypeTree;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_hello_world_glass() -> String {
        r#"package com.example;

service Greeter {
    fn greet({ name: string }) -> { message: string };
}
"#
        .to_string()
    }

    fn parse_glass_file(content: &str) -> Program {
        Parser::parse(content.to_string()).expect("Failed to parse glass file")
    }

    fn create_type_tree(programs: &[Program]) -> TypeTree {
        TypeTree::from_programs(programs).expect("Failed to create type tree")
    }

    #[test]
    fn test_rust_code_generator_basic() {
        // Create a temporary directory for output
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let out_dir = temp_dir.path().to_path_buf();

        // Create and parse a hello world glass file
        let hello_world_content = create_hello_world_glass();
        let program = parse_glass_file(&hello_world_content);
        let programs = vec![program];

        // Create a type tree
        let type_tree = create_type_tree(&programs);

        // Create the code generator
        let programs_with_file_names = vec![(&programs[0], "hello_world.glass".to_string())];
        let generator = RustCodeGenerator::new(&type_tree, programs_with_file_names);

        // Create a project configuration
        let project = Project {
            root_path: temp_dir.path().to_path_buf(),
            generator_config: GeneratorConfig {
                rust: Some(RustGeneratorConfig {
                    out_dir,
                    cargo_template: None,
                }),
            },
        };

        // Generate code
        let outputs = generator
            .generate(&project)
            .expect("Code generation failed");

        // Verify outputs
        assert!(!outputs.is_empty(), "No outputs were generated");

        // Check for lib.rs
        let lib_rs = outputs
            .iter()
            .find(|output| output.path == Path::new("src/lib.rs"));
        assert!(lib_rs.is_some(), "lib.rs was not generated");

        // Check for com module
        let com_module = outputs
            .iter()
            .find(|output| output.path == Path::new("src/com/mod.rs"));
        assert!(com_module.is_some(), "com module was not generated");

        // Check for example module
        let example_module = outputs
            .iter()
            .find(|output| output.path == Path::new("src/com/example/mod.rs"));
        assert!(example_module.is_some(), "example module was not generated");

        // Check for hello_world.rs
        let hello_world = outputs
            .iter()
            .find(|output| output.path == Path::new("src/com/example/hello_world.rs"));
        assert!(hello_world.is_some(), "hello_world.rs was not generated");
    }

    #[test]
    fn test_rust_code_generator_no_config() {
        // Create a temporary directory for output
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create and parse a hello world glass file
        let hello_world_content = create_hello_world_glass();
        let program = parse_glass_file(&hello_world_content);
        let programs = vec![program];

        // Create a type tree
        let type_tree = create_type_tree(&programs);

        // Create the code generator
        let programs_with_file_names = vec![(&programs[0], "hello_world.glass".to_string())];
        let generator = RustCodeGenerator::new(&type_tree, programs_with_file_names);

        // Create a project configuration without Rust config
        let project = Project {
            root_path: temp_dir.path().to_path_buf(),
            generator_config: GeneratorConfig { rust: None },
        };

        // Generate code - should fail with InvalidConfig error
        let result = generator.generate(&project);
        assert!(
            result.is_err(),
            "Generation should fail without Rust config"
        );

        match result {
            Err(RustCodeGeneratorError::InvalidConfig { .. }) => {
                // This is the expected error
            }
            _ => panic!("Expected InvalidConfig error"),
        }
    }

    #[test]
    fn test_rust_code_generator_with_schema() {
        // Create a temporary directory for output
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let out_dir = temp_dir.path().to_path_buf();

        // Create a program with a schema definition
        let package = PackageDecl {
            path: PackagePath {
                segments: vec!["com".to_string(), "example".to_string()],
                span: Span::dummy(),
            },
            span: Span::dummy(),
        };

        let schema = SchemaDef {
            name: "Person".to_string(),
            fields: vec![
                SchemaField {
                    name: "name".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::Primitive(PrimitiveType::String),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                },
                SchemaField {
                    name: "age".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::Primitive(PrimitiveType::U32),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                },
            ],
            span: Span::dummy(),
        };

        let program = Program {
            package: Some(package),
            imports: vec![],
            definitions: vec![Definition::Schema(schema)],
            span: Span::dummy(),
        };

        let programs = vec![program];

        // Create a type tree
        let type_tree = create_type_tree(&programs);

        // Create the code generator
        let programs_with_file_names = vec![(&programs[0], "person.glass".to_string())];
        let generator = RustCodeGenerator::new(&type_tree, programs_with_file_names);

        // Create a project configuration
        let project = Project {
            root_path: temp_dir.path().to_path_buf(),
            generator_config: GeneratorConfig {
                rust: Some(RustGeneratorConfig {
                    out_dir,
                    cargo_template: None,
                }),
            },
        };

        // Generate code
        let outputs = generator
            .generate(&project)
            .expect("Code generation failed");

        // Verify outputs
        assert!(!outputs.is_empty(), "No outputs were generated");

        // Check for person.rs
        let person_rs = outputs
            .iter()
            .find(|output| output.path == Path::new("src/com/example/person.rs"));
        assert!(person_rs.is_some(), "person.rs was not generated");

        // Check that the Person struct is in the output
        let person_content = &person_rs.unwrap().content;
        println!("Generated Person struct content:\n{person_content}");
        assert!(
            person_content.contains("pub struct Person"),
            "Person struct isn't found in output"
        );
        assert!(
            person_content.contains("pub name"),
            "name field isn't found in Person struct"
        );
        assert!(
            person_content.contains("String"),
            "String type isn't found for name field"
        );
        assert!(
            person_content.contains("pub age"),
            "age field isn't found in Person struct"
        );
        assert!(
            person_content.contains("u32"),
            "u32 type not found for age field"
        );
    }

    #[test]
    fn test_rust_code_generator_with_enum() {
        // Create a temporary directory for output
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let out_dir = temp_dir.path().to_path_buf();

        // Create a program with an enum definition
        let package = PackageDecl {
            path: PackagePath {
                segments: vec!["com".to_string(), "example".to_string()],
                span: Span::dummy(),
            },
            span: Span::dummy(),
        };

        let enum_def = EnumDef {
            name: "Status".to_string(),
            variants: vec![
                "Active".to_string(),
                "Inactive".to_string(),
                "Pending".to_string(),
            ],
            span: Span::dummy(),
        };

        let program = Program {
            package: Some(package),
            imports: vec![],
            definitions: vec![Definition::Enum(enum_def)],
            span: Span::dummy(),
        };

        let programs = vec![program];

        // Create a type tree
        let type_tree = create_type_tree(&programs);

        // Create the code generator
        let programs_with_file_names = vec![(&programs[0], "status.glass".to_string())];
        let generator = RustCodeGenerator::new(&type_tree, programs_with_file_names);

        // Create a project configuration
        let project = Project {
            root_path: temp_dir.path().to_path_buf(),
            generator_config: GeneratorConfig {
                rust: Some(RustGeneratorConfig {
                    out_dir,
                    cargo_template: None,
                }),
            },
        };

        // Generate code
        let outputs = generator
            .generate(&project)
            .expect("Code generation failed");

        // Verify outputs
        assert!(!outputs.is_empty(), "No outputs were generated");

        // Check for status.rs
        let status_rs = outputs
            .iter()
            .find(|output| output.path == Path::new("src/com/example/status.rs"));
        assert!(status_rs.is_some(), "status.rs was not generated");

        // Check that the Status enum is in the output
        let status_content = &status_rs.unwrap().content;
        assert!(
            status_content.contains("pub enum Status"),
            "Status enum not found in output"
        );
        assert!(
            status_content.contains("Active"),
            "Active variant isn't found in Status enum"
        );
        assert!(
            status_content.contains("Inactive"),
            "Inactive variant isn't found in Status enum"
        );
        assert!(
            status_content.contains("Pending"),
            "Pending variant isn't found in Status enum"
        );
    }
}
