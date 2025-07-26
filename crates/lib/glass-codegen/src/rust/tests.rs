use super::*;
use glass_parser::ast::{
    Definition, EnumDef, PackageDecl, PackagePath, Program, SchemaDef, SchemaField, SchemaRef,
    Span, Type, TypeWithSpan,
};
use glass_parser::parser::Parser;
use std::fs;
use tempfile::TempDir;

// Helper function to create a simple program with a schema
fn create_test_program(
    package_name: &str,
    schema_name: &str,
    fields: Vec<(&str, Type)>,
) -> Program {
    let package = if package_name == "root" {
        None
    } else {
        Some(PackageDecl {
            path: PackagePath {
                segments: package_name.split('.').map(String::from).collect(),
                span: Span::dummy(),
            },
            span: Span::dummy(),
        })
    };

    let schema_fields = fields
        .into_iter()
        .map(|(name, type_value)| SchemaField {
            name: name.to_string(),
            field_type: TypeWithSpan {
                type_value,
                span: Span::dummy(),
            },
            span: Span::dummy(),
        })
        .collect();

    let schema_def = SchemaDef {
        name: schema_name.to_string(),
        fields: schema_fields,
        span: Span::dummy(),
    };

    Program {
        package,
        imports: vec![],
        definitions: vec![Definition::Schema(schema_def)],
        span: Span::dummy(),
    }
}

// Helper function to create a simple program with an enum
fn create_test_enum_program(package_name: &str, enum_name: &str, variants: Vec<&str>) -> Program {
    let package = if package_name == "root" {
        None
    } else {
        Some(PackageDecl {
            path: PackagePath {
                segments: package_name.split('.').map(String::from).collect(),
                span: Span::dummy(),
            },
            span: Span::dummy(),
        })
    };

    let enum_variants = variants.into_iter().map(|name| name.to_string()).collect();

    let enum_def = EnumDef {
        name: enum_name.to_string(),
        variants: enum_variants,
        span: Span::dummy(),
    };

    Program {
        package,
        imports: vec![],
        definitions: vec![Definition::Enum(enum_def)],
        span: Span::dummy(),
    }
}

// Helper function to write outputs to temporary directory
fn write_outputs_to_temp_dir(outputs: &[GeneratorOutput], temp_dir: &TempDir) {
    for output in outputs {
        let full_path = temp_dir.path().join(&output.path);

        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }

        fs::write(&full_path, &output.content).expect("Failed to write file");
    }
}

#[test]
fn test_rust_generator_simple_schema() {
    // Create a simple program with a schema
    let program = create_test_program(
        "com.example",
        "User",
        vec![
            ("id", Type::Primitive(PrimitiveType::String)),
            ("name", Type::Primitive(PrimitiveType::String)),
            ("age", Type::Primitive(PrimitiveType::U32)),
        ],
    );

    let programs = vec![program];
    let type_tree = TypeTree::from_programs(&programs).expect("Failed to create type tree");
    let programs_with_file_names = vec![(&programs[0], "user.glass".to_string())];

    let generator = RustGenerator::new(&type_tree, programs_with_file_names);
    let outputs = generator.generate().expect("Code generation failed");

    // Create temporary directory and write outputs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    write_outputs_to_temp_dir(&outputs, &temp_dir);

    // Verify lib.rs was generated
    let lib_rs_path = temp_dir.path().join("src/lib.rs");
    assert!(lib_rs_path.exists(), "lib.rs was not generated");
    let lib_content = fs::read_to_string(&lib_rs_path).expect("Failed to read lib.rs");
    assert!(
        lib_content.contains("pub mod com;"),
        "lib.rs should contain com module"
    );

    // Verify package structure
    let com_mod_path = temp_dir.path().join("src/com/mod.rs");
    assert!(com_mod_path.exists(), "com/mod.rs was not generated");
    let com_content = fs::read_to_string(&com_mod_path).expect("Failed to read com/mod.rs");
    assert!(
        com_content.contains("pub mod example;"),
        "com/mod.rs should contain example module"
    );

    let example_mod_path = temp_dir.path().join("src/com/example/mod.rs");
    assert!(
        example_mod_path.exists(),
        "com/example/mod.rs was not generated"
    );
    let example_content =
        fs::read_to_string(&example_mod_path).expect("Failed to read com/example/mod.rs");
    assert!(
        example_content.contains("pub mod user;"),
        "example/mod.rs should contain user module"
    );

    // Verify user.rs was generated with correct content
    let user_rs_path = temp_dir.path().join("src/com/example/user.rs");
    assert!(user_rs_path.exists(), "user.rs was not generated");
    let user_content = fs::read_to_string(&user_rs_path).expect("Failed to read user.rs");

    assert!(
        user_content.contains("pub struct User"),
        "User struct should be generated"
    );
    assert!(
        user_content.contains("pub id: String"),
        "id field should be String"
    );
    assert!(
        user_content.contains("pub name: String"),
        "name field should be String"
    );
    assert!(
        user_content.contains("pub age: u32"),
        "age field should be u32"
    );
    assert!(
        user_content.contains("serde::Serialize"),
        "Should have Serialize derive"
    );
    assert!(
        user_content.contains("serde::Deserialize"),
        "Should have Deserialize derive"
    );
}

