//! Solver for imports, type references and dependencies.
//!
//! This module provides tools to build type relationship trees
//! which are used to validate type references and dependencies between Glass modules.
//!
//! It will also provide tools for type recursion detection, circular dependencies and
//! provide general data that helps in code generation.
use crate::ast::*;
use std::collections::{HashMap, HashSet};
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

/// Represents a type dependency node in the type tree
#[derive(Debug, Clone, PartialEq)]
pub struct TypeNode {
    /// The fully qualified name of the type (package.name)
    pub qualified_name: String,
    /// The type definition (Schema or Enum)
    pub definition: TypeDefinition,
    /// Set of types this type depends on (excluding primitives)
    pub dependencies: HashSet<String>,
    /// Source span for error reporting
    pub span: Span,
}

/// Type definition that can have dependencies
#[derive(Debug, Clone, PartialEq)]
pub enum TypeDefinition {
    Schema(SchemaDef),
    Enum(EnumDef),
}

/// Information about a Glass program for type resolution
#[derive(Debug, Clone)]
pub struct ProgramInfo {
    /// The program AST
    pub program: Program,
    /// The file path this program was loaded from (if any)
    pub file_path: Option<String>,
    /// The package name this program declares
    pub package_name: Option<String>,
}

/// A complete type tree built from multiple Glass programs
#[derive(Debug, Clone)]
pub struct TypeTree {
    /// Map from qualified type name to type node
    nodes: HashMap<String, TypeNode>,
    /// Map from file path to package name (for import resolution)
    file_to_package: HashMap<String, String>,
    /// Map from package name to the programs that declare that package
    package_to_programs: HashMap<String, Vec<usize>>,
    /// All programs indexed by their position
    programs: Vec<ProgramInfo>,
    /// Import graph: maps file paths to their imported file paths
    import_graph: HashMap<String, Vec<String>>,
    /// Map from file path to types defined in that file
    file_to_types: HashMap<String, Vec<String>>,
}

impl TypeTree {
    /// Adds a program to the type tree with an explicit file path
    pub fn add_program_with_path(
        &mut self,
        program: Program,
        file_path: String,
    ) -> Result<(), TypeTreeError> {
        let package_name = program
            .package
            .as_ref()
            .map(|p| p.path.to_string())
            .unwrap_or_default();

        let program_index = self.programs.len();
        self.programs.push(ProgramInfo {
            program: program.clone(),
            file_path: Some(file_path.clone()),
            package_name: Some(package_name.clone()),
        });

        // Map file path to package name
        self.file_to_package
            .insert(file_path.clone(), package_name.clone());

        // Track which programs declare each package
        self.package_to_programs
            .entry(package_name)
            .or_default()
            .push(program_index);

        // Process imports for the program
        let mut imported_files = Vec::new();
        for import in &program.imports {
            // Resolve the import path relative to the current file
            let resolved_path = self.resolve_import_path(&file_path, &import.path);
            imported_files.push(resolved_path);
        }
        self.import_graph.insert(file_path, imported_files);

        Ok(())
    }

    /// Resolve an import path relative to the importing file path
    pub fn resolve_import_path(&self, importing_file: &str, import_path: &str) -> String {
        if let Some(stripped) = import_path.strip_prefix("/") {
            // Absolute path
            stripped.to_string()
        } else if import_path.starts_with("../") || import_path.starts_with("./") {
            // Relative path - resolve relative to the importing file's directory
            let parent_dir = std::path::Path::new(importing_file)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            if parent_dir.is_empty() {
                import_path.to_string()
            } else {
                format!("{parent_dir}/{import_path}")
            }
        } else {
            // Treat as a path relative to the project root
            import_path.to_string()
        }
    }

    /// Gets all program information with their file paths
    pub fn get_programs_with_paths(&self) -> Vec<(String, &ProgramInfo)> {
        self.programs
            .iter()
            .filter_map(|info| info.file_path.as_ref().map(|path| (path.clone(), info)))
            .collect()
    }

    /// Gets all program information
    pub fn get_all_program_infos(&self) -> &[ProgramInfo] {
        &self.programs
    }

