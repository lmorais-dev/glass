//! Solver for imports, type references, and dependencies.
//!
//! This module provides tools to build type relationship trees
//! which are used to validate type references and dependencies between Glass modules.
//!
//! It will also provide tools for type recursion detection, circular dependencies and
//! provide general data that helps in code generation.

/// Core functionality for type tree construction and basic operations
mod core;
/// Error types for import validation and type tree operations
mod errors;
/// Import-related functionality for resolving and validating imports
mod imports;
/// Data structures for type tree, type nodes, and program information
mod models;
/// Type resolution and validation functionality
mod resolution;
/// Tests for the type tree module
#[cfg(test)]
mod tests;

pub use errors::{ImportValidationError, TypeTreeError};
pub use models::{ProgramInfo, TypeDefinition, TypeNode, TypeTree};