#[test]
fn test_rust_generator_enum() {
    // Create a program with an enum
    let program = create_test_enum_program(
        "com.example",
        "Status",
        vec!["Active", "Inactive", "Pending"],
    );

    let programs = vec![program];
    let type_tree = TypeTree::from_programs(&programs).expect("Failed to create type tree");
    let programs_with_file_names = vec![(&programs[0], "status.glass".to_string())];

    let generator = RustGenerator::new(&type_tree, programs_with_file_names);
    let outputs = generator.generate().expect("Code generation failed");

    // Create a temporary directory and write outputs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    write_outputs_to_temp_dir(&outputs, &temp_dir);

    // Verify status.rs was generated with the correct content
    let status_rs_path = temp_dir.path().join("src/com/example/status.rs");
    assert!(status_rs_path.exists(), "status.rs was not generated");
    let status_content = fs::read_to_string(&status_rs_path).expect("Failed to read status.rs");

    assert!(
        status_content.contains("pub enum Status"),
        "Status enum should be generated"
    );
    assert!(
        status_content.contains("Active"),
        "Active variant should be present"
    );
    assert!(
        status_content.contains("Inactive"),
        "Inactive variant should be present"
    );
    assert!(
        status_content.contains("Pending"),
        "Pending variant should be present"
    );
    assert!(
        status_content.contains("Copy, Clone"),
        "Should have Copy and Clone derives"
    );
}

#[test]
fn test_rust_generator_root_package() {
    // Create a program in the root package
    let program = create_test_program(
        "root",
        "SimpleType",
        vec![("value", Type::Primitive(PrimitiveType::String))],
    );

    let programs = vec![program];
    let type_tree = TypeTree::from_programs(&programs).expect("Failed to create type tree");
    let programs_with_file_names = vec![(&programs[0], "simple.glass".to_string())];

    let generator = RustGenerator::new(&type_tree, programs_with_file_names);
    let outputs = generator.generate().expect("Code generation failed");

    // Create temporary directory and write outputs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    write_outputs_to_temp_dir(&outputs, &temp_dir);

    // Verify lib.rs contains root module declaration
    let lib_rs_path = temp_dir.path().join("src/lib.rs");
    let lib_content = fs::read_to_string(&lib_rs_path).expect("Failed to read lib.rs");
    assert!(
        lib_content.contains("pub mod simple;"),
        "lib.rs should contain simple module"
    );

    // Verify simple.rs was generated directly in src/
    let simple_rs_path = temp_dir.path().join("src/simple.rs");
    assert!(simple_rs_path.exists(), "simple.rs was not generated");
    let simple_content = fs::read_to_string(&simple_rs_path).expect("Failed to read simple.rs");
    assert!(
        simple_content.contains("pub struct SimpleType"),
        "SimpleType struct should be generated"
    );
}

#[test]
fn test_rust_generator_with_dependencies() {
    // Create programs with dependencies
    let user_program = create_test_program(
        "com.example",
        "User",
        vec![
            ("id", Type::Primitive(PrimitiveType::String)),
            ("name", Type::Primitive(PrimitiveType::String)),
        ],
    );

    let post_program = create_test_program(
        "com.example",
        "Post",
        vec![
            ("id", Type::Primitive(PrimitiveType::String)),
            ("title", Type::Primitive(PrimitiveType::String)),
            (
                "author",
                Type::SchemaRef(SchemaRef {
                    package: Some(PackagePath {
                        segments: vec!["com".to_string(), "example".to_string()],
                        span: Span::dummy(),
                    }),
                    name: "User".to_string(),
                    span: Span::dummy(),
                }),
            ),
        ],
    );

    let programs = vec![user_program, post_program];
    let type_tree = TypeTree::from_programs(&programs).expect("Failed to create type tree");
    let programs_with_file_names = vec![
        (&programs[0], "user.glass".to_string()),
        (&programs[1], "post.glass".to_string()),
    ];

    let generator = RustGenerator::new(&type_tree, programs_with_file_names);
    let outputs = generator.generate().expect("Code generation failed");

    // Create temporary directory and write outputs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    write_outputs_to_temp_dir(&outputs, &temp_dir);

    // Verify post.rs references User correctly
    let post_rs_path = temp_dir.path().join("src/com/example/post.rs");
    assert!(post_rs_path.exists(), "post.rs was not generated");
    let post_content = fs::read_to_string(&post_rs_path).expect("Failed to read post.rs");

    assert!(
        post_content.contains("pub struct Post"),
        "Post struct should be generated"
    );
    assert!(
        post_content.contains("crate::com::example::user::User"),
        "Should reference User with correct path"
    );
}

#[test]
fn test_rust_generator_complex_types() {
    // Create a program with complex types (Option, Vec)
    let program = create_test_program(
        "com.example",
        "ComplexType",
        vec![
            (
                "optional_field",
                Type::Option(Box::new(TypeWithSpan {
                    type_value: Type::Primitive(PrimitiveType::String),
                    span: Span::dummy(),
                })),
            ),
            (
                "list_field",
                Type::Vec(Box::new(TypeWithSpan {
                    type_value: Type::Primitive(PrimitiveType::I32),
                    span: Span::dummy(),
                })),
            ),
            (
                "nested_optional",
                Type::Option(Box::new(TypeWithSpan {
                    type_value: Type::Vec(Box::new(TypeWithSpan {
                        type_value: Type::Primitive(PrimitiveType::Bool),
                        span: Span::dummy(),
                    })),
                    span: Span::dummy(),
                })),
            ),
        ],
    );

    let programs = vec![program];
    let type_tree = TypeTree::from_programs(&programs).expect("Failed to create type tree");
    let programs_with_file_names = vec![(&programs[0], "complex.glass".to_string())];

    let generator = RustGenerator::new(&type_tree, programs_with_file_names);
    let outputs = generator.generate().expect("Code generation failed");

    // Create temporary directory and write outputs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    write_outputs_to_temp_dir(&outputs, &temp_dir);

    // Verify complex.rs has correct field types
    let complex_rs_path = temp_dir.path().join("src/com/example/complex.rs");
    assert!(complex_rs_path.exists(), "complex.rs was not generated");
    let complex_content = fs::read_to_string(&complex_rs_path).expect("Failed to read complex.rs");

    assert!(
        complex_content.contains("pub optional_field: Option<String>"),
        "Optional field should be Option<String>"
    );
    assert!(
        complex_content.contains("pub list_field: Vec<i32>"),
        "List field should be Vec<i32>"
    );
    assert!(
        complex_content.contains("pub nested_optional: Option<Vec<bool>>"),
        "Nested optional should be Option<Vec<bool>>"
    );
}

