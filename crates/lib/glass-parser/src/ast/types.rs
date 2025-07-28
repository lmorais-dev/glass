use crate::ast::schema::SchemaRef;
use crate::error::ParserError;
use crate::parser::Rule;
use crate::prelude::ParserResult;
use pest::iterators::Pair;

/// Primitive types for Glass
#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveType {
    String,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Bool,
}

/// Option type for Glass
///
/// The inner field is a [Box] so to avoid problems
/// with recursive types.
#[derive(Debug, Clone, PartialEq)]
pub struct OptionType {
    pub inner: Box<Type>,
}

/// Vector type for Glass
///
/// The inner field is a [Box] so to avoid problems
/// with recursive types.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorType {
    pub inner: Box<Type>,
}

/// Main type definition for Glass
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Primitive(PrimitiveType),
    Option(OptionType),
    Vector(VectorType),
    Schema(SchemaRef),
}

impl Type {
    pub fn try_parse(pair: Pair<'_, Rule>) -> ParserResult<Self> {
        match pair.as_rule() {
            Rule::type_decl => {
                let inner_type = match pair.into_inner().next() {
                    Some(inner_type) => inner_type,
                    None => return Err(ParserError::NoNextToken),
                };

                Self::try_parse(inner_type)
            }
            Rule::primitive_type => {
                let primitive = Self::parse_string_to_primitive_type(pair.as_str());
                Ok(Type::Primitive(primitive))
            }
            Rule::option_type => {
                let inner_type = match pair.into_inner().next() {
                    Some(inner_type) => inner_type,
                    None => return Err(ParserError::NoNextToken),
                };

                Ok(Type::Option(OptionType {
                    inner: Box::new(Self::try_parse(inner_type)?),
                }))
            }
            Rule::vector_type => {
                let inner_type = match pair.into_inner().next() {
                    Some(inner_type) => inner_type,
                    None => return Err(ParserError::NoNextToken),
                };

                Ok(Type::Vector(VectorType {
                    inner: Box::new(Self::try_parse(inner_type)?),
                }))
            }
            Rule::schema_ident => Ok(Type::Schema(SchemaRef(pair.as_str().to_owned()))),
            _ => {
                Err(ParserError::UnexpectedRule(pair.as_rule()))
            }
        }
    }

    fn parse_string_to_primitive_type(primitive: &str) -> PrimitiveType {
        match primitive {
            "string" => PrimitiveType::String,
            "bool" => PrimitiveType::Bool,
            "u8" => PrimitiveType::U8,
            "u16" => PrimitiveType::U16,
            "u32" => PrimitiveType::U32,
            "u64" => PrimitiveType::U64,
            "u128" => PrimitiveType::U128,
            "i8" => PrimitiveType::I8,
            "i16" => PrimitiveType::I16,
            "i32" => PrimitiveType::I32,
            "i64" => PrimitiveType::I64,
            "i128" => PrimitiveType::I128,
            "f32" => PrimitiveType::F32,
            "f64" => PrimitiveType::F64,
            // The grammar itself guarantees this is unreachable, so we can safely assume it so.
            _ => unreachable!(),
        }
    }
}
