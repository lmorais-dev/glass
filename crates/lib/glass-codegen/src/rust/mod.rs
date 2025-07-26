//! Rust code generator for Glass
//!
//! The current implementation includes a Rust code generator that converts
//! Glass schemas and enums to Rust structs and enums.
//!
//! The main components are:
//! - `RustGenerator`: Implements the `CodeGenerator` trait for Rust code generation
//! - `create_rust_generator`: Factory function to create a Rust code generator

use crate::project::Project;
use crate::{CodeGenerator, GeneratorOutput};
use glass_parser::ast::{Definition, EnumDef, PrimitiveType, Program, SchemaDef, SchemaRef, Type, TypeWithSpan};
use glass_parser::type_tree::TypeTree;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::error::CodeGeneratorError;

pub mod old;

/// Rust code generator implementation
///
/// This struct implements the `CodeGenerator` trait for generating Rust code from Glass programs.
/// It uses the `TypeTree` from the glass-parser crate to handle type resolution and dependencies.
///
/// # Examples
///
/// ```rust,no_run
/// use glass_codegen::rust::create_rust_generator;
/// use glass_codegen::CodeGenerator;
/// use glass_codegen::project::Project;
/// use glass_parser::ast::Program;
/// use glass_parser::type_tree::TypeTree;
///
/// // Create a type tree from Glass programs
/// let programs = vec![/* Glass programs */];
/// let type_tree = TypeTree::from_programs(&programs).unwrap();
///
/// // Create a Rust code generator
/// let programs_with_file_names = vec![(programs.first().unwrap(), "example.glass".to_string())];
/// let generator = create_rust_generator(&type_tree, programs_with_file_names);
///
/// // Generate Rust code
/// let project = Project {
///     root_path: std::path::PathBuf::from("output"),
///     generator_config: glass_codegen::project::GeneratorConfig {
///         rust: Some(glass_codegen::project::RustGeneratorConfig {
///             out_dir: std::path::PathBuf::from("src"),
///             cargo_template: None,
///         }),
///     },
/// };
/// let outputs = generator.generate(&project).unwrap();
/// ```
pub struct RustGenerator<'a> {
    /// The type tree built from the programs
    type_tree: &'a TypeTree,
    
    /// The programs with their file names
    programs_with_file_names: Vec<(&'a Program, String)>,
}

impl<'a> RustGenerator<'a> {
    /// Create a new Rust code generator
    pub fn new(type_tree: &'a TypeTree, programs_with_file_names: Vec<(&'a Program, String)>) -> Self {
        Self {
            type_tree,
            programs_with_file_names,
        }
    }
    
    /// Build a map of type locations for resolving type references
    fn build_type_location_map(&self) -> HashMap<String, TypeLocation> {
        let mut type_locations = HashMap::new();
        
        for (program_idx, (program, file_name)) in self.programs_with_file_names.iter().enumerate() {
            let package_name = program.package.as_ref().map(|p| p.path.segments.join("."));
            
            for definition in &program.definitions {
                match definition {
                    Definition::Schema(schema) => {
                        let qualified_name = if let Some(ref pkg) = package_name {
                            format!("{}.{}", pkg, schema.name)
                        } else {
                            schema.name.clone()
                        };
                        
                        type_locations.insert(qualified_name.clone(), TypeLocation {
                            qualified_name,
                            program_idx,
                            file_name: file_name.clone(),
                            package: package_name.clone(),
                        });
                    }
                    Definition::Enum(enum_def) => {
                        let qualified_name = if let Some(ref pkg) = package_name {
                            format!("{}.{}", pkg, enum_def.name)
                        } else {
                            enum_def.name.clone()
                        };
                        
                        type_locations.insert(qualified_name.clone(), TypeLocation {
                            qualified_name,
                            program_idx,
                            file_name: file_name.clone(),
                            package: package_name.clone(),
                        });
                    }
                    Definition::Service(_service) => {
                        // Services don't define types, so we don't need to add them to type_locations
                    }
                }
            }
        }
        
        type_locations
    }
    
    /// Group programs by package for organizing output
    fn group_programs_by_package(&self) -> HashMap<String, Vec<(&'a Program, String)>> {
        let mut programs_by_package = HashMap::new();
        
        for (program, file_name) in &self.programs_with_file_names {
            let package_name = program.package.as_ref()
                .map(|p| p.path.segments.join("."))
                .unwrap_or_else(|| "".to_string());
            
            programs_by_package
                .entry(package_name)
                .or_insert_with(Vec::new)
                .push((*program, file_name.clone()));
        }
        
        programs_by_package
    }
}

