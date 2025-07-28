use crate::prelude::*;
use quote::quote;

mod interface;
mod schema;
mod util;

pub fn generate(validated_file: &ValidatedFile) -> String {
    let mut generated_code = Vec::new();
    for schema in validated_file.schema_map.values() {
        let generated_schema = schema::generate_schema(schema);
        generated_code.push(generated_schema);
    }

    for interface in validated_file.interface_map.values() {
        let generated_interface = interface::generate_interface(interface);
        generated_code.push(generated_interface);
    }

    let generated_code = quote! {
        #(#generated_code)*
    };

    let syntax_tree = syn::parse2::<syn::File>(generated_code).unwrap();
    prettyplease::unparse(&syntax_tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glass_parser::ast::File;
    use glass_parser::prelude::ValidatedFile;
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
    fn test_generate_success() {
        let content = r#"
            interface Greeter {
                fn say_hello(User) -> string;
                fn greet_all(GreetAllRequest) -> stream GreetAllResponseItem;
            }
            
            schema User {
                id: u64;
            }

            schema GreetAllRequest {
                people: vec<User>;   
            }
            
            schema GreetAllResponseItem {
                user_id: u64;
                message: string;
            }
        "#;
        let (path, cleanup) = create_temp_file("validate_success", content);
        let mut file = File::try_new(path).unwrap();
        file.try_parse().unwrap();

        let result = ValidatedFile::validate(file);
        assert!(result.is_ok());

        let validated_file = result.unwrap();
        let generated_code = generate(&validated_file);
        println!("{generated_code}");
        assert!(!generated_code.is_empty());

        cleanup();
    }
}
