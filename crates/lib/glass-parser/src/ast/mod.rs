use std::fmt;

/// Represents a span (location) in the source code
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Start position (line, column) in the source code
    pub start: (usize, usize),
    /// End position (line, column) in the source code
    pub end: (usize, usize),
}

impl Span {
    /// Creates a new span with the given start and end positions
    pub fn new(start: (usize, usize), end: (usize, usize)) -> Self {
        Self { start, end }
    }

    /// Creates a dummy span (0,0)-(0,0)
    pub fn dummy() -> Self {
        Self::new((0, 0), (0, 0))
    }

    /// Creates a span from a pest span
    pub fn from_pest(span: pest::Span<'_>) -> Self {
        let start_pos = span.start_pos().line_col();
        let end_pos = span.end_pos().line_col();
        Self::new(start_pos, end_pos)
    }
}

/// The root node of the AST representing a complete Glass program
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Optional package declaration
    pub package: Option<PackageDecl>,
    /// List of import statements
    pub imports: Vec<ImportStmt>,
    /// List of definitions (services, schemas, enums)
    pub definitions: Vec<Definition>,
    /// Source code span
    pub span: Span,
}

/// Package declaration defining the namespace for the current file
#[derive(Debug, Clone, PartialEq)]
pub struct PackageDecl {
    /// Package path (e.g., "com.example.project")
    pub path: PackagePath,
    /// Source code span
    pub span: Span,
}

/// Package path consisting of identifiers separated by dots
#[derive(Debug, Clone, PartialEq)]
pub struct PackagePath {
    /// List of identifiers in the package path
    pub segments: Vec<String>,
    /// Source code span
    pub span: Span,
}

impl fmt::Display for PackagePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.segments.join("."))
    }
}

/// Import statement for importing other Glass files
#[derive(Debug, Clone, PartialEq)]
pub struct ImportStmt {
    /// Path to the imported file
    pub path: String,
    /// Source code span
    pub span: Span,
}

/// Definition of a service, schema, or enum
#[derive(Debug, Clone, PartialEq)]
pub enum Definition {
    /// Service definition
    Service(ServiceDef),
    /// Schema definition
    Schema(SchemaDef),
    /// Enum definition
    Enum(EnumDef),
}

/// Service definition with methods
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceDef {
    /// Service name
    pub name: String,
    /// List of service methods
    pub methods: Vec<ServiceMethod>,
    /// Source code span
    pub span: Span,
}

/// Service method definition
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceMethod {
    /// Method name
    pub name: String,
    /// Method parameter with span information
    pub param: MethodParamWithSpan,
    /// Method return type with span information
    pub return_type: MethodReturnWithSpan,
    /// Source code span
    pub span: Span,
}

/// Method parameter type with source location
#[derive(Debug, Clone, PartialEq)]
pub struct MethodParamWithSpan {
    /// The parameter type
    pub param: MethodParam,
    /// Source code span
    pub span: Span,
}

/// Method parameter type
#[derive(Debug, Clone, PartialEq)]
pub enum MethodParam {
    /// Stream of data
    Stream(Box<TypeWithSpan>),
    /// Inline schema
    InlineSchema(InlineSchema),
    /// Reference to a schema
    SchemaRef(SchemaRef),
}

/// Method return type with source location
#[derive(Debug, Clone, PartialEq)]
pub struct MethodReturnWithSpan {
    /// The return type
    pub return_type: MethodReturn,
    /// Source code span
    pub span: Span,
}

/// Method return type
#[derive(Debug, Clone, PartialEq)]
pub enum MethodReturn {
    /// Stream of data
    Stream(Box<TypeWithSpan>),
    /// Inline schema
    InlineSchema(InlineSchema),
    /// Reference to a schema
    SchemaRef(SchemaRef),
}

/// Schema definition with fields
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaDef {
    /// Schema name
    pub name: String,
    /// List of schema fields
    pub fields: Vec<SchemaField>,
    /// Source code span
    pub span: Span,
}