    /// Creates a new empty type tree
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            file_to_package: HashMap::new(),
            package_to_programs: HashMap::new(),
            programs: Vec::new(),
            import_graph: HashMap::new(),
            file_to_types: HashMap::new(),
        }
    }

    /// Builds a type tree from a list of Glass programs with their optional file paths
    pub fn from_programs_with_paths(
        programs_with_paths: &[(Program, Option<String>)],
    ) -> Result<Self, TypeTreeError> {
        let mut tree = Self::new();

        // Store all programs and build file-to-package mapping
        for (program, file_path) in programs_with_paths {
            let program_index = tree.programs.len();

            let package_name = program
                .package
                .as_ref()
                .map(|p| p.path.to_string())
                .unwrap_or_else(|| "".to_string());

            tree.programs.push(ProgramInfo {
                program: program.clone(),
                file_path: file_path.clone(),
                package_name: Some(package_name.clone()),
            });

            // Map file path to package name if a file path is provided
            if let Some(file_path) = file_path {
                tree.file_to_package
                    .insert(file_path.clone(), package_name.clone());

                // Process imports for this file
                let mut imported_files = Vec::new();
                for import in &program.imports {
                    // Resolve the import path relative to the current file
                    let resolved_path = tree.resolve_import_path(file_path, &import.path);
                    imported_files.push(resolved_path);
                }
                tree.import_graph.insert(file_path.clone(), imported_files);
            }

            // Track which programs declare each package
            tree.package_to_programs
                .entry(package_name)
                .or_default()
                .push(program_index);
        }

        // First pass: collect all type definitions
        let programs = tree.programs.clone();
        for program_info in &programs {
            tree.collect_types_from_program(&program_info.program)?;
        }

        // Second pass: build dependency relationships
        for program_info in &programs {
            tree.build_dependencies_from_program(&program_info.program)?;
        }

        // Validate for circular dependencies
        tree.validate_no_cycles()?;

        Ok(tree)
    }

    /// Builds a type tree from a list of Glass programs (without file paths)
    pub fn from_programs(programs: &[Program]) -> Result<Self, TypeTreeError> {
        let programs_with_paths: Vec<_> = programs.iter().map(|p| (p.clone(), None)).collect();
        Self::from_programs_with_paths(&programs_with_paths)
    }

    /// Collects all type definitions from a program
    fn collect_types_from_program(&mut self, program: &Program) -> Result<(), TypeTreeError> {
        let package_name = program
            .package
            .as_ref()
            .map(|p| p.path.to_string())
            .unwrap_or_default();

        // Get the file path for this program by comparing package names
        // This is more reliable than pointer equality
        let file_path = self
            .programs
            .iter()
            .find(|info| {
                let info_package = info.program.package.as_ref().map(|p| p.path.to_string()).unwrap_or_default();
                info_package == package_name && 
                info.program.definitions.len() == program.definitions.len()
            })
            .and_then(|info| info.file_path.clone());

        // Initialize a list of types for this file if we have a file path
        let mut file_types = Vec::new();

        // Collect type definitions
        for definition in &program.definitions {
            match definition {
                Definition::Schema(schema) => {
                    let qualified_name = if package_name.is_empty() {
                        schema.name.clone()
                    } else {
                        format!("{}.{}", package_name, schema.name)
                    };

                    let node = TypeNode {
                        qualified_name: qualified_name.clone(),
                        definition: TypeDefinition::Schema(schema.clone()),
                        dependencies: HashSet::new(), // Will be filled in second pass
                        span: schema.span.clone(),
                    };

                    self.nodes.insert(qualified_name.clone(), node);

                    // Track this type for the file
                    file_types.push(qualified_name);
                }
                Definition::Enum(enum_def) => {
                    let qualified_name = if package_name.is_empty() {
                        enum_def.name.clone()
                    } else {
                        format!("{}.{}", package_name, enum_def.name)
                    };

                    let node = TypeNode {
                        qualified_name: qualified_name.clone(),
                        definition: TypeDefinition::Enum(enum_def.clone()),
                        dependencies: HashSet::new(), // Enums typically don't have dependencies
                        span: enum_def.span.clone(),
                    };

                    self.nodes.insert(qualified_name.clone(), node);

                    // Track this type for the file
                    file_types.push(qualified_name);
                }
                Definition::Service(_) => {
                    // Services are not part of the type tree
                    continue;
                }
            }
        }

        // Store the types defined in this file if we have a file path
        if let Some(path) = file_path {
            self.file_to_types.insert(path, file_types);
        }

        Ok(())
    }

    /// Builds dependency relationships from a program
    fn build_dependencies_from_program(&mut self, program: &Program) -> Result<(), TypeTreeError> {
        let package_name = program
            .package
            .as_ref()
            .map(|p| p.path.to_string())
            .unwrap_or_default();

        for definition in &program.definitions {
            match definition {
                Definition::Schema(schema) => {
                    let qualified_name = if package_name.is_empty() {
                        schema.name.clone()
                    } else {
                        format!("{}.{}", package_name, schema.name)
                    };

                    let mut dependencies = HashSet::new();

                    // Analyze each field for type dependencies
                    for field in &schema.fields {
                        self.collect_type_dependencies(
                            &field.field_type,
                            program,
                            &mut dependencies,
                        )?;
                    }

                    // Update the node with dependencies
                    if let Some(node) = self.nodes.get_mut(&qualified_name) {
                        node.dependencies = dependencies;
                    }
                }
                Definition::Enum(_) => {
                    // Enums don't have type dependencies
                    continue;
                }
                Definition::Service(_) => {
                    // Services are not part of the type tree
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Recursively collects type dependencies from a type, ignoring primitives
    fn collect_type_dependencies(
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
    fn resolve_schema_reference(
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

        let qualified_name = if let Some(package) = &schema_ref.package {
            // Fully qualified reference
            format!("{}.{}", package, schema_ref.name)
        } else {
            // Unqualified reference - need to resolve

            // 1. Try the local package first
            if !current_package.is_empty() {
                let local_qualified = format!("{}.{}", current_package, schema_ref.name);
                if self.nodes.contains_key(&local_qualified) {
                    return Ok(local_qualified);
                }
            }

            // 2. Try without package (root level)
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
                        if self.nodes.contains_key(&imported_qualified) {
                            return Ok(imported_qualified);
                        }
                    }
                }
            }

            return Err(TypeTreeError::TypeNotFound {
                name: schema_ref.name.clone(),
            });
        };

        // Verify the type exists
        if !self.nodes.contains_key(&qualified_name) {
            return Err(TypeTreeError::TypeNotFound {
                name: qualified_name,
            });
        }

        Ok(qualified_name)
    }

    /// Validates that there are no circular dependencies in the type tree
    fn validate_no_cycles(&self) -> Result<(), TypeTreeError> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for qualified_name in self.nodes.keys() {
            if !visited.contains(qualified_name) {
                self.detect_cycle_dfs(qualified_name, &mut visited, &mut rec_stack, &mut path)?;
            }
        }

        Ok(())
    }

    /// Depth-first search to detect cycles
    fn detect_cycle_dfs(
        &self,
        node_name: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Result<(), TypeTreeError> {
        visited.insert(node_name.to_string());
        rec_stack.insert(node_name.to_string());
        path.push(node_name.to_string());

        if let Some(node) = self.nodes.get(node_name) {
            for dependency in &node.dependencies {
                if !visited.contains(dependency) {
                    self.detect_cycle_dfs(dependency, visited, rec_stack, path)?;
                } else if rec_stack.contains(dependency) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|x| x == dependency).unwrap();
                    let cycle_path = path[cycle_start..].join(" -> ");
                    return Err(TypeTreeError::CircularDependency {
                        path: format!("{cycle_path} -> {dependency}"),
                    });
                }
            }
        }

        rec_stack.remove(node_name);
        path.pop();
        Ok(())
    }

    /// Gets a type node by its qualified name
    pub fn get_type(&self, qualified_name: &str) -> Option<&TypeNode> {
        self.nodes.get(qualified_name)
    }

    /// Gets all type nodes
    pub fn get_all_types(&self) -> &HashMap<String, TypeNode> {
        &self.nodes
    }

    /// Gets the dependencies of a specific type
    pub fn get_dependencies(&self, qualified_name: &str) -> Option<&HashSet<String>> {
        self.nodes
            .get(qualified_name)
            .map(|node| &node.dependencies)
    }

    /// Gets all types that depend on the given type
    pub fn get_dependents(&self, qualified_name: &str) -> Vec<String> {
        self.nodes
            .iter()
            .filter_map(|(name, node)| {
                if node.dependencies.contains(qualified_name) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Gets the package name for a given file path
    pub fn get_package_for_file(&self, file_path: &str) -> Option<&String> {
        self.file_to_package.get(file_path)
    }

    /// Gets all programs that declare a specific package
    pub fn get_programs_for_package(&self, package_name: &str) -> Vec<&Program> {
        if let Some(program_indices) = self.package_to_programs.get(package_name) {
            program_indices
                .iter()
                .filter_map(|&index| self.programs.get(index).map(|info| &info.program))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Extract the package name from a qualified type name
    pub fn extract_package_from_qualified_name(&self, qualified_name: &str) -> String {
        qualified_name
            .rsplit_once('.')
            .map(|(package, _)| package.to_string())
            .unwrap_or_default()
    }

    /// Check if a given type exists in the type tree
    pub fn has_type(&self, qualified_name: &str) -> bool {
        self.nodes.contains_key(qualified_name)
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

    /// Find file that declares a specific package
    pub fn find_file_for_package(&self, package_name: &str) -> Option<String> {
        self.file_to_package
            .iter()
            .find(|(_, pkg)| *pkg == package_name)
            .map(|(file, _)| file.clone())
    }

    /// Validate all imports can be resolved
    pub fn validate_imports(&self) -> Result<(), Vec<ImportValidationError>> {
        let mut errors = Vec::new();

        // First check for circular imports - if found, return just that error
        if let Some(cycle) = self.detect_circular_imports() {
            return Err(vec![ImportValidationError::CircularImport { cycle }]);
        }

        // If no circular imports, check for missing imports
        for (file_path, imported_files) in &self.import_graph {
            for imported_file in imported_files {
                // Check if imported file exists in our program set
                if !self.file_to_package.contains_key(imported_file) {
                    // Find the original import statement that caused this
                    let original_import = self.find_original_import(file_path, imported_file);

                    errors.push(ImportValidationError::FileNotFound {
                        import_path: original_import.unwrap_or_else(|| imported_file.clone()),
                        from_file: file_path.clone(),
                        resolved_path: imported_file.clone(),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Find the original import statement path that resulted in a resolved path
    fn find_original_import(&self, file_path: &str, resolved_path: &str) -> Option<String> {
        // Find the program with this file path
        for info in &self.programs {
            if info.file_path.as_deref() == Some(file_path) {
                // Find the import statement that would resolve to this path
                for import in &info.program.imports {
                    let test_resolved = self.resolve_import_path(file_path, &import.path);
                    if test_resolved == resolved_path {
                        return Some(import.path.clone());
                    }
                }
            }
        }
        None
    }

    /// Detect circular imports in the import graph
    fn detect_circular_imports(&self) -> Option<Vec<String>> {
        let mut visited = HashMap::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for file_path in self.import_graph.keys() {
            if !visited.contains_key(file_path) {
                if let Some(cycle) =
                    self.detect_import_cycle_dfs(file_path, &mut visited, &mut rec_stack, &mut path)
                {
                    // Make sure the cycle is complete by adding the first element at the end if needed
                    if cycle.first() != cycle.last() {
                        let mut complete_cycle = cycle.clone();
                        if let Some(first) = cycle.first() {
                            complete_cycle.push(first.clone());
                        }
                        return Some(complete_cycle);
                    }
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// DFS-based cycle detection for imports
    fn detect_import_cycle_dfs(
        &self,
        file_path: &str,
        visited: &mut HashMap<String, bool>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(file_path.to_string(), true);
        rec_stack.insert(file_path.to_string());
        path.push(file_path.to_string());

        if let Some(imports) = self.import_graph.get(file_path) {
            for imported_file in imports {
                if !visited.contains_key(imported_file) {
                    if let Some(cycle) =
                        self.detect_import_cycle_dfs(imported_file, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(imported_file) {
                    // Found a cycle
                    let cycle_start = path.iter().position(|x| x == imported_file).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(imported_file.clone()); // Complete the cycle
                    return Some(cycle);
                }
            }
        }

        rec_stack.remove(file_path);
        path.pop();
        None
    }

    /// Check if a type is accessible from a given file (via imports or same package)
    pub fn is_type_accessible(&self, from_file: &str, qualified_type_name: &str) -> bool {
        // Extract package from qualified type name
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
}

impl Default for TypeTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{
        Definition, EnumDef, ImportStmt, PackageDecl, PackagePath, Program, SchemaDef, SchemaField,
        SchemaRef, Span, Type, TypeWithSpan,
    };
    use crate::parser::Parser;

    // Helper function to create a simple program with a schema
    fn create_test_program(
        package_name: &str,
        schema_name: &str,
        fields: Vec<(&str, Type)>,
    ) -> Program {
        let package = Some(PackageDecl {
            path: PackagePath {
                segments: package_name.split('.').map(String::from).collect(),
                span: Span::dummy(),
            },
            span: Span::dummy(),
        });

        let schema_fields = fields
            .into_iter()
            .map(|(name, type_value)| SchemaField {
                name: name.to_string(),
                field_type: TypeWithSpan {
                    type_value,
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            })
            .collect();

        let schema_def = SchemaDef {
            name: schema_name.to_string(),
            fields: schema_fields,
            span: Span::dummy(),
        };

        Program {
            package,
            imports: vec![],
            definitions: vec![Definition::Schema(schema_def)],
            span: Span::dummy(),
        }
    }

    // Helper function to create a simple program with an enum
    fn create_test_enum_program(
        package_name: &str,
        enum_name: &str,
        variants: Vec<&str>,
    ) -> Program {
        let package = Some(PackageDecl {
            path: PackagePath {
                segments: package_name.split('.').map(String::from).collect(),
                span: Span::dummy(),
            },
            span: Span::dummy(),
        });

        let enum_def = EnumDef {
            name: enum_name.to_string(),
            variants: variants.into_iter().map(String::from).collect(),
            span: Span::dummy(),
        };

        Program {
            package,
            imports: vec![],
            definitions: vec![Definition::Enum(enum_def)],
            span: Span::dummy(),
        }
    }

    #[test]
    fn test_empty_type_tree() {
        let tree = TypeTree::new();
        assert!(tree.get_all_types().is_empty());
    }

    #[test]
    fn test_file_to_package_mapping() {
        let tree = TypeTree::new();
        assert_eq!(tree.get_package_for_file("some/file.glass"), None);
    }

    #[test]
    fn test_from_programs_single_schema() {
        // Create a program with a single schema
        let program = create_test_program(
            "com.example",
            "User",
            vec![
                ("id", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("name", Type::Primitive(crate::ast::PrimitiveType::String)),
            ],
        );

        // Create a type tree from the program
        let result = TypeTree::from_programs(&[program]);
        assert!(result.is_ok());

        let tree = result.unwrap();

        // Check that the type tree contains the expected type
        let qualified_name = "com.example.User";
        let type_node = tree.get_type(qualified_name);
        assert!(type_node.is_some());

        // Check the type node properties
        let type_node = type_node.unwrap();
        match &type_node.definition {
            TypeDefinition::Schema(schema_def) => {
                assert_eq!(schema_def.name, "User");
                assert_eq!(schema_def.fields.len(), 2);
                assert_eq!(schema_def.fields[0].name, "id");
                assert_eq!(schema_def.fields[1].name, "name");
            }
            _ => panic!("Expected schema definition"),
        }

        // Check that dependencies are empty (primitive types don't count as dependencies)
        assert!(type_node.dependencies.is_empty());
    }

    #[test]
    fn test_from_programs_with_dependencies() {
        // Create a program with two schemas where one references the other
        let address_program = create_test_program(
            "com.example",
            "Address",
            vec![
                ("street", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("city", Type::Primitive(crate::ast::PrimitiveType::String)),
            ],
        );

        // Create a schema that references Address
        let address_ref = SchemaRef {
            package: Some(PackagePath {
                segments: vec!["com".to_string(), "example".to_string()],
                span: Span::dummy(),
            }),
            name: "Address".to_string(),
            span: Span::dummy(),
        };

        let user_program = create_test_program(
            "com.example",
            "User",
            vec![
                ("id", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("name", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("address", Type::SchemaRef(address_ref)),
            ],
        );

        // Create a type tree from both programs
        let result = TypeTree::from_programs(&[address_program, user_program]);
        assert!(result.is_ok());

        let tree = result.unwrap();

        // Check that both types exist
        let address_name = "com.example.Address";
        let user_name = "com.example.User";

        assert!(tree.get_type(address_name).is_some());
        assert!(tree.get_type(user_name).is_some());

        // Check that User depends on Address
        let user_deps = tree.get_dependencies(user_name).unwrap();
        assert!(user_deps.contains(address_name));

        // Check that Address has no dependencies
        let address_deps = tree.get_dependencies(address_name).unwrap();
        assert!(address_deps.is_empty());

        // Check that Address is a dependent of User
        let address_dependents = tree.get_dependents(address_name);
        assert_eq!(address_dependents.len(), 1);
        assert!(address_dependents.contains(&user_name.to_string()));
    }

    #[test]
    fn test_from_programs_with_enum() {
        // Create a program with an enum
        let program =
            create_test_enum_program("com.example", "Status", vec!["OK", "ERROR", "PENDING"]);

        // Create a type tree from the program
        let result = TypeTree::from_programs(&[program]);
        assert!(result.is_ok());

        let tree = result.unwrap();

        // Check that the type tree contains the expected type
        let qualified_name = "com.example.Status";
        let type_node = tree.get_type(qualified_name);
        assert!(type_node.is_some());

        // Check the type node properties
        let type_node = type_node.unwrap();
        match &type_node.definition {
            TypeDefinition::Enum(enum_def) => {
                assert_eq!(enum_def.name, "Status");
                assert_eq!(enum_def.variants.len(), 3);
                assert_eq!(enum_def.variants[0], "OK");
                assert_eq!(enum_def.variants[1], "ERROR");
                assert_eq!(enum_def.variants[2], "PENDING");
            }
            _ => panic!("Expected enum definition"),
        }

        // Check that dependencies are empty
        assert!(type_node.dependencies.is_empty());
    }

    #[test]
    fn test_circular_dependency_detection() {
        // Create programs with circular dependencies
        let schema_a_ref = SchemaRef {
            package: Some(PackagePath {
                segments: vec!["com".to_string(), "example".to_string()],
                span: Span::dummy(),
            }),
            name: "SchemaB".to_string(),
            span: Span::dummy(),
        };

        let schema_a = create_test_program(
            "com.example",
            "SchemaA",
            vec![
                ("id", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("ref_to_b", Type::SchemaRef(schema_a_ref)),
            ],
        );

        let schema_b_ref = SchemaRef {
            package: Some(PackagePath {
                segments: vec!["com".to_string(), "example".to_string()],
                span: Span::dummy(),
            }),
            name: "SchemaA".to_string(),
            span: Span::dummy(),
        };

        let schema_b = create_test_program(
            "com.example",
            "SchemaB",
            vec![
                ("id", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("ref_to_a", Type::SchemaRef(schema_b_ref)),
            ],
        );

        // Create a type tree from both programs
        let result = TypeTree::from_programs(&[schema_a, schema_b]);

        // Should detect circular dependency
        assert!(result.is_err());

        match result {
            Err(TypeTreeError::CircularDependency { path }) => {
                // The exact path might vary depending on which node is visited first
                assert!(path.contains("com.example.SchemaA"));
                assert!(path.contains("com.example.SchemaB"));
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_missing_type_reference() {
        // Create a program that references a non-existent type
        let missing_ref = SchemaRef {
            package: Some(PackagePath {
                segments: vec!["com".to_string(), "example".to_string()],
                span: Span::dummy(),
            }),
            name: "NonExistent".to_string(),
            span: Span::dummy(),
        };

        let program = create_test_program(
            "com.example",
            "User",
            vec![
                ("id", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("missing", Type::SchemaRef(missing_ref)),
            ],
        );

        // Create a type tree from the program
        let result = TypeTree::from_programs(&[program]);

        // Should detect missing type
        assert!(result.is_err());

        match result {
            Err(TypeTreeError::TypeNotFound { name }) => {
                assert_eq!(name, "com.example.NonExistent");
            }
            _ => panic!("Expected TypeNotFound error"),
        }
    }

    #[test]
    fn test_complex_type_references() {
        // Test with Option and Vec types
        let address_program = create_test_program(
            "com.example",
            "Address",
            vec![
                ("street", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("city", Type::Primitive(crate::ast::PrimitiveType::String)),
            ],
        );

        // Create a schema reference for Address
        let address_ref = SchemaRef {
            package: Some(PackagePath {
                segments: vec!["com".to_string(), "example".to_string()],
                span: Span::dummy(),
            }),
            name: "Address".to_string(),
            span: Span::dummy(),
        };

        // Create Option<Address> type
        let option_address = Type::Option(Box::new(TypeWithSpan {
            type_value: Type::SchemaRef(address_ref.clone()),
            span: Span::dummy(),
        }));

        // Create Vec<Address> type
        let vec_address = Type::Vec(Box::new(TypeWithSpan {
            type_value: Type::SchemaRef(address_ref),
            span: Span::dummy(),
        }));

        let user_program = create_test_program(
            "com.example",
            "User",
            vec![
                ("id", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("name", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("optional_address", option_address),
                ("addresses", vec_address),
            ],
        );

        // Create a type tree from both programs
        let result = TypeTree::from_programs(&[address_program, user_program]);
        assert!(result.is_ok());

        let tree = result.unwrap();

        // Check that both types exist
        let address_name = "com.example.Address";
        let user_name = "com.example.User";

        assert!(tree.get_type(address_name).is_some());
        assert!(tree.get_type(user_name).is_some());

        // Check that User depends on Address (even through Option and Vec)
        let user_deps = tree.get_dependencies(user_name).unwrap();
        assert!(user_deps.contains(address_name));
    }

    #[test]
    fn test_import_validation() {
        // Create a program with a missing import
        let program_with_missing_import = Program {
            package: Some(PackageDecl {
                path: PackagePath {
                    segments: vec!["com".to_string(), "example".to_string()],
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            }),
            imports: vec![ImportStmt {
                path: "nonexistent/file.glass".to_string(),
                span: Span::dummy(),
            }],
            definitions: vec![],
            span: Span::dummy(),
        };

        // Create a type tree with the program
        let mut tree = TypeTree::new();
        tree.add_program_with_path(program_with_missing_import, "main.glass".to_string())
            .unwrap();

        // Validate imports - should fail with an error
        let validation_result = tree.validate_imports();
        assert!(validation_result.is_err());

        let errors = validation_result.unwrap_err();
        assert_eq!(errors.len(), 1);

        match &errors[0] {
            ImportValidationError::FileNotFound {
                import_path,
                from_file,
                ..
            } => {
                assert_eq!(import_path, "nonexistent/file.glass");
                assert_eq!(from_file, "main.glass");
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_file_to_types_mapping() {
        // Create a program with two types
        let program = Program {
            package: Some(PackageDecl {
                path: PackagePath {
                    segments: vec!["com".to_string(), "example".to_string()],
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            }),
            imports: vec![],
            definitions: vec![
                Definition::Schema(SchemaDef {
                    name: "User".to_string(),
                    fields: vec![SchemaField {
                        name: "name".to_string(),
                        field_type: TypeWithSpan {
                            type_value: Type::Primitive(crate::ast::PrimitiveType::String),
                            span: Span::dummy(),
                        },
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                }),
                Definition::Enum(EnumDef {
                    name: "Status".to_string(),
                    variants: vec!["OK".to_string(), "ERROR".to_string()],
                    span: Span::dummy(),
                }),
            ],
            span: Span::dummy(),
        };

        // Create a type tree with the program
        let programs_with_paths = vec![(program, Some("models.glass".to_string()))];

        let tree = TypeTree::from_programs_with_paths(&programs_with_paths).unwrap();

        // Check that the file_to_types mapping was built correctly
        assert!(tree.file_to_types.contains_key("models.glass"));
        let types = tree.file_to_types.get("models.glass").unwrap();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&"com.example.User".to_string()));
        assert!(types.contains(&"com.example.Status".to_string()));

        // Test find_file_for_type
        assert_eq!(
            tree.find_file_for_type("com.example.User"),
            Some("models.glass".to_string())
        );
        assert_eq!(
            tree.find_file_for_type("com.example.Status"),
            Some("models.glass".to_string())
        );
        assert_eq!(tree.find_file_for_type("com.example.NonExistent"), None);
    }

    #[test]
    fn test_circular_import_detection() {
        // Create programs with circular imports
        let program_a = Program {
            package: Some(PackageDecl {
                path: PackagePath {
                    segments: vec!["com".to_string(), "example".to_string(), "a".to_string()],
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            }),
            imports: vec![ImportStmt {
                path: "b/b.glass".to_string(), // Use absolute path instead of relative
                span: Span::dummy(),
            }],
            definitions: vec![],
            span: Span::dummy(),
        };

        let program_b = Program {
            package: Some(PackageDecl {
                path: PackagePath {
                    segments: vec!["com".to_string(), "example".to_string(), "b".to_string()],
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            }),
            imports: vec![ImportStmt {
                path: "a/a.glass".to_string(), // Use absolute path instead of relative
                span: Span::dummy(),
            }],
            definitions: vec![],
            span: Span::dummy(),
        };

        // Create a type tree with the programs
        let mut tree = TypeTree::new();
        tree.add_program_with_path(program_a, "a/a.glass".to_string())
            .unwrap();
        tree.add_program_with_path(program_b, "b/b.glass".to_string())
            .unwrap();

        // Validate imports - should detect circular dependency
        let validation_result = tree.validate_imports();
        assert!(validation_result.is_err());

        let errors = validation_result.unwrap_err();
        assert_eq!(errors.len(), 1);

        match &errors[0] {
            ImportValidationError::CircularImport { cycle } => {
                assert_eq!(cycle.len(), 3); // a -> b -> a
                assert!(cycle.contains(&"a/a.glass".to_string()));
                assert!(cycle.contains(&"b/b.glass".to_string()));
            }
            _ => panic!("Expected CircularImport error"),
        }
    }

    #[test]
    fn test_import_resolution() {
        // Create a program with a schema in commons package
        let commons_program = Program {
            package: Some(PackageDecl {
                path: PackagePath {
                    segments: vec![
                        "com".to_string(),
                        "example".to_string(),
                        "commons".to_string(),
                    ],
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            }),
            imports: vec![],
            definitions: vec![Definition::Schema(SchemaDef {
                name: "PodcastInfo".to_string(),
                fields: vec![SchemaField {
                    name: "name".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::Primitive(crate::ast::PrimitiveType::String),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        };

        // Create a program that imports the commons package
        let user_program = Program {
            package: Some(PackageDecl {
                path: PackagePath {
                    segments: vec!["com".to_string(), "example".to_string(), "user".to_string()],
                    span: Span::dummy(),
                },
                span: Span::dummy(),
            }),
            imports: vec![ImportStmt {
                path: "commons/podcast.glass".to_string(),
                span: Span::dummy(),
            }],
            definitions: vec![Definition::Schema(SchemaDef {
                name: "User".to_string(),
                fields: vec![SchemaField {
                    name: "podcast".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::SchemaRef(SchemaRef {
                            package: Some(PackagePath {
                                segments: vec![
                                    "com".to_string(),
                                    "example".to_string(),
                                    "commons".to_string(),
                                ],
                                span: Span::dummy(),
                            }),
                            name: "PodcastInfo".to_string(),
                            span: Span::dummy(),
                        }),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        };

        // Create a type tree with proper file paths
        let programs_with_paths = vec![
            (commons_program, Some("commons/podcast.glass".to_string())),
            (user_program, Some("user/user.glass".to_string())),
        ];

        let tree = TypeTree::from_programs_with_paths(&programs_with_paths).unwrap();

        // Check that the import graph was built correctly
        assert!(tree.import_graph.contains_key("user/user.glass"));
        let imports = tree.import_graph.get("user/user.glass").unwrap();
        assert_eq!(imports, &vec!["commons/podcast.glass".to_string()]);

        // Check that types were collected correctly
        assert!(tree.has_type("com.example.commons.PodcastInfo"));
        assert!(tree.has_type("com.example.user.User"));

        // Check type accessibility
        assert!(
            tree.is_type_accessible("commons/podcast.glass", "com.example.commons.PodcastInfo")
        );
        assert!(tree.is_type_accessible("user/user.glass", "com.example.commons.PodcastInfo")); // Should be accessible via import
        assert!(!tree.is_type_accessible("commons/podcast.glass", "com.example.user.User")); // Should not be accessible without import
    }

    #[test]
    fn test_get_programs_for_package() {
        // Create two programs in the same package
        let program1 = create_test_program(
            "com.example",
            "User",
            vec![
                ("id", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("name", Type::Primitive(crate::ast::PrimitiveType::String)),
            ],
        );

        let program2 =
            create_test_enum_program("com.example", "Status", vec!["OK", "ERROR", "PENDING"]);

        // Create a program in a different package
        let program3 = create_test_program(
            "com.other",
            "Product",
            vec![
                ("id", Type::Primitive(crate::ast::PrimitiveType::String)),
                ("name", Type::Primitive(crate::ast::PrimitiveType::String)),
            ],
        );

        // Create a type tree from all programs
        let programs_with_paths = vec![
            (program1, Some("file1.glass".to_string())),
            (program2, Some("file2.glass".to_string())),
            (program3, Some("file3.glass".to_string())),
        ];

        let result = TypeTree::from_programs_with_paths(&programs_with_paths);
        assert!(result.is_ok());

        let tree = result.unwrap();

        // Check programs for com.example package
        let example_programs = tree.get_programs_for_package("com.example");
        assert_eq!(example_programs.len(), 2);

        // Check programs for com.other package
        let other_programs = tree.get_programs_for_package("com.other");
        assert_eq!(other_programs.len(), 1);

        // Check a file to package mapping
        assert_eq!(
            tree.get_package_for_file("file1.glass"),
            Some(&"com.example".to_string())
        );
        assert_eq!(
            tree.get_package_for_file("file2.glass"),
            Some(&"com.example".to_string())
        );
        assert_eq!(
            tree.get_package_for_file("file3.glass"),
            Some(&"com.other".to_string())
        );
        assert_eq!(tree.get_package_for_file("nonexistent.glass"), None);
    }

    #[test]
    fn test_end_to_end_import_resolution() {
        // Create Glass source files
        let commons_source = r#"package com.example.commons;

schema PodcastInfo {
    id: string;
    name: string;
    description: string;
}
"#;

        let user_source = r#"package com.example.user;

import "commons/podcast.glass";

schema User {
    id: string;
    name: string;
    favorite_podcast: com.example.commons.PodcastInfo;
}
"#;

        // Parse the source files
        let commons_program = Parser::parse(commons_source.to_string()).unwrap();
        let user_program = Parser::parse(user_source.to_string()).unwrap();

        // Create a type tree with the programs and their file paths
        let programs_with_paths = vec![
            (commons_program, Some("commons/podcast.glass".to_string())),
            (user_program, Some("user/user.glass".to_string())),
        ];

        let tree = TypeTree::from_programs_with_paths(&programs_with_paths).unwrap();

        // Verify the import graph
        assert!(tree.import_graph.contains_key("user/user.glass"));
        let imported_files = tree.import_graph.get("user/user.glass").unwrap();
        assert_eq!(imported_files, &vec!["commons/podcast.glass".to_string()]);

        // Verify type resolution
        assert!(tree.has_type("com.example.commons.PodcastInfo"));
        assert!(tree.has_type("com.example.user.User"));

        // Verify dependencies
        let user_deps = tree.get_dependencies("com.example.user.User").unwrap();
        assert!(user_deps.contains("com.example.commons.PodcastInfo"));

        // Verify type accessibility
        assert!(
            tree.is_type_accessible("commons/podcast.glass", "com.example.commons.PodcastInfo")
        );
        assert!(tree.is_type_accessible("user/user.glass", "com.example.commons.PodcastInfo"));
        assert!(!tree.is_type_accessible("commons/podcast.glass", "com.example.user.User"));
    }

    #[test]
    fn test_unqualified_type_resolution() {
        // Create Glass source files with unqualified references
        let commons_source = r#"package com.example.commons;

schema PodcastInfo {
    id: string;
    name: string;
    description: string;
}
"#;

        let user_source = r#"package com.example.user;

import "commons/podcast.glass";

schema User {
    id: string;
    name: string;
    favorite_podcast: PodcastInfo;  // Unqualified reference
}
"#;

        // Parse the source files
        let commons_program = Parser::parse(commons_source.to_string()).unwrap();
        let user_program = Parser::parse(user_source.to_string()).unwrap();

        // Create a type tree with the programs and their file paths
        let programs_with_paths = vec![
            (commons_program, Some("commons/podcast.glass".to_string())),
            (user_program, Some("user/user.glass".to_string())),
        ];

        let tree = TypeTree::from_programs_with_paths(&programs_with_paths).unwrap();

        // Verify that the unqualified reference was resolved correctly
        let user_deps = tree.get_dependencies("com.example.user.User").unwrap();
        assert!(user_deps.contains("com.example.commons.PodcastInfo"));
    }

    #[test]
    fn test_circular_import_detection_2() {
        // Create Glass source files with circular imports
        let a_source = r#"package com.example.a;

import "b/b.glass";

schema A {
    id: string;
    b_ref: com.example.b.B;
}
"#;

        let b_source = r#"package com.example.b;

import "a/a.glass";

schema B {
    id: string;
    a_ref: com.example.a.A;
}
"#;

        // Parse the source files
        let a_program = Parser::parse(a_source.to_string()).unwrap();
        let b_program = Parser::parse(b_source.to_string()).unwrap();

        // Create a type tree with the programs and their file paths
        let programs_with_paths = vec![
            (a_program, Some("a/a.glass".to_string())),
            (b_program, Some("b/b.glass".to_string())),
        ];

        // This should fail with a circular dependency error
        let result = TypeTree::from_programs_with_paths(&programs_with_paths);
        assert!(result.is_err());

        match result {
            Err(TypeTreeError::CircularDependency { path }) => {
                assert!(path.contains("com.example.a.A"));
                assert!(path.contains("com.example.b.B"));
            }
            _ => panic!("Expected CircularDependency error"),
        }
    }

    #[test]
    fn test_missing_import_validation() {
        // Create a program with a reference to a non-existent import
        let source = r#"package com.example.main;

import "nonexistent.glass";

schema Main {
    id: string;
}
"#;

        // Parse the source
        let program = Parser::parse(source.to_string()).unwrap();

        // Create a type tree with the program
        let mut tree = TypeTree::new();
        tree.add_program_with_path(program, "main.glass".to_string())
            .unwrap();

        // Validate imports - should fail
        let validation_result = tree.validate_imports();
        assert!(validation_result.is_err());

        let errors = validation_result.unwrap_err();
        assert_eq!(errors.len(), 1);

        match &errors[0] {
            ImportValidationError::FileNotFound {
                import_path,
                from_file,
                ..
            } => {
                assert_eq!(import_path, "nonexistent.glass");
                assert_eq!(from_file, "main.glass");
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }
}
