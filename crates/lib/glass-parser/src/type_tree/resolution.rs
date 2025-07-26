use super::errors::TypeTreeError;
use super::models::TypeTree;
use crate::ast::*;
use std::collections::{HashMap, HashSet};

impl TypeTree {
    /// Recursively collects type dependencies from a type, ignoring primitives
    pub fn collect_type_dependencies(
        &self,
        type_with_span: &TypeWithSpan,
        current_program: &Program,
        dependencies: &mut HashSet<String>,
    ) -> Result<(), TypeTreeError> {
        match &type_with_span.type_value {
            Type::Primitive(_) => {
                // Ignore primitives as specified
            }
            Type::Option(inner) => {
                self.collect_type_dependencies(inner, current_program, dependencies)?;
            }
            Type::Vec(inner) => {
                self.collect_type_dependencies(inner, current_program, dependencies)?;
            }
            Type::SchemaRef(schema_ref) => {
                let qualified_name = self.resolve_schema_reference(schema_ref, current_program)?;
                dependencies.insert(qualified_name);
            }
        }
        Ok(())
    }

    /// Resolves a schema reference to its fully qualified name
    pub fn resolve_schema_reference(
        &self,
        schema_ref: &SchemaRef,
        current_program: &Program,
    ) -> Result<String, TypeTreeError> {
        let current_package = current_program
            .package
            .as_ref()
            .map(|p| p.path.to_string())
            .unwrap_or_default();

        // Find the file path for this program
        let current_file = self
            .programs
            .iter()
            .find(|info| std::ptr::eq(&info.program, current_program))
            .and_then(|info| info.file_path.as_ref())
            .cloned();

        // If this is a fully qualified reference (with package)
        let qualified_name = if let Some(package) = &schema_ref.package {
            // Fully qualified reference
            format!("{}.{}", package, schema_ref.name)
        } else {
            // Unqualified reference - need to resolve

            // Keep track of all attempted paths for better error messages
            let mut attempted_paths = Vec::new();

            // 1. Try the local package first
            if !current_package.is_empty() {
                let local_qualified = format!("{}.{}", current_package, schema_ref.name);
                attempted_paths.push(local_qualified.clone());
                if self.nodes.contains_key(&local_qualified) {
                    return Ok(local_qualified);
                }
            }

            // 2. Try without package (root level)
            attempted_paths.push(schema_ref.name.clone());
            if self.nodes.contains_key(&schema_ref.name) {
                return Ok(schema_ref.name.clone());
            }

            // 3. Try packages from imported files using resolved import paths
            if let Some(file_path) = &current_file {
                if let Some(imported_files) = self.import_graph.get(file_path) {
                    for imported_file in imported_files {
                        if let Some(import_package) = self.file_to_package.get(imported_file) {
                            let imported_qualified =
                                format!("{}.{}", import_package, schema_ref.name);
                            attempted_paths.push(imported_qualified.clone());
                            if self.nodes.contains_key(&imported_qualified) {
                                return Ok(imported_qualified);
                            }
                        }
                    }
                }
            } else {
                // Legacy fallback for when we don't have file paths
                for import in &current_program.imports {
                    if let Some(import_package) = self.file_to_package.get(&import.path) {
                        let imported_qualified = format!("{}.{}", import_package, schema_ref.name);
                        attempted_paths.push(imported_qualified.clone());
                        if self.nodes.contains_key(&imported_qualified) {
                            return Ok(imported_qualified);
                        }
                    }
                }
            }

            // If we get here, the type was not found
            return Err(TypeTreeError::TypeNotFound {
                name: format!(
                    "{} (tried: {})",
                    schema_ref.name.clone(),
                    attempted_paths.join(", ")
                ),
            });
        };

        // Verify the type exists
        if !self.nodes.contains_key(&qualified_name) {
            return Err(TypeTreeError::TypeNotFound {
                name: qualified_name.clone(),
            });
        }

        // Check if the type is accessible from the current file
        if let Some(ref file_path) = current_file {
            if !self.is_type_accessible(file_path, &qualified_name) {
                return Err(TypeTreeError::TypeNotAccessible {
                    type_name: qualified_name,
                    from_file: file_path.clone(),
                });
            }
        }

        Ok(qualified_name)
    }