#[test]
fn test_rust_generator_multiple_packages() {
    // Create programs in different packages
    let user_program = create_test_program(
        "com.users",
        "User",
        vec![("name", Type::Primitive(PrimitiveType::String))],
    );

    let post_program = create_test_program(
        "com.posts",
        "Post",
        vec![("title", Type::Primitive(PrimitiveType::String))],
    );

    let admin_program = create_test_program(
        "com.admin.users",
        "AdminUser",
        vec![("permissions", Type::Primitive(PrimitiveType::String))],
    );

    let programs = vec![user_program, post_program, admin_program];
    let type_tree = TypeTree::from_programs(&programs).expect("Failed to create type tree");
    let programs_with_file_names = vec![
        (&programs[0], "user.glass".to_string()),
        (&programs[1], "post.glass".to_string()),
        (&programs[2], "admin_user.glass".to_string()),
    ];

    let generator = RustGenerator::new(&type_tree, programs_with_file_names);
    let outputs = generator.generate().expect("Code generation failed");

    // Create temporary directory and write outputs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    write_outputs_to_temp_dir(&outputs, &temp_dir);

    // Verify package structure
    assert!(
        temp_dir.path().join("src/com/users/user.rs").exists(),
        "users/user.rs should exist"
    );
    assert!(
        temp_dir.path().join("src/com/posts/post.rs").exists(),
        "posts/post.rs should exist"
    );
    assert!(
        temp_dir
            .path()
            .join("src/com/admin/users/admin_user.rs")
            .exists(),
        "admin/users/admin_user.rs should exist"
    );

    // Verify module hierarchy
    let lib_content =
        fs::read_to_string(temp_dir.path().join("src/lib.rs")).expect("Failed to read lib.rs");
    assert!(
        lib_content.contains("pub mod com;"),
        "lib.rs should contain com module"
    );

    let com_content = fs::read_to_string(temp_dir.path().join("src/com/mod.rs"))
        .expect("Failed to read com/mod.rs");
    assert!(
        com_content.contains("pub mod users;"),
        "com/mod.rs should contain users module"
    );
    assert!(
        com_content.contains("pub mod posts;"),
        "com/mod.rs should contain posts module"
    );
    assert!(
        com_content.contains("pub mod admin;"),
        "com/mod.rs should contain admin module"
    );
}

#[test]
fn test_rust_generator_parsed_glass_file() {
    // Parse a real Glass file
    let glass_source = r#"
            package com.example.blog;

            enum PostStatus {
                Draft,
                Published,
                Archived,
            }

            schema Author {
                id: string;
                name: string;
                email: string;
            }

            schema Post {
                id: string;
                title: string;
                content: string;
                status: PostStatus;
                author: Author;
                tags: Vec<string>;
                published_at: Option<string>;
            }
        "#;

    let program = Parser::parse(glass_source.to_string()).expect("Failed to parse Glass file");
    let programs = vec![program];
    let type_tree = TypeTree::from_programs(&programs).expect("Failed to create type tree");
    let programs_with_file_names = vec![(&programs[0], "blog.glass".to_string())];

    let generator = RustGenerator::new(&type_tree, programs_with_file_names);
    let outputs = generator.generate().expect("Code generation failed");

    // Create a temporary directory and write outputs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    write_outputs_to_temp_dir(&outputs, &temp_dir);

    // Verify blog.rs was generated with all types
    let blog_rs_path = temp_dir.path().join("src/com/example/blog/blog.rs");
    assert!(blog_rs_path.exists(), "blog.rs was not generated");
    let blog_content = fs::read_to_string(&blog_rs_path).expect("Failed to read blog.rs");

    // Check that all types are present
    assert!(
        blog_content.contains("pub enum PostStatus"),
        "PostStatus enum should be generated"
    );
    assert!(
        blog_content.contains("pub struct Author"),
        "Author struct should be generated"
    );
    assert!(
        blog_content.contains("pub struct Post"),
        "Post struct should be generated"
    );

    // Check enum variants
    assert!(
        blog_content.contains("Draft"),
        "Draft variant should be present"
    );
    assert!(
        blog_content.contains("Published"),
        "Published variant should be present"
    );
    assert!(
        blog_content.contains("Archived"),
        "Archived variant should be present"
    );

    // Check struct fields with correct types
    assert!(
        blog_content.contains("pub status: crate::com::example::blog::blog::PostStatus"),
        "Post should reference PostStatus"
    );
    assert!(
        blog_content.contains("pub author: crate::com::example::blog::blog::Author"),
        "Post should reference Author"
    );
    assert!(
        blog_content.contains("pub tags: Vec<String>"),
        "Tags should be Vec<String>"
    );
    assert!(
        blog_content.contains("pub published_at: Option<String>"),
        "Published_at should be Option<String>"
    );
}

