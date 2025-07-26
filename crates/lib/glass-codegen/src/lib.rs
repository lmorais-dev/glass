use std::path::PathBuf;

pub mod rust;
mod error;

#[derive(Clone)]
pub struct GeneratorOutput {
    pub path: PathBuf,
    pub content: String,
}

/// Trait defining the interface for code generators
///
/// This trait provides a common interface for all code generators in the Glass system.
/// Implementing this trait allows a generator to be used with the Glass toolchain.
///
/// # Examples
///
/// ```rust,no_run
/// use glass_codegen::CodeGenerator;
/// use glass_codegen::project::Project;
/// use glass_codegen::GeneratorOutput;
/// use std::error::Error;
/// use std::fmt;
///
/// // Define a custom error type that implements std::error::Error
/// #[derive(Debug)]
/// struct MyError(String);
///
/// impl fmt::Display for MyError {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "{}", self.0)
///     }
/// }
///
/// impl Error for MyError {}
///
/// struct MyGenerator;
///
/// impl CodeGenerator for MyGenerator {
///     type Error = MyError;
///
///     fn generate(&self) -> Result<Vec<GeneratorOutput>, Self::Error> {
///         // Generate code here
///         Ok(vec![])
///     }
///
///     fn name(&self) -> &'static str {
///         "my-generator"
///     }
/// }
/// ```
pub trait CodeGenerator {
    /// The error type returned by this generator
    ///
    /// This associated type allows each generator to define its own error type,
    /// which must implement the `std::error::Error` trait.
    type Error: std::error::Error;

    /// Generate code from the given programs and project configuration
    ///
    /// This method is the main entry point for code generation. It takes a project
    /// configuration and returns a list of generated files.
    ///
    /// # Parameters
    ///
    /// * `project` - The project configuration
    ///
    /// # Returns
    ///
    /// A list of generated files or an error if code generation fails.
    fn generate(&self) -> Result<Vec<GeneratorOutput>, Self::Error>;

    /// Get the name of this generator
    ///
    /// This method returns a unique name for the generator, which can be used
    /// to identify it in the Glass toolchain.
    fn name(&self) -> &'static str;
}