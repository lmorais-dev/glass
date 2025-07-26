use super::errors::{ImportValidationError, TypeTreeError};
use super::models::TypeTree;
use crate::ast::*;
use std::collections::{HashMap, HashSet};

impl TypeTree {
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

        // First check if we have an import graph at all
        if self.import_graph.is_empty() {
            return None;
        }

        // Check each file that has imports
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
        // Mark as visited and add to recursion stack and path
        visited.insert(file_path.to_string(), true);
        rec_stack.insert(file_path.to_string());
        path.push(file_path.to_string());

        // Get the imports for this file
        if let Some(imports) = self.import_graph.get(file_path) {
            // Skip empty imports list
            if !imports.is_empty() {
                for imported_file in imports {
                    // If the import doesn't exist in our file system, we skip it
                    // This prevents false positives when imports can't be resolved
                    if !self.file_to_package.contains_key(imported_file) {
                        continue;
                    }

                    if !visited.contains_key(imported_file) {
                        // Recursive DFS call for unvisited imports
                        if let Some(cycle) =
                            self.detect_import_cycle_dfs(imported_file, visited, rec_stack, path)
                        {
                            return Some(cycle);
                        }
                    } else if rec_stack.contains(imported_file) {
                        // Found a cycle - construct the cycle path
                        let cycle_start = path.iter().position(|x| x == imported_file).unwrap_or(0); // Fallback to 0 if not found
                        let mut cycle = path[cycle_start..].to_vec();
                        cycle.push(imported_file.clone()); // Complete the cycle
                        return Some(cycle);
                    }
                }
            }
        }

        // Remove from the recursion stack and path before returning
        rec_stack.remove(file_path);
        path.pop();
        None
    }
}