#[test]
fn test_rust_generator_type_dependency_order() {
    // Create programs where dependency order matters
    let programs_source = r#"
            package com.example;

            schema C {
                b_ref: B;
                value: string;
            }

            schema A {
                id: string;
            }

            schema B {
                a_ref: A;
                name: string;
            }
        "#;

    let program = Parser::parse(programs_source.to_string()).expect("Failed to parse Glass file");
    let programs = vec![program];
    let type_tree = TypeTree::from_programs(&programs).expect("Failed to create type tree");
    let programs_with_file_names = vec![(&programs[0], "dependency_test.glass".to_string())];

    let generator = RustGenerator::new(&type_tree, programs_with_file_names);
    let outputs = generator.generate().expect("Code generation failed");

    // Create temporary directory and write outputs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    write_outputs_to_temp_dir(&outputs, &temp_dir);

    // Verify the file was generated
    let test_rs_path = temp_dir.path().join("src/com/example/dependency_test.rs");
    assert!(
        test_rs_path.exists(),
        "dependency_test.rs was not generated"
    );
    let test_content =
        fs::read_to_string(&test_rs_path).expect("Failed to read dependency_test.rs");

    // Verify all structs are present
    assert!(
        test_content.contains("pub struct A"),
        "A struct should be generated"
    );
    assert!(
        test_content.contains("pub struct B"),
        "B struct should be generated"
    );
    assert!(
        test_content.contains("pub struct C"),
        "C struct should be generated"
    );

    // Verify dependency order (A should come before B, B should come before C)
    let a_pos = test_content
        .find("pub struct A")
        .expect("A struct not found");
    let b_pos = test_content
        .find("pub struct B")
        .expect("B struct not found");
    let c_pos = test_content
        .find("pub struct C")
        .expect("C struct not found");

    assert!(
        a_pos < b_pos,
        "A should be defined before B due to dependency"
    );
    assert!(
        b_pos < c_pos,
        "B should be defined before C due to dependency"
    );
}

#[test]
fn test_rust_generator_mixed_types() {
    // Test with both schemas and enums in the same file
    let mixed_source = r#"
            package com.example.mixed;

            enum Color {
                Red,
                Green,
                Blue,
            }

            schema Product {
                id: string;
                name: string;
                color: Color;
                price: f64;
                in_stock: bool;
            }

            enum Size {
                Small,
                Medium,
                Large,
                ExtraLarge,
            }

            schema Clothing {
                product: Product;
                size: Size;
                material: string;
            }
        "#;

    let program = Parser::parse(mixed_source.to_string()).expect("Failed to parse Glass file");
    let programs = vec![program];
    let type_tree = TypeTree::from_programs(&programs).expect("Failed to create type tree");
    let programs_with_file_names = vec![(&programs[0], "mixed.glass".to_string())];

    let generator = RustGenerator::new(&type_tree, programs_with_file_names);
    let outputs = generator.generate().expect("Code generation failed");

    // Create a temporary directory and write outputs
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    write_outputs_to_temp_dir(&outputs, &temp_dir);

    // Verify mixed.rs was generated with all types
    let mixed_rs_path = temp_dir.path().join("src/com/example/mixed/mixed.rs");
    assert!(mixed_rs_path.exists(), "mixed.rs was not generated");
    let mixed_content = fs::read_to_string(&mixed_rs_path).expect("Failed to read mixed.rs");

    // Check that all types are present
    assert!(
        mixed_content.contains("pub enum Color"),
        "Color enum should be generated"
    );
    assert!(
        mixed_content.contains("pub struct Product"),
        "Product struct should be generated"
    );
    assert!(
        mixed_content.contains("pub enum Size"),
        "Size enum should be generated"
    );
    assert!(
        mixed_content.contains("pub struct Clothing"),
        "Clothing struct should be generated"
    );

    // Check proper type references
    assert!(
        mixed_content.contains("pub color: crate::com::example::mixed::mixed::Color"),
        "Product should reference Color enum"
    );
    assert!(
        mixed_content.contains("pub product: crate::com::example::mixed::mixed::Product"),
        "Clothing should reference Product"
    );
    assert!(
        mixed_content.contains("pub size: crate::com::example::mixed::mixed::Size"),
        "Clothing should reference Size enum"
    );
}