    /// Check if a type is accessible from a given file (via imports or the same package)
    pub fn is_type_accessible(&self, from_file: &str, qualified_type_name: &str) -> bool {
        // Extract package from a qualified type name
        let type_package = self.extract_package_from_qualified_name(qualified_type_name);

        // Check if it's in the same package
        if let Some(from_package) = self.file_to_package.get(from_file) {
            if from_package == &type_package {
                return true;
            }
        }

        // Check if the file containing this type is directly imported
        if let Some(imported_files) = self.import_graph.get(from_file) {
            for imported_file in imported_files {
                if let Some(imported_package) = self.file_to_package.get(imported_file) {
                    if imported_package == &type_package {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Find the file that defines a specific type
    pub fn find_file_for_type(&self, qualified_name: &str) -> Option<String> {
        // Extract the package from the qualified name
        let package_name = self.extract_package_from_qualified_name(qualified_name);

        // Find all files declaring this package
        for (file_path, package) in &self.file_to_package {
            if package == &package_name {
                if let Some(types) = self.file_to_types.get(file_path) {
                    if types.contains(&qualified_name.to_string()) {
                        return Some(file_path.clone());
                    }
                }
            }
        }

        None
    }

    /// Gets all types that would be affected by changes to the given type
    /// This includes the type itself and all types that directly or indirectly depend on it
    pub fn get_all_affected_types(&self, qualified_name: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        let mut to_process = vec![qualified_name.to_string()];
        let mut processed = HashSet::new();

        while let Some(current) = to_process.pop() {
            if !processed.contains(&current) {
                processed.insert(current.clone());
                result.insert(current.clone());

                // Add all dependents to process next
                let dependents = self.get_dependents(&current);
                for dependent in dependents {
                    if !processed.contains(&dependent) {
                        to_process.push(dependent);
                    }
                }
            }
        }

        result
    }

    /// Gets all types that are reachable from a specific file
    /// This includes types in the same package and types from imported packages
    pub fn get_reachable_types(&self, from_file: &str) -> HashSet<String> {
        let mut result = HashSet::new();

        // Get the package for this file
        if let Some(file_package) = self.file_to_package.get(from_file) {
            // Add all types from the same package
            for (name, _node) in &self.nodes {
                let type_package = self.extract_package_from_qualified_name(name);
                if &type_package == file_package {
                    result.insert(name.clone());
                }
            }

            // Add types from imported files
            if let Some(imported_files) = self.import_graph.get(from_file) {
                for imported_file in imported_files {
                    if let Some(imported_package) = self.file_to_package.get(imported_file) {
                        // Add all types from the imported package
                        for (name, _node) in &self.nodes {
                            let type_package = self.extract_package_from_qualified_name(name);
                            if &type_package == imported_package {
                                result.insert(name.clone());
                            }
                        }
                    }
                }
            }
        }

        result
    }

    /// Validates that all type references in a program can be resolved
    pub fn validate_type_references(&self, program: &Program) -> Result<(), Vec<TypeTreeError>> {
        let mut errors = Vec::new();

        // Find the file path for this program
        let file_path = self
            .programs
            .iter()
            .find(|info| std::ptr::eq(&info.program, program))
            .and_then(|info| info.file_path.as_ref())
            .cloned();

        // Get reachable types if we have a file path
        let reachable_types = if let Some(ref path) = file_path {
            self.get_reachable_types(path)
        } else {
            HashSet::new()
        };

        // Check each schema definition
        for definition in &program.definitions {
            if let Definition::Schema(schema) = definition {
                // Check each field for type references
                for field in &schema.fields {
                    if let Err(err) = self.validate_type_reference(
                        &field.field_type,
                        program,
                        &reachable_types,
                        &file_path,
                    ) {
                        errors.push(err);
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validates a single type reference
    pub fn validate_type_reference(
        &self,
        type_with_span: &TypeWithSpan,
        current_program: &Program,
        reachable_types: &HashSet<String>,
        current_file: &Option<String>,
    ) -> Result<(), TypeTreeError> {
        match &type_with_span.type_value {
            Type::Primitive(_) => Ok(()), // Primitives are always valid
            Type::Option(inner) => {
                self.validate_type_reference(inner, current_program, reachable_types, current_file)
            }
            Type::Vec(inner) => {
                self.validate_type_reference(inner, current_program, reachable_types, current_file)
            }
            Type::SchemaRef(schema_ref) => {
                // Try to resolve the schema reference
                let qualified_name =
                    match self.resolve_schema_reference(schema_ref, current_program) {
                        Ok(name) => name,
                        Err(err) => return Err(err),
                    };

                // Check if the type is reachable if we have reachable types
                if !reachable_types.is_empty() && !reachable_types.contains(&qualified_name) {
                    if let Some(file) = current_file {
                        return Err(TypeTreeError::TypeNotAccessible {
                            type_name: qualified_name,
                            from_file: file.clone(),
                        });
                    }
                }

                Ok(())
            }
        }
    }
}
