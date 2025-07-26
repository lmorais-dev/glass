use crate::ast::{
    Definition, EnumDef, ImportStmt, InlineField, InlineSchema, MethodParam, MethodParamWithSpan,
    MethodReturn, MethodReturnWithSpan, PackageDecl, PackagePath, Program, SchemaDef, SchemaField,
    SchemaRef, ServiceDef, ServiceMethod, Span, Type, TypeWithSpan,
};
use crate::error::ParserError;
use pest::Parser as PestParserTrait;
use pest::iterators::Pair;

#[derive(pest_derive::Parser)]
#[grammar = "grammars/glass_v1.pest"]
pub struct PestParser;

pub struct Parser;

impl Parser {
    pub fn parse(source: String) -> Result<Program, ParserError> {
        // Parse the source string using the PestParser
        let pairs = PestParser::parse(Rule::program, &source)
            .map_err(|error| ParserError::PestError(Box::new(error)))?;

        // Convert the parse tree to an AST
        let program_pair = pairs
            .peek()
            .ok_or_else(|| ParserError::MissingElement("program".to_string()))?;

        Self::parse_program(program_pair)
    }

    fn parse_program(pair: Pair<Rule>) -> Result<Program, ParserError> {
        if pair.as_rule() != Rule::program {
            return Err(ParserError::UnexpectedRule {
                expected: "program".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut package = None;
        let mut imports = Vec::new();
        let mut definitions = Vec::new();

        // Process the inner pairs
        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::package_decl => {
                    package = Some(Self::parse_package_decl(inner_pair)?);
                }
                Rule::import_stmt => {
                    imports.push(Self::parse_import_stmt(inner_pair)?);
                }
                Rule::service_def => {
                    definitions.push(Definition::Service(Self::parse_service_def(inner_pair)?));
                }
                Rule::schema_def => {
                    definitions.push(Definition::Schema(Self::parse_schema_def(inner_pair)?));
                }
                Rule::enum_def => {
                    definitions.push(Definition::Enum(Self::parse_enum_def(inner_pair)?));
                }
                Rule::EOI => {
                    // End of input, ignore
                }
                _ => {
                    return Err(ParserError::UnexpectedRule {
                        expected: "package_decl, import_stmt, service_def, schema_def, or enum_def"
                            .to_string(),
                        found: format!("{:?}", inner_pair.as_rule()),
                    });
                }
            }
        }

        Ok(Program {
            package,
            imports,
            definitions,
            span,
        })
    }

    fn parse_package_decl(pair: Pair<Rule>) -> Result<PackageDecl, ParserError> {
        if pair.as_rule() != Rule::package_decl {
            return Err(ParserError::UnexpectedRule {
                expected: "package_decl".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut path = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::package_path => {
                    path = Some(Self::parse_package_path(inner_pair)?);
                }
                _ => {
                    return Err(ParserError::UnexpectedRule {
                        expected: "package_path".to_string(),
                        found: format!("{:?}", inner_pair.as_rule()),
                    });
                }
            }
        }

        let path = path.ok_or_else(|| ParserError::MissingElement("package_path".to_string()))?;

        Ok(PackageDecl { path, span })
    }

    fn parse_package_path(pair: Pair<Rule>) -> Result<PackagePath, ParserError> {
        if pair.as_rule() != Rule::package_path {
            return Err(ParserError::UnexpectedRule {
                expected: "package_path".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut segments = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::identifier => {
                    segments.push(inner_pair.as_str().to_string());
                }
                _ => {
                    return Err(ParserError::UnexpectedRule {
                        expected: "identifier".to_string(),
                        found: format!("{:?}", inner_pair.as_rule()),
                    });
                }
            }
        }

        Ok(PackagePath { segments, span })
    }

    fn parse_import_stmt(pair: Pair<Rule>) -> Result<ImportStmt, ParserError> {
        if pair.as_rule() != Rule::import_stmt {
            return Err(ParserError::UnexpectedRule {
                expected: "import_stmt".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut path = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::string_literal => {
                    // Remove the quotes from the string literal
                    let raw_str = inner_pair.as_str();
                    path = Some(raw_str[1..raw_str.len() - 1].to_string());
                }
                _ => {
                    // Ignore other rules (like the semicolon)
                }
            }
        }

        let path = path.ok_or_else(|| ParserError::MissingElement("string_literal".to_string()))?;

        Ok(ImportStmt { path, span })
    }

    fn parse_enum_def(pair: Pair<Rule>) -> Result<EnumDef, ParserError> {
        if pair.as_rule() != Rule::enum_def {
            return Err(ParserError::UnexpectedRule {
                expected: "enum_def".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut name = None;
        let mut variants = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::identifier => {
                    // The first identifier is the enum name
                    if name.is_none() {
                        name = Some(inner_pair.as_str().to_string());
                    }
                }
                Rule::enum_variant => {
                    // Parse enum variant
                    let variant = inner_pair.into_inner().next().ok_or_else(|| {
                        ParserError::MissingElement("enum variant identifier".to_string())
                    })?;
                    if variant.as_rule() == Rule::identifier {
                        variants.push(variant.as_str().to_string());
                    } else {
                        return Err(ParserError::UnexpectedRule {
                            expected: "identifier".to_string(),
                            found: format!("{:?}", variant.as_rule()),
                        });
                    }
                }
                _ => {
                    // Ignore other rules (like commas and braces)
                }
            }
        }

        let name = name.ok_or_else(|| ParserError::MissingElement("enum name".to_string()))?;

        Ok(EnumDef {
            name,
            variants,
            span,
        })
    }

    fn parse_schema_def(pair: Pair<Rule>) -> Result<SchemaDef, ParserError> {
        if pair.as_rule() != Rule::schema_def {
            return Err(ParserError::UnexpectedRule {
                expected: "schema_def".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut name = None;
        let mut fields = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::identifier => {
                    // The first identifier is the schema name
                    if name.is_none() {
                        name = Some(inner_pair.as_str().to_string());
                    }
                }
                Rule::schema_field => {
                    // Parse schema field
                    fields.push(Self::parse_schema_field(inner_pair)?);
                }
                _ => {
                    // Ignore other rules (like braces)
                }
            }
        }

        let name = name.ok_or_else(|| ParserError::MissingElement("schema name".to_string()))?;

        Ok(SchemaDef { name, fields, span })
    }

    fn parse_schema_field(pair: Pair<Rule>) -> Result<SchemaField, ParserError> {
        if pair.as_rule() != Rule::schema_field {
            return Err(ParserError::UnexpectedRule {
                expected: "schema_field".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut name = None;
        let mut field_type = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::identifier => {
                    name = Some(inner_pair.as_str().to_string());
                }
                Rule::field_type => {
                    field_type = Some(Self::parse_field_type(inner_pair)?);
                }
                _ => {
                    // Ignore other rules (like colons and semicolons)
                }
            }
        }

        let name = name.ok_or_else(|| ParserError::MissingElement("field name".to_string()))?;

        let field_type =
            field_type.ok_or_else(|| ParserError::MissingElement("field type".to_string()))?;

        Ok(SchemaField {
            name,
            field_type,
            span,
        })
    }

    fn parse_field_type(pair: Pair<Rule>) -> Result<TypeWithSpan, ParserError> {
        if pair.as_rule() != Rule::field_type {
            return Err(ParserError::UnexpectedRule {
                expected: "field_type".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let inner_pair = pair
            .into_inner()
            .next()
            .ok_or_else(|| ParserError::MissingElement("field type inner".to_string()))?;

        match inner_pair.as_rule() {
            Rule::option_type => {
                let option_span = Span::from_pest(inner_pair.as_span());
                let inner_type = inner_pair
                    .into_inner()
                    .next()
                    .ok_or_else(|| ParserError::MissingElement("option inner type".to_string()))?;

                if inner_type.as_rule() == Rule::field_type {
                    let inner_type_with_span = Self::parse_field_type(inner_type)?;
                    Ok(TypeWithSpan {
                        type_value: Type::Option(Box::new(inner_type_with_span)),
                        span: option_span,
                    })
                } else {
                    Err(ParserError::UnexpectedRule {
                        expected: "field_type".to_string(),
                        found: format!("{:?}", inner_type.as_rule()),
                    })
                }
            }
            Rule::vec_type => {
                let vec_span = Span::from_pest(inner_pair.as_span());
                let inner_type = inner_pair
                    .into_inner()
                    .next()
                    .ok_or_else(|| ParserError::MissingElement("vec inner type".to_string()))?;

                if inner_type.as_rule() == Rule::field_type {
                    let inner_type_with_span = Self::parse_field_type(inner_type)?;
                    Ok(TypeWithSpan {
                        type_value: Type::Vec(Box::new(inner_type_with_span)),
                        span: vec_span,
                    })
                } else {
                    Err(ParserError::UnexpectedRule {
                        expected: "field_type".to_string(),
                        found: format!("{:?}", inner_type.as_rule()),
                    })
                }
            }
            Rule::primitive_type => {
                let primitive_span = Span::from_pest(inner_pair.as_span());
                let primitive_str = inner_pair.as_str();
                let primitive_type = crate::ast::parse_primitive_type(primitive_str)
                    .ok_or_else(|| ParserError::InvalidPrimitiveType(primitive_str.to_string()))?;

                Ok(TypeWithSpan {
                    type_value: Type::Primitive(primitive_type),
                    span: primitive_span,
                })
            }
            Rule::schema_ref => Ok(TypeWithSpan {
                type_value: Type::SchemaRef(Self::parse_schema_ref(inner_pair)?),
                span,
            }),
            _ => Err(ParserError::UnexpectedRule {
                expected: "option_type, vec_type, primitive_type, or schema_ref".to_string(),
                found: format!("{:?}", inner_pair.as_rule()),
            }),
        }
    }

    fn parse_schema_ref(pair: Pair<Rule>) -> Result<SchemaRef, ParserError> {
        if pair.as_rule() != Rule::schema_ref {
            return Err(ParserError::UnexpectedRule {
                expected: "schema_ref".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut package = None;
        let mut name = None;

        let inner_pairs: Vec<Pair<Rule>> = pair.into_inner().collect();

        // If there are at least 2 pairs, the first is the package path and the second is the name
        if inner_pairs.len() >= 2 {
            let package_pair = &inner_pairs[0];
            if package_pair.as_rule() == Rule::package_path {
                package = Some(Self::parse_package_path(package_pair.clone())?);
            }

            let name_pair = &inner_pairs[inner_pairs.len() - 1];
            if name_pair.as_rule() == Rule::identifier {
                name = Some(name_pair.as_str().to_string());
            }
        } else if inner_pairs.len() == 1 {
            // If there's only one pair, it's the name
            let name_pair = &inner_pairs[0];
            if name_pair.as_rule() == Rule::identifier {
                name = Some(name_pair.as_str().to_string());
            }
        }

        let name =
            name.ok_or_else(|| ParserError::MissingElement("schema reference name".to_string()))?;

        Ok(SchemaRef {
            package,
            name,
            span,
        })
    }

    fn parse_service_def(pair: Pair<Rule>) -> Result<ServiceDef, ParserError> {
        if pair.as_rule() != Rule::service_def {
            return Err(ParserError::UnexpectedRule {
                expected: "service_def".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut name = None;
        let mut methods = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::identifier => {
                    // The first identifier is the service name
                    if name.is_none() {
                        name = Some(inner_pair.as_str().to_string());
                    }
                }
                Rule::service_method => {
                    // Parse service method
                    methods.push(Self::parse_service_method(inner_pair)?);
                }
                _ => {
                    // Ignore other rules (like braces)
                }
            }
        }

        let name = name.ok_or_else(|| ParserError::MissingElement("service name".to_string()))?;

        Ok(ServiceDef {
            name,
            methods,
            span,
        })
    }

    fn parse_service_method(pair: Pair<Rule>) -> Result<ServiceMethod, ParserError> {
        if pair.as_rule() != Rule::service_method {
            return Err(ParserError::UnexpectedRule {
                expected: "service_method".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut name = None;
        let mut param = None;
        let mut return_type = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::identifier => {
                    // The first identifier is the method name
                    if name.is_none() {
                        name = Some(inner_pair.as_str().to_string());
                    }
                }
                Rule::method_param => {
                    param = Some(Self::parse_method_param(inner_pair)?);
                }
                Rule::method_return => {
                    return_type = Some(Self::parse_method_return(inner_pair)?);
                }
                _ => {
                    // Ignore other rules (like parentheses, arrow, and semicolon)
                }
            }
        }

        let name = name.ok_or_else(|| ParserError::MissingElement("method name".to_string()))?;

        let param =
            param.ok_or_else(|| ParserError::MissingElement("method parameter".to_string()))?;

        let return_type = return_type
            .ok_or_else(|| ParserError::MissingElement("method return type".to_string()))?;

        Ok(ServiceMethod {
            name,
            param,
            return_type,
            span,
        })
    }

    fn parse_method_param(pair: Pair<Rule>) -> Result<MethodParamWithSpan, ParserError> {
        if pair.as_rule() != Rule::method_param {
            return Err(ParserError::UnexpectedRule {
                expected: "method_param".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let inner_pair = pair
            .into_inner()
            .next()
            .ok_or_else(|| ParserError::MissingElement("method parameter inner".to_string()))?;

        let param = match inner_pair.as_rule() {
            Rule::stream_type => {
                MethodParam::Stream(Box::new(Self::parse_stream_type(inner_pair)?))
            }
            Rule::inline_schema => {
                MethodParam::InlineSchema(Self::parse_inline_schema(inner_pair)?)
            }
            Rule::schema_ref => MethodParam::SchemaRef(Self::parse_schema_ref(inner_pair)?),
            _ => {
                return Err(ParserError::UnexpectedRule {
                    expected: "stream_type, inline_schema, or schema_ref".to_string(),
                    found: format!("{:?}", inner_pair.as_rule()),
                });
            }
        };

        Ok(MethodParamWithSpan { param, span })
    }

    fn parse_method_return(pair: Pair<Rule>) -> Result<MethodReturnWithSpan, ParserError> {
        if pair.as_rule() != Rule::method_return {
            return Err(ParserError::UnexpectedRule {
                expected: "method_return".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let inner_pair = pair
            .into_inner()
            .next()
            .ok_or_else(|| ParserError::MissingElement("method return inner".to_string()))?;

        let return_type = match inner_pair.as_rule() {
            Rule::stream_type => {
                MethodReturn::Stream(Box::new(Self::parse_stream_type(inner_pair)?))
            }
            Rule::inline_schema => {
                MethodReturn::InlineSchema(Self::parse_inline_schema(inner_pair)?)
            }
            Rule::schema_ref => MethodReturn::SchemaRef(Self::parse_schema_ref(inner_pair)?),
            _ => {
                return Err(ParserError::UnexpectedRule {
                    expected: "stream_type, inline_schema, or schema_ref".to_string(),
                    found: format!("{:?}", inner_pair.as_rule()),
                });
            }
        };

        Ok(MethodReturnWithSpan { return_type, span })
    }

    fn parse_stream_type(pair: Pair<Rule>) -> Result<TypeWithSpan, ParserError> {
        if pair.as_rule() != Rule::stream_type {
            return Err(ParserError::UnexpectedRule {
                expected: "stream_type".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let inner_pair = pair
            .into_inner()
            .next()
            .ok_or_else(|| ParserError::MissingElement("stream type inner".to_string()))?;

        match inner_pair.as_rule() {
            Rule::inline_schema => {
                let inline_schema = Self::parse_inline_schema(inner_pair)?;
                Ok(TypeWithSpan {
                    type_value: Type::SchemaRef(SchemaRef {
                        package: None,
                        name: format!("InlineSchema_{}", inline_schema.span.start.0),
                        span: inline_schema.span.clone(),
                    }),
                    span,
                })
            }
            Rule::schema_ref => {
                let schema_ref = Self::parse_schema_ref(inner_pair)?;
                Ok(TypeWithSpan {
                    type_value: Type::SchemaRef(schema_ref),
                    span,
                })
            }
            _ => Err(ParserError::UnexpectedRule {
                expected: "inline_schema or schema_ref".to_string(),
                found: format!("{:?}", inner_pair.as_rule()),
            }),
        }
    }

    fn parse_inline_schema(pair: Pair<Rule>) -> Result<InlineSchema, ParserError> {
        if pair.as_rule() != Rule::inline_schema {
            return Err(ParserError::UnexpectedRule {
                expected: "inline_schema".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut fields = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::inline_field => {
                    fields.push(Self::parse_inline_field(inner_pair)?);
                }
                _ => {
                    // Ignore other rules (like braces)
                }
            }
        }

        Ok(InlineSchema { fields, span })
    }

    fn parse_inline_field(pair: Pair<Rule>) -> Result<InlineField, ParserError> {
        if pair.as_rule() != Rule::inline_field {
            return Err(ParserError::UnexpectedRule {
                expected: "inline_field".to_string(),
                found: format!("{:?}", pair.as_rule()),
            });
        }

        let span = Span::from_pest(pair.as_span());
        let mut name = None;
        let mut field_type = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::identifier => {
                    name = Some(inner_pair.as_str().to_string());
                }
                Rule::field_type => {
                    field_type = Some(Self::parse_field_type(inner_pair)?);
                }
                _ => {
                    // Ignore other rules (like colons, commas, and semicolons)
                }
            }
        }

        let name =
            name.ok_or_else(|| ParserError::MissingElement("inline field name".to_string()))?;

        let field_type = field_type
            .ok_or_else(|| ParserError::MissingElement("inline field type".to_string()))?;

        Ok(InlineField {
            name,
            field_type,
            span,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_empty_program() {
        let source = "".to_string();
        let result = Parser::parse(source);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert!(program.package.is_none());
        assert!(program.imports.is_empty());
        assert!(program.definitions.is_empty());
    }

    #[test]
    fn test_parse_package_declaration() {
        let source = "package com.example.test;".to_string();
        let result = Parser::parse(source);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert!(program.package.is_some());
        let package = program.package.unwrap();
        assert_eq!(package.path.segments, vec!["com", "example", "test"]);
    }

    #[test]
    fn test_parse_import_statement() {
        let source = "import \"com/example/other.glass\";".to_string();
        let result = Parser::parse(source);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.imports.len(), 1);
        assert_eq!(program.imports[0].path, "com/example/other.glass");
    }

    #[test]
    fn test_parse_enum_definition() {
        let source = "enum Status { OK, ERROR, PENDING }".to_string();
        let result = Parser::parse(source);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.definitions.len(), 1);
        match &program.definitions[0] {
            crate::ast::Definition::Enum(enum_def) => {
                assert_eq!(enum_def.name, "Status");
                assert_eq!(enum_def.variants, vec!["OK", "ERROR", "PENDING"]);
            }
            _ => panic!("Expected enum definition"),
        }
    }

    #[test]
    fn test_parse_schema_definition() {
        let source = "schema User {
            id: string;
            name: string;
            age: u32;
            is_active: bool;
        }"
        .to_string();
        let result = Parser::parse(source);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.definitions.len(), 1);
        match &program.definitions[0] {
            Definition::Schema(schema_def) => {
                assert_eq!(schema_def.name, "User");
                assert_eq!(schema_def.fields.len(), 4);
                assert_eq!(schema_def.fields[0].name, "id");
                assert_eq!(schema_def.fields[1].name, "name");
                assert_eq!(schema_def.fields[2].name, "age");
                assert_eq!(schema_def.fields[3].name, "is_active");
            }
            _ => panic!("Expected schema definition"),
        }
    }

    #[test]
    fn test_parse_service_definition() {
        let source = "service UserService {
            fn getUser(User) -> User;
            fn listUsers(stream User) -> stream User;
            fn createUser({ name: string, age: u32 }) -> User;
        }"
        .to_string();
        let result = Parser::parse(source);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.definitions.len(), 1);
        match &program.definitions[0] {
            crate::ast::Definition::Service(service_def) => {
                assert_eq!(service_def.name, "UserService");
                assert_eq!(service_def.methods.len(), 3);
                assert_eq!(service_def.methods[0].name, "getUser");
                assert_eq!(service_def.methods[1].name, "listUsers");
                assert_eq!(service_def.methods[2].name, "createUser");
            }
            _ => panic!("Expected service definition"),
        }
    }

    #[test]
    fn test_parse_complete_program() {
        let source = r#"
        package com.example.test;
        
        import "com/example/other.glass";
        
        enum Status {
            OK,
            ERROR,
            PENDING
        }
        
        schema User {
            id: string;
            name: string;
            age: u32;
            status: Status;
            is_active: bool;
        }
        
        service UserService {
            fn getUser(User) -> User;
            fn listUsers(stream User) -> stream User;
            fn createUser({ name: string, age: u32 }) -> User;
            fn updateStatus(User) -> Status;
        }
        "#
        .to_string();

        let result = Parser::parse(source);
        assert!(result.is_ok());
        let program = result.unwrap();

        // Check package
        assert!(program.package.is_some());
        let package = program.package.unwrap();
        assert_eq!(package.path.segments, vec!["com", "example", "test"]);

        // Check imports
        assert_eq!(program.imports.len(), 1);
        assert_eq!(program.imports[0].path, "com/example/other.glass");

        // Check definitions
        assert_eq!(program.definitions.len(), 3);

        // Check enum definition
        match &program.definitions[0] {
            crate::ast::Definition::Enum(enum_def) => {
                assert_eq!(enum_def.name, "Status");
                assert_eq!(enum_def.variants, vec!["OK", "ERROR", "PENDING"]);
            }
            _ => panic!("Expected enum definition"),
        }

        // Check schema definition
        match &program.definitions[1] {
            crate::ast::Definition::Schema(schema_def) => {
                assert_eq!(schema_def.name, "User");
                assert_eq!(schema_def.fields.len(), 5);
                assert_eq!(schema_def.fields[0].name, "id");
                assert_eq!(schema_def.fields[1].name, "name");
                assert_eq!(schema_def.fields[2].name, "age");
                assert_eq!(schema_def.fields[3].name, "status");
                assert_eq!(schema_def.fields[4].name, "is_active");
            }
            _ => panic!("Expected schema definition"),
        }

        // Check service definition
        match &program.definitions[2] {
            crate::ast::Definition::Service(service_def) => {
                assert_eq!(service_def.name, "UserService");
                assert_eq!(service_def.methods.len(), 4);
                assert_eq!(service_def.methods[0].name, "getUser");
                assert_eq!(service_def.methods[1].name, "listUsers");
                assert_eq!(service_def.methods[2].name, "createUser");
                assert_eq!(service_def.methods[3].name, "updateStatus");
            }
            _ => panic!("Expected service definition"),
        }
    }

    #[test]
    fn test_parse_error_invalid_syntax() {
        let source = "package com.example.test;\n\nservice UserService {\n  fn getUser(User) ->\n}"
            .to_string();
        let result = Parser::parse(source);
        assert!(result.is_err());
    }
}
