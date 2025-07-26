use super::errors::TypeTreeError;
use crate::ast::*;
use std::collections::{HashMap, HashSet};

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
    pub nodes: HashMap<String, TypeNode>,
    /// Map from file path to package name (for import resolution)
    pub file_to_package: HashMap<String, String>,
    /// Map from package name to the programs that declare that package
    pub package_to_programs: HashMap<String, Vec<usize>>,
    /// All programs indexed by their position
    pub programs: Vec<ProgramInfo>,
    /// Import graph: maps file paths to their imported file paths
    pub import_graph: HashMap<String, Vec<String>>,
    /// Map from file path to types defined in that file
    pub file_to_types: HashMap<String, Vec<String>>,
}

impl Default for TypeTree {
    fn default() -> Self {
        Self::new()
    }
}