/// Represents the location of a type in the program
#[derive(Debug, Clone)]
struct TypeLocation {
    /// The fully qualified name of the type
    qualified_name: String,
    
    /// The index of the program in the program list
    program_idx: usize,
    
    /// The file name of the program
    file_name: String,
    
    /// The package name of the program
    package: Option<String>,
}

impl<'a> CodeGenerator for RustGenerator<'a> {
    type Error = CodeGeneratorError;
    
    fn generate(&self, project: &Project) -> Result<Vec<GeneratorOutput>, Self::Error> {
        // Check if Rust generation is configured
        let rust_config = match &project.generator_config.rust {
            Some(config) => config,
            None => return Err(CodeGeneratorError::InvalidConfig { 
                message: "Rust generator configuration is missing".to_string() 
            }),
        };
        
        let mut outputs = Vec::new();
        
        // Build type location map for resolving type references
        let type_locations = self.build_type_location_map();
        
        // Group programs by package
        let programs_by_package = self.group_programs_by_package();
        
        // Generate lib.rs
        let lib_rs = self.generate_lib_rs(&programs_by_package)?;
        outputs.push(lib_rs);
        
        // Generate package modules
        for (package_name, program_files) in &programs_by_package {
            self.generate_package_modules(
                &mut outputs,
                package_name,
                program_files,
                &type_locations,
                &rust_config.out_dir,
            )?;
        }
        
        Ok(outputs)
    }
    
    fn name(&self) -> &'static str {
        "rust"
    }
}

