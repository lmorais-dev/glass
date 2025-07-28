use crate::ast::types::Type;
use crate::parser::Rule;
use crate::prelude::*;
use pest::iterators::Pair;

/// Schema definition
///
/// Composed of its name and a vector of fields.
#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub fields: Vec<SchemaField>,
}

impl Schema {
    pub fn try_parse(pair: Pair<'_, Rule>) -> ParserResult<Self> {
        let mut inner = pair.into_inner();

        let schema_name = match inner.next() {
            Some(pair) => pair.as_str().to_owned(),
            None => {
                return Err(ParserError::NoNextToken);
            }
        };

        let schema_body_pair = match inner.next() {
            Some(pair) => pair,
            None => {
                return Err(ParserError::NoNextToken);
            }
        };

        let mut schema_fields = Vec::new();
        schema_body_pair
            .into_inner()
            .into_iter()
            .try_for_each(|pair| {
                let schema_field = SchemaField::try_parse(pair)?;
                schema_fields.push(schema_field);
                Ok::<(), ParserError>(())
            })?;

        Ok(Self {
            name: schema_name,
            fields: schema_fields,
        })
    }
}

/// Schema field definition
///
/// Composed of its name and type.
#[derive(Debug, Clone)]
pub struct SchemaField {
    pub name: String,
    pub ty: Type,
}

impl SchemaField {
    pub fn try_parse(pair: Pair<'_, Rule>) -> ParserResult<Self> {
        let mut inner = pair.into_inner();
        let field_name = match inner.next() {
            Some(pair) => pair.as_str().to_owned(),
            None => {
                return Err(ParserError::NoNextToken);
            }
        };

        let field_type = match inner.next() {
            Some(pair) => Type::try_parse(pair)?,
            None => {
                return Err(ParserError::NoNextToken);
            }
        };

        Ok(Self {
            name: field_name,
            ty: field_type,
        })
    }
}

/// SchemaRef is a way for the [Type] to refer back to a [Schema] without
/// causing a circular dependency between the types.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaRef(pub String);