/// Schema field definition
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaField {
    /// Field name
    pub name: String,
    /// Field type with span information
    pub field_type: TypeWithSpan,
    /// Source code span
    pub span: Span,
}

/// Enum definition with variants
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    /// Enum name
    pub name: String,
    /// List of enum variants
    pub variants: Vec<String>,
    /// Source code span
    pub span: Span,
}

/// Inline schema definition
#[derive(Debug, Clone, PartialEq)]
pub struct InlineSchema {
    /// List of inline fields
    pub fields: Vec<InlineField>,
    /// Source code span
    pub span: Span,
}

/// Inline field definition
#[derive(Debug, Clone, PartialEq)]
pub struct InlineField {
    /// Field name
    pub name: String,
    /// Field type with span information
    pub field_type: TypeWithSpan,
    /// Source code span
    pub span: Span,
}

/// Field type with source location
#[derive(Debug, Clone, PartialEq)]
pub struct TypeWithSpan {
    /// The type
    pub type_value: Type,
    /// Source code span
    pub span: Span,
}

/// Field type
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Optional type (Option<T>)
    Option(Box<TypeWithSpan>),
    /// Vector type (Vec<T>)
    Vec(Box<TypeWithSpan>),
    /// Primitive type
    Primitive(PrimitiveType),
    /// Reference to a schema
    SchemaRef(SchemaRef),
}

/// Primitive types
#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveType {
    /// String type
    String,
    /// Unsigned integer types
    U8,
    U16,
    U32,
    U64,
    U128,
    /// Signed integer types
    I8,
    I16,
    I32,
    I64,
    I128,
    /// Floating-point types
    F32,
    F64,
    /// Boolean type
    Bool,
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::String => write!(f, "string"),
            PrimitiveType::U8 => write!(f, "u8"),
            PrimitiveType::U16 => write!(f, "u16"),
            PrimitiveType::U32 => write!(f, "u32"),
            PrimitiveType::U64 => write!(f, "u64"),
            PrimitiveType::U128 => write!(f, "u128"),
            PrimitiveType::I8 => write!(f, "i8"),
            PrimitiveType::I16 => write!(f, "i16"),
            PrimitiveType::I32 => write!(f, "i32"),
            PrimitiveType::I64 => write!(f, "i64"),
            PrimitiveType::I128 => write!(f, "i128"),
            PrimitiveType::F32 => write!(f, "f32"),
            PrimitiveType::F64 => write!(f, "f64"),
            PrimitiveType::Bool => write!(f, "bool"),
        }
    }
}

/// Reference to a schema (can be qualified with package path)
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaRef {
    /// Optional package path
    pub package: Option<PackagePath>,
    /// Schema name
    pub name: String,
    /// Source code span
    pub span: Span,
}

impl fmt::Display for SchemaRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(package) = &self.package {
            write!(f, "{}.{}", package, self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

/// Parse a string into a primitive type
pub fn parse_primitive_type(s: &str) -> Option<PrimitiveType> {
    match s {
        "string" => Some(PrimitiveType::String),
        "u8" => Some(PrimitiveType::U8),
        "u16" => Some(PrimitiveType::U16),
        "u32" => Some(PrimitiveType::U32),
        "u64" => Some(PrimitiveType::U64),
        "u128" => Some(PrimitiveType::U128),
        "i8" => Some(PrimitiveType::I8),
        "i16" => Some(PrimitiveType::I16),
        "i32" => Some(PrimitiveType::I32),
        "i64" => Some(PrimitiveType::I64),
        "i128" => Some(PrimitiveType::I128),
        "f32" => Some(PrimitiveType::F32),
        "f64" => Some(PrimitiveType::F64),
        "bool" => Some(PrimitiveType::Bool),
        _ => None,
    }
}

/// Parse a string into a primitive type with span information
pub fn parse_primitive_type_with_span(s: &str, span: Span) -> Option<TypeWithSpan> {
    parse_primitive_type(s).map(|primitive_type| TypeWithSpan {
        type_value: Type::Primitive(primitive_type),
        span,
    })
}