// Implementation of the Rust code generator methods
impl<'a> RustGenerator<'a> {
    /// Generate the lib.rs file
    fn generate_lib_rs(
        &self,
        programs_by_package: &HashMap<String, Vec<(&'a Program, String)>>
    ) -> Result<GeneratorOutput, CodeGeneratorError> {
        use quote::{format_ident, quote};
        let mut mod_declarations = Vec::new();
        
        // Generate mod declarations for each package
        for package_name in programs_by_package.keys() {
            if package_name.is_empty() {
                continue;
            }
            
            let package_segments: Vec<&str> = package_name.split('.').collect();
            let root_mod = format_ident!("{}", package_segments[0]);
            
            mod_declarations.push(quote! {
                pub mod #root_mod;
            });
        }
        
        // Generate mod declarations for root-level programs
        if let Some(root_programs) = programs_by_package.get("") {
            self.generate_mod_declarations(&mut mod_declarations, root_programs);
        }
        
        // Combine all mod declarations
        let mod_declarations = quote! {
            #(#mod_declarations)*
        };
        
        // Convert to string
        let content = mod_declarations.to_string();
        
        Ok(GeneratorOutput {
            path: PathBuf::from("lib.rs"),
            content,
        })
    }
    
    /// Generate module declarations for programs
    fn generate_mod_declarations(
        &self,
        mod_declarations: &mut Vec<proc_macro2::TokenStream>,
        program_files: &[(&'a Program, String)]
    ) {
        use quote::{format_ident, quote};
        
        for (_program, file_name) in program_files {
            // Extract module name from file name
            let file_stem = Path::new(file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            
            let mod_name = format_ident!("{}", file_stem);
            
            mod_declarations.push(quote! {
                pub mod #mod_name;
            });
        }
    }
    
    /// Generate package modules
    fn generate_package_modules(
        &self,
        outputs: &mut Vec<GeneratorOutput>,
        package_name: &str,
        program_files: &[(&'a Program, String)],
        type_locations: &HashMap<String, TypeLocation>,
        _out_dir: &Path,
    ) -> Result<(), CodeGeneratorError> {
        if package_name.is_empty() {
            // Handle root-level programs
            for (program, file_name) in program_files {
                let file_stem = Path::new(file_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                
                let rust_code = self.generate_program_rust_code(program, type_locations)?;
                
                outputs.push(GeneratorOutput {
                    path: PathBuf::from(format!("{file_stem}.rs")),
                    content: rust_code,
                });
            }
        } else {
            // Handle package modules
            let package_segments: Vec<&str> = package_name.split('.').collect();
            
            // Generate package hierarchy
            self.generate_package_hierarchy(outputs, &package_segments, program_files)?;
            
            // Generate Rust code for each program in the package
            for (program, file_name) in program_files {
                let file_stem = Path::new(file_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                
                let rust_code = self.generate_program_rust_code(program, type_locations)?;
                
                let mut path = PathBuf::new();
                for segment in &package_segments {
                    path.push(segment);
                }
                path.push(format!("{file_stem}.rs"));
                
                outputs.push(GeneratorOutput {
                    path,
                    content: rust_code,
                });
            }
        }
        
        Ok(())
    }
    
    /// Generate package hierarchy
    fn generate_package_hierarchy(
        &self,
        outputs: &mut Vec<GeneratorOutput>,
        package_segments: &[&str],
        program_files: &[(&'a Program, String)]
    ) -> Result<(), CodeGeneratorError> {
        use quote::{format_ident, quote};
        
        // Generate mod.rs files for each level of the package hierarchy
        let mut current_path = PathBuf::new();
        
        for (i, &segment) in package_segments.iter().enumerate() {
            current_path.push(segment);
            
            let mut mod_declarations = Vec::new();
            
            if i == package_segments.len() - 1 {
                // This is the leaf package, include the program modules
                for (_, file_name) in program_files {
                    let file_stem = Path::new(file_name)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    
                    let mod_name = format_ident!("{}", file_stem);
                    
                    mod_declarations.push(quote! {
                        pub mod #mod_name;
                    });
                }
            } else {
                // This is an intermediate package, include the next segment
                let next_segment = format_ident!("{}", package_segments[i + 1]);
                
                mod_declarations.push(quote! {
                    pub mod #next_segment;
                });
            }
            
            let mod_declarations = quote! {
                #(#mod_declarations)*
            };
            
            let mut mod_path = current_path.clone();
            mod_path.push("mod.rs");
            
            outputs.push(GeneratorOutput {
                path: mod_path,
                content: mod_declarations.to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Generate Rust code for a program
    fn generate_program_rust_code(
        &self,
        program: &'a Program,
        type_locations: &HashMap<String, TypeLocation>
    ) -> Result<String, CodeGeneratorError> {
        use proc_macro2::TokenStream;
        
        let mut tokens = TokenStream::new();
        
        // Generate code for each definition in the program
        for definition in &program.definitions {
            match definition {
                Definition::Schema(schema) => {
                    let schema_tokens = self.generate_schema_tokens(schema, type_locations)?;
                    tokens.extend(schema_tokens);
                }
                Definition::Enum(enum_def) => {
                    let enum_tokens = self.generate_enum_tokens(enum_def)?;
                    tokens.extend(enum_tokens);
                }
                Definition::Service(_) => {
                    // Skip service definitions for now
                    // In a future version, we could generate client/server code for services
                }
            }
        }
        
        Ok(tokens.to_string())
    }
    
    /// Generate Rust code for a schema
    fn generate_schema_tokens(
        &self,
        schema_def: &'a SchemaDef,
        type_locations: &HashMap<String, TypeLocation>
    ) -> Result<proc_macro2::TokenStream, CodeGeneratorError> {
        use quote::{format_ident, quote};
        let struct_name = format_ident!("{}", &schema_def.name);
        
        let mut field_tokens = Vec::new();
        
        for field in &schema_def.fields {
            let field_name = format_ident!("{}", &field.name);
            let field_type = self.convert_type_to_syn(&field.field_type, type_locations)?;
            
            field_tokens.push(quote! {
                pub #field_name: #field_type,
            });
        }
        
        let tokens = quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #struct_name {
                #(#field_tokens)*
            }
        };
        
        Ok(tokens)
    }
    
    /// Generate Rust code for an enum
    fn generate_enum_tokens(
        &self,
        enum_def: &'a EnumDef
    ) -> Result<proc_macro2::TokenStream, CodeGeneratorError> {
        use quote::{format_ident, quote};
        
        let enum_name = format_ident!("{}", &enum_def.name);
        
        let mut variant_tokens = Vec::new();
        
        for variant in &enum_def.variants {
            let variant_name = format_ident!("{}", variant);
            
            variant_tokens.push(quote! {
                #variant_name,
            });
        }
        
        let tokens = quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub enum #enum_name {
                #(#variant_tokens)*
            }
        };
        
        Ok(tokens)
    }
    
    /// Convert a Glass type to a Rust type
    fn convert_type_to_syn(
        &self,
        type_with_span: &'a TypeWithSpan,
        type_locations: &HashMap<String, TypeLocation>
    ) -> Result<syn::Type, CodeGeneratorError> {
        use syn::parse_quote;
        
        match &type_with_span.type_value {
            Type::Primitive(primitive) => {
                let rust_type = self.convert_primitive_to_rust(primitive);
                Ok(parse_quote!(#rust_type))
            }
            Type::Option(inner) => {
                let inner_type = self.convert_type_to_syn(inner, type_locations)?;
                Ok(parse_quote!(Option<#inner_type>))
            }
            Type::Vec(inner) => {
                let inner_type = self.convert_type_to_syn(inner, type_locations)?;
                Ok(parse_quote!(Vec<#inner_type>))
            }
            Type::SchemaRef(schema_ref) => {
                let type_path = self.resolve_schema_reference_path(schema_ref, type_locations)?;
                Ok(parse_quote!(#type_path))
            }
        }
    }
    
    /// Resolve a schema reference to a Rust path
    fn resolve_schema_reference_path(
        &self,
        schema_ref: &'a SchemaRef,
        type_locations: &HashMap<String, TypeLocation>
    ) -> Result<String, CodeGeneratorError> {
        if let Some(package) = &schema_ref.package {
            // Fully qualified reference
            let qualified_name = format!("{}.{}", package.segments.join("."), schema_ref.name);
            
            if let Some(_location) = type_locations.get(&qualified_name) {
                Ok(schema_ref.name.clone())
            } else {
                Err(CodeGeneratorError::TypeNotFound { name: qualified_name })
            }
        } else {
            // Unqualified reference, need to find in current scope or imports
            self.find_unqualified_type(&schema_ref.name, type_locations)
        }
    }
    
    /// Find an unqualified type in the current scope or imports
    fn find_unqualified_type(
        &self,
        type_name: &str,
        type_locations: &HashMap<String, TypeLocation>
    ) -> Result<String, CodeGeneratorError> {
        // First, check if the type exists as-is (in the current package)
        if type_locations.contains_key(type_name) {
            return Ok(type_name.to_string());
        }
        
        // Then, check all qualified types to see if any match the unqualified name
        for qualified_name in type_locations.keys() {
            if qualified_name.ends_with(&format!(".{type_name}")) {
                return Ok(type_name.to_string());
            }
        }
        
        // If not found, return an error
        Err(CodeGeneratorError::TypeNotFound { name: type_name.to_string() })
    }
    
    /// Convert a primitive type to a Rust type
    fn convert_primitive_to_rust(&self, primitive: &'a PrimitiveType) -> String {
        match primitive {
            PrimitiveType::String => "String".to_string(),
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::I8 => "i8".to_string(),
            PrimitiveType::I16 => "i16".to_string(),
            PrimitiveType::I32 => "i32".to_string(),
            PrimitiveType::I64 => "i64".to_string(),
            PrimitiveType::I128 => "i128".to_string(),
            PrimitiveType::U8 => "u8".to_string(),
            PrimitiveType::U16 => "u16".to_string(),
            PrimitiveType::U32 => "u32".to_string(),
            PrimitiveType::U64 => "u64".to_string(),
            PrimitiveType::U128 => "u128".to_string(),
            PrimitiveType::F32 => "f32".to_string(),
            PrimitiveType::F64 => "f64".to_string(),
        }
    }
}

/// Factory function to create a Rust code generator
///
/// This function creates a new `RustGenerator` instance that implements the `CodeGenerator` trait.
/// It's the recommended way to create a Rust code generator, as it hides the implementation details
/// and returns a trait object that can be used with the Glass toolchain.
///
/// # Parameters
///
/// * `type_tree` - The type tree built from Glass programs
/// * `programs_with_file_names` - A list of Glass programs with their file names
///
/// # Returns
///
/// A trait object that implements the `CodeGenerator` trait with `CodeGeneratorError` as the error type.
///
/// # Examples
///
/// ```rust,no_run
/// use glass_codegen::rust::create_rust_generator;
/// use glass_parser::ast::Program;
/// use glass_parser::type_tree::TypeTree;
///
/// // Create a type tree from Glass programs
/// let programs = vec![/* Glass programs */];
/// let type_tree = TypeTree::from_programs(&programs).unwrap();
///
/// // Create a Rust code generator
/// let programs_with_file_names = vec![(programs.first().unwrap(), "example.glass".to_string())];
/// let generator = create_rust_generator(&type_tree, programs_with_file_names);
/// ```
pub fn create_rust_generator<'a>(
    type_tree: &'a TypeTree,
    programs_with_file_names: Vec<(&'a Program, String)>
) -> impl CodeGenerator<Error = CodeGeneratorError> + 'a {
    RustGenerator::new(type_tree, programs_with_file_names)
}