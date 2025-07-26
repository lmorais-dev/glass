use crate::project::Project;
use crate::{CodeGenerator, GeneratorOutput};
use glass_parser::ast::{
    Definition, EnumDef, PrimitiveType, Program, SchemaDef, SchemaRef, Type, TypeWithSpan,
};
use glass_parser::type_tree::{TypeDefinition, TypeTree};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use syn::{parse_quote, Type as SynType};
use crate::error::CodeGeneratorError;

/// Enhanced type location with TypeTree integration
#[derive(Debug, Clone)]
struct TypeLocation {
    /// Full qualified name (e.g., "com.example.MyType")
    qualified_name: String,
    /// Index into the TypeTree programs
    program_idx: usize,
    /// File name where the type is defined
    file_name: String,
    /// Package segments for module path generation
    package: Option<String>,
    /// Dependencies of this type (from TypeTree)
    dependencies: HashSet<String>,
    /// Rust module path (e.g., "crate::com::example::my_type::MyType")
    rust_path: String,
}

pub struct RustGenerator<'a> {
    type_tree: &'a TypeTree,
    programs_with_file_names: Vec<(&'a Program, String)>,
}

impl<'a> RustGenerator<'a> {
    pub fn new(type_tree: &'a TypeTree, programs_with_file_names: Vec<(&'a Program, String)>) -> Self {
        Self {
            type_tree,
            programs_with_file_names,
        }
    }

    /// Build an enhanced type location map using TypeTree's rich metadata
    fn build_type_location_map(&self) -> HashMap<String, TypeLocation> {
        let mut type_locations = HashMap::new();

        // Use TypeTree's nodes for comprehensive type information
        for (qualified_name, type_node) in &self.type_tree.nodes {
            // Find the program that defines this type
            let program_info = self.find_program_for_type(qualified_name);
            if let Some((program_idx, file_name)) = program_info {
                let package = self.type_tree.programs[program_idx].package_name.clone();
                
                // Generate Rust module path using TypeTree structure
                let rust_path = self.generate_rust_path(qualified_name, &package, &file_name);
                
                type_locations.insert(
                    qualified_name.clone(),
                    TypeLocation {
                        qualified_name: qualified_name.clone(),
                        program_idx,
                        file_name: file_name.clone(),
                        package,
                        dependencies: type_node.dependencies.clone(),
                        rust_path,
                    },
                );
            }
        }

        type_locations
    }

    /// Find which program defines a given type using TypeTree
    fn find_program_for_type(&self, qualified_name: &str) -> Option<(usize, String)> {
        // Use TypeTree's file_to_types mapping for efficient lookup
        for (file_path, types) in &self.type_tree.file_to_types {
            if types.contains(&qualified_name.to_string()) {
                // Find the program index for this file
                for (idx, (_, filename)) in self.programs_with_file_names.iter().enumerate() {
                    if file_path.ends_with(filename) {
                        return Some((idx, filename.clone()));
                    }
                }
            }
        }
        None
    }

    /// Generate an optimized Rust module path using TypeTree package information
    fn generate_rust_path(&self, qualified_name: &str, package: &Option<String>, file_name: &str) -> String {
        let file_stem = file_name.strip_suffix(".glass").unwrap_or(file_name);
        let type_name = qualified_name.split('.').next_back().unwrap_or(qualified_name);
        
        match package {
            Some(pkg) if pkg != "root" => {
                let package_path = pkg.replace('.', "::");
                format!("crate::{package_path}::{file_stem}::{type_name}")
            }
            _ => format!("crate::{file_stem}::{type_name}")
        }
    }

    /// Group programs by package using TypeTree's efficient mapping
    fn group_programs_by_package(&self) -> HashMap<String, Vec<(&'a Program, String)>> {
        let mut packages = HashMap::new();

        // Use TypeTree's package_to_programs for efficient grouping
        for (program, filename) in &self.programs_with_file_names {
            let package_name = program
                .package
                .as_ref()
                .map(|p| p.path.to_string())
                .unwrap_or_else(|| "root".to_string());

            packages
                .entry(package_name)
                .or_insert_with(Vec::new)
                .push((*program, filename.clone()));
        }

        packages
    }

    /// Generate topologically sorted dependencies using TypeTree
    fn get_generation_order(&self, type_locations: &HashMap<String, TypeLocation>) -> Result<Vec<String>, CodeGeneratorError> {
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();
        let mut result = Vec::new();

        // Use TypeTree's dependency graph for proper ordering
        for qualified_name in type_locations.keys() {
            if !visited.contains(qualified_name) {
                Self::topological_sort_visit(
                    qualified_name,
                    type_locations,
                    &mut visited,
                    &mut temp_mark,
                    &mut result,
                )?;
            }
        }

        Ok(result)
    }

    fn topological_sort_visit(
        qualified_name: &str,
        type_locations: &HashMap<String, TypeLocation>,
        visited: &mut HashSet<String>,
        temp_mark: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), CodeGeneratorError> {
        if temp_mark.contains(qualified_name) {
            return Err(CodeGeneratorError::CircularDependency {
                chain: format!("Circular dependency involving {qualified_name}"),
            });
        }

        if visited.contains(qualified_name) {
            return Ok(());
        }

        temp_mark.insert(qualified_name.to_string());

        // Visit dependencies first using TypeTree's dependency information
        if let Some(location) = type_locations.get(qualified_name) {
            for dep in &location.dependencies {
                if type_locations.contains_key(dep) {
                    Self::topological_sort_visit(dep, type_locations, visited, temp_mark, result)?;
                }
            }
        }

        temp_mark.remove(qualified_name);
        visited.insert(qualified_name.to_string());
        result.push(qualified_name.to_string());

        Ok(())
    }

    /// Enhanced type resolution using TypeTree's sophisticated lookup
    fn resolve_schema_reference_path(
        &self,
        schema_ref: &'a SchemaRef,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<String, CodeGeneratorError> {
        let qualified_name = if let Some(package) = &schema_ref.package {
            format!("{}.{}", package, schema_ref.name)
        } else {
            // Use TypeTree's resolution capabilities for unqualified names
            self.resolve_unqualified_type(&schema_ref.name, type_locations)?
        };

        let location = type_locations.get(&qualified_name).ok_or_else(|| {
            CodeGeneratorError::TypeNotFound {
                name: qualified_name.clone(),
            }
        })?;

        Ok(location.rust_path.clone())
    }

    /// Advanced unqualified type resolution using TypeTree
    fn resolve_unqualified_type(
        &self,
        type_name: &str,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<String, CodeGeneratorError> {
        // Direct match first
        if type_locations.contains_key(type_name) {
            return Ok(type_name.to_string());
        }

        // Use TypeTree nodes for comprehensive search
        let candidates: Vec<String> = self.type_tree.nodes
            .keys()
            .filter(|qualified_name| {
                qualified_name.split('.').next_back() == Some(type_name)
            })
            .cloned()
            .collect();

        match candidates.len() {
            0 => Err(CodeGeneratorError::TypeNotFound {
                name: type_name.to_string(),
            }),
            1 => Ok(candidates[0].clone()),
            _ => Err(CodeGeneratorError::AmbiguousTypeReference {
                name: type_name.to_string(),
                candidates,
            }),
        }
    }

    fn generate_lib_rs(
        &self,
        programs_by_package: &HashMap<String, Vec<(&'a Program, String)>>,
    ) -> Result<GeneratorOutput, CodeGeneratorError> {
        let mut package_names: Vec<_> = programs_by_package.keys().collect();
        package_names.sort();

        let mut mod_declarations = Vec::new();

        for package_name in package_names {
            if package_name == "root" {
                let root_programs = programs_by_package.get("root").unwrap();
                self.generate_mod_declarations(&mut mod_declarations, root_programs);
            } else {
                let first_segment = package_name.split('.').next().unwrap();
                let mod_name = format_ident!("{}", first_segment);
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

    fn generate_mod_declarations(
        &self,
        mod_declarations: &mut Vec<TokenStream>,
        program_files: &[(&'a Program, String)],
    ) {
        for (_, filename) in program_files {
            let file_stem = filename.strip_suffix(".glass").unwrap_or(filename);
            let mod_name = format_ident!("{}", file_stem);
            mod_declarations.push(quote! { pub mod #mod_name; });
        }
    }

    fn generate_package_modules(
        &self,
        outputs: &mut Vec<GeneratorOutput>,
        package_name: &str,
        program_files: &[(&'a Program, String)],
        type_locations: &HashMap<String, TypeLocation>,
        _out_dir: &Path,
    ) -> Result<(), CodeGeneratorError> {
        // Handle root package
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

        // Generate package hierarchy
        let package_segments: Vec<&str> = package_name.split('.').collect();
        self.generate_package_hierarchy(outputs, &package_segments, program_files)?;

        // Generate program files
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

    fn generate_package_hierarchy(
        &self,
        outputs: &mut Vec<GeneratorOutput>,
        package_segments: &[&str],
        program_files: &[(&'a Program, String)],
    ) -> Result<(), CodeGeneratorError> {
        for i in 1..=package_segments.len() {
            let segments = &package_segments[..i];
            let module_path = segments.join("/");
            let mod_rs_path = PathBuf::from(format!("src/{module_path}/mod.rs"));

            // Skip if already generated
            if outputs.iter().any(|output| output.path == mod_rs_path) {
                continue;
            }

            let mut mod_declarations = Vec::new();
            if i < package_segments.len() {
                let next_segment = package_segments[i];
                let next_mod_name = format_ident!("{}", next_segment);
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

    fn generate_program_rust_code(
        &self,
        program: &'a Program,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<String, CodeGeneratorError> {
        let mut tokens = Vec::new();

        // Get all types defined in this program
        let program_types: Vec<String> = program.definitions.iter()
            .filter_map(|def| match def {
                Definition::Schema(schema) => Some(self.get_qualified_name(&schema.name, program)),
                Definition::Enum(enum_def) => Some(self.get_qualified_name(&enum_def.name, program)),
                Definition::Service(_) => None, // Handle services separately
            })
            .collect();

        // Filter type_locations to only include types from this program
        let program_type_locations: HashMap<String, TypeLocation> = type_locations
            .iter()
            .filter(|(qualified_name, _)| program_types.contains(qualified_name))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Generate types in dependency order using get_generation_order
        let generation_order = self.get_generation_order(&program_type_locations)?;
        
        for qualified_name in generation_order {
            if let Some(type_node) = self.type_tree.nodes.get(&qualified_name) {
                match &type_node.definition {
                    TypeDefinition::Schema(schema_def) => {
                        let schema_tokens = self.generate_schema_tokens(schema_def, type_locations)?;
                        tokens.push(schema_tokens);
                    }
                    TypeDefinition::Enum(enum_def) => {
                        let enum_tokens = self.generate_enum_tokens(enum_def)?;
                        tokens.push(enum_tokens);
                    }
                }
            }
        }

        let combined = quote! {
            #(#tokens)*
        };

        Ok(combined.to_string())
    }

    fn get_qualified_name(&self, type_name: &str, program: &Program) -> String {
        if let Some(package) = &program.package {
            format!("{}.{type_name}", package.path)
        } else {
            type_name.to_string()
        }
    }

    fn generate_schema_tokens(
        &self,
        schema_def: &'a SchemaDef,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<TokenStream, CodeGeneratorError> {
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

    fn generate_enum_tokens(
        &self,
        enum_def: &'a EnumDef,
    ) -> Result<TokenStream, CodeGeneratorError> {
        let enum_name = format_ident!("{}", enum_def.name);
        let mut variants = Vec::new();

        for variant in &enum_def.variants {
            let variant_name = format_ident!("{}", variant);
            variants.push(quote! { #variant_name });
        }

        Ok(quote! {
            #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
            pub enum #enum_name {
                #(#variants,)*
            }
        })
    }

    fn convert_type_to_syn(
        &self,
        type_with_span: &'a TypeWithSpan,
        type_locations: &HashMap<String, TypeLocation>,
    ) -> Result<SynType, CodeGeneratorError> {
        match &type_with_span.type_value {
            Type::Primitive(primitive) => {
                let type_str = self.convert_primitive_to_rust(primitive);
                let syn_type: SynType = syn::parse_str(&type_str)
                    .map_err(|e| CodeGeneratorError::SynError(e.to_string()))?;
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
                let qualified_path = self.resolve_schema_reference_path(schema_ref, type_locations)?;
                let syn_type: SynType = syn::parse_str(&qualified_path)
                    .map_err(|e| CodeGeneratorError::SynError(e.to_string()))?;
                Ok(syn_type)
            }
        }
    }

    fn convert_primitive_to_rust(&self, primitive: &'a PrimitiveType) -> String {
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

impl<'a> CodeGenerator for RustGenerator<'a> {
    type Error = CodeGeneratorError;

    fn generate(&self, project: &Project) -> Result<Vec<GeneratorOutput>, Self::Error> {
        let _generator_config = project.generator_config.rust.as_ref().ok_or_else(|| {
            CodeGeneratorError::InvalidConfig {
                message: "No Rust generator configuration found".to_string(),
            }
        })?;

        let mut outputs = Vec::new();

        // Build enhanced type location map using TypeTree
        let type_locations = self.build_type_location_map();

        // Group programs efficiently using TypeTree
        let programs_by_package = self.group_programs_by_package();

        // Generate lib.rs
        let lib_rs_output = self.generate_lib_rs(&programs_by_package)?;
        outputs.push(lib_rs_output);

        // Generate package modules in dependency order
        for (package_name, program_files) in &programs_by_package {
            self.generate_package_modules(
                &mut outputs,
                package_name,
                program_files,
                &type_locations,
                &PathBuf::from("src"),
            )?;
        }

        Ok(outputs)
    }

    fn name(&self) -> &'static str {
        "rust-generator-v2"
    }
}

pub fn create_rust_generator<'a>(
    type_tree: &'a TypeTree,
    programs_with_file_names: Vec<(&'a Program, String)>
) -> impl CodeGenerator<Error = CodeGeneratorError> + 'a {
    RustGenerator::new(type_tree, programs_with_file_names)
}