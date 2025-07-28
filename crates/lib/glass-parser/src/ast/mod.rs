use crate::ast::interface::Interface;
use crate::ast::schema::Schema;
use crate::parser::{Parser as GlassParser, Rule};
use crate::prelude::*;
use pest::Parser;
use std::path::PathBuf;
use tracing::{error, info};

pub mod interface;
pub mod schema;
pub mod types;

/// Defines a Glass file
///
/// This struct holds a crudely parsed AST, meaning it just parses and
/// exposes the parsed tree without any kind of validation.
#[derive(Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub interfaces: Vec<Interface>,
    pub schemas: Vec<Schema>,
}

impl File {
    pub fn try_new(path: PathBuf) -> ParserResult<Self> {
        if !path.exists() {
            let file_path = path.to_str().unwrap_or("unknown");
            return Err(ParserError::FileNotFound(file_path.to_string()));
        }

        Ok(Self {
            path,
            interfaces: vec![],
            schemas: vec![],
        })
    }

    #[tracing::instrument(skip(self))]
    pub fn try_parse(&mut self) -> ParserResult<()> {
        info!(path = ?self.path, "A parsing job has begun");
        let contents = self.read_file_contents()?;
        if contents.is_empty() {
            return Ok(());
        }

        let pairs = match GlassParser::parse(Rule::file, &contents) {
            Ok(pairs) => pairs,
            Err(error) => {
                error!(path = ?self.path, "Failed to parse the Glass code");
                return Err(ParserError::Pest(Box::from(error)));
            }
        };

        let mut interfaces = vec![];
        let mut schemas = vec![];

        for pair in pairs {
            match pair.as_rule() {
                Rule::file => {
                    let inner = pair.into_inner();
                    for pair in inner {
                        match pair.as_rule() {
                            Rule::interface_decl => {
                                let interface = Interface::try_parse(pair)?;
                                interfaces.push(interface);
                            }
                            Rule::schema_decl => {
                                let schema = Schema::try_parse(pair)?;
                                schemas.push(schema);
                            }
                            Rule::EOI => (),
                            _ => {
                                error!(path = ?self.path, "Unexpected rule: {:?}", pair.as_rule());
                                return Err(ParserError::UnexpectedRule(pair.as_rule()));
                            }
                        }
                    }
                }
                Rule::EOI => (),
                _ => {
                    error!(path = ?self.path, "Unexpected rule: {:?}", pair.as_rule());
                    return Err(ParserError::UnexpectedRule(pair.as_rule()));
                }
            }
        }

        self.interfaces = interfaces;
        self.schemas = schemas;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn read_file_contents(&self) -> ParserResult<String> {
        // If we are calling this, then we already validated that the file at least exists.
        // So it should be fine to just try reading from it.
        std::fs::read_to_string(&self.path).map_err(|error| {
            error!(?error, path = ?self.path, "Failed to read the specified file");
            ParserError::Io(error)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::interface::{FunctionParam, FunctionReturn};
    use crate::ast::types::{PrimitiveType, Type};
    use crate::prelude::*;
    use std::fs::File as StdFile;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::Builder;

    /// Helper to create a named temporary file with specific content.
    fn create_temp_file(prefix: &str, content: &str) -> (PathBuf, impl FnOnce()) {
        let temp_dir = Builder::new().prefix(prefix).tempdir().unwrap();
        let file_path = temp_dir.path().join("test.glass");
        let mut file = StdFile::create(&file_path).unwrap();
        file.write_fmt(format_args!("{content}")).unwrap();

        let path_buf = file_path.to_path_buf();
        let cleanup = move || temp_dir.close().unwrap();

        (path_buf, cleanup)
    }

    #[test]
    fn test_file_try_new_not_found() {
        let path = PathBuf::from("non_existent_file.glass");
        let result = File::try_new(path);
        assert!(matches!(result, Err(ParserError::FileNotFound(_))));
    }

    #[test]
    fn test_parse_empty_file() {
        let (path, cleanup) = create_temp_file("empty", "");
        let mut file = File::try_new(path).unwrap();
        let result = file.try_parse();

        assert!(result.is_ok());
        assert!(file.schemas.is_empty());
        assert!(file.interfaces.is_empty());

        cleanup();
    }

    #[test]
    fn test_parse_syntax_error() {
        let (path, cleanup) = create_temp_file("syntax_error", "schema Oops { id: u64 ");
        let mut file = File::try_new(path).unwrap();
        let result = file.try_parse();

        assert!(matches!(result, Err(ParserError::Pest(_))));

        cleanup();
    }

    #[test]
    fn test_full_file_parse_success() {
        let content = r#"
            schema User {
                id: u64;
                name: string;
            }

            interface Greeter {
                fn say_hello(User) -> option<string>;
                fn logout(User);
            }
        "#;
        let (path, cleanup) = create_temp_file("full_parse", content);
        let mut file = File::try_new(path).unwrap();
        let result = file.try_parse();

        // 1. Assert parsing was successful
        assert!(result.is_ok());

        // 2. Assert schemas were parsed correctly
        assert_eq!(file.schemas.len(), 1);
        let schema = &file.schemas[0];
        assert_eq!(schema.name, "User");
        assert_eq!(schema.fields.len(), 2);
        assert_eq!(schema.fields[0].name, "id");
        assert_eq!(schema.fields[0].ty, Type::Primitive(PrimitiveType::U64));

        // 3. Assert interfaces were parsed correctly
        assert_eq!(file.interfaces.len(), 1);
        let interface = &file.interfaces[0];
        assert_eq!(interface.name, "Greeter");
        assert_eq!(interface.functions.len(), 2);

        let say_hello = &interface.functions[0];
        assert_eq!(say_hello.name, "say_hello");
        assert!(matches!(
            say_hello.param,
            FunctionParam::Simple(Type::Schema(_))
        ));
        assert!(matches!(
            say_hello.return_type,
            Some(FunctionReturn::Simple(Type::Option(_)))
        ));

        let logout = &interface.functions[1];
        assert_eq!(logout.name, "logout");
        assert!(logout.return_type.is_none());

        cleanup();
    }
}
