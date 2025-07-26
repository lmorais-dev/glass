use super::errors::TypeTreeError;
use super::models::{ProgramInfo, TypeDefinition, TypeNode, TypeTree};
use crate::ast::*;
use std::collections::{HashMap, HashSet};

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
        self.import_graph.insert(file_path.clone(), imported_files);

        // Collect types from this program
        self.collect_types_from_program(&program)?;

        // Build dependencies for this program
        self.build_dependencies_from_program(&program)?;

        Ok(())
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
                let info_package = info
                    .program
                    .package
                    .as_ref()
                    .map(|p| p.path.to_string())
                    .unwrap_or_default();
                info_package == package_name
                    && info.program.definitions.len() == program.definitions.len()
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
}
