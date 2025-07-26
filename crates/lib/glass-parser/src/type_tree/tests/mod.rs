use crate::ast::*;
use crate::parser::Parser;
use crate::type_tree::*;

// Helper function to create a simple program with a schema
fn create_test_program(
    package_name: &str,
    schema_name: &str,
    fields: Vec<(&str, Type)>,
) -> Program {
    let package = Some(PackageDecl {
        path: PackagePath {
            segments: package_name.split('.').map(String::from).collect(),
            span: Span::dummy(),
        },
        span: Span::dummy(),
    });

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
    let package = Some(PackageDecl {
        path: PackagePath {
            segments: package_name.split('.').map(String::from).collect(),
            span: Span::dummy(),
        },
        span: Span::dummy(),
    });

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

#[test]
fn test_basic_type_tree() {
    // Create a simple program with a schema
    let program = create_test_program(
        "com.example",
        "User",
        vec![
            ("id", Type::Primitive(PrimitiveType::String)),
            ("name", Type::Primitive(PrimitiveType::String)),
        ],
    );

    // Build a type tree from the program
    let type_tree = TypeTree::from_programs(&[program]).unwrap();

    // Check that the type exists in the tree
    assert!(type_tree.has_type("com.example.User"));

    // Check that the type has no dependencies
    let dependencies = type_tree.get_dependencies("com.example.User").unwrap();
    assert!(dependencies.is_empty());
}

#[test]
fn test_type_dependencies() {
    // Create a program with a schema that depends on another schema
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

    // Build a type tree from the programs
    let type_tree = TypeTree::from_programs(&[user_program, post_program]).unwrap();

    // Check that the Post type depends on the User type
    let dependencies = type_tree.get_dependencies("com.example.Post").unwrap();
    assert!(dependencies.contains("com.example.User"));

    // Check that the User type has no dependencies
    let dependencies = type_tree.get_dependencies("com.example.User").unwrap();
    assert!(dependencies.is_empty());

    // Check that the User type is a dependent of the Post type
    let dependents = type_tree.get_dependents("com.example.User");
    assert!(dependents.contains(&"com.example.Post".to_string()));
}

#[test]
fn test_circular_dependency_detection() {
    // Create programs with circular dependencies
    let a_program = create_test_program(
        "com.example",
        "A",
        vec![(
            "b",
            Type::SchemaRef(SchemaRef {
                package: Some(PackagePath {
                    segments: vec!["com".to_string(), "example".to_string()],
                    span: Span::dummy(),
                }),
                name: "B".to_string(),
                span: Span::dummy(),
            }),
        )],
    );

    let b_program = create_test_program(
        "com.example",
        "B",
        vec![(
            "a",
            Type::SchemaRef(SchemaRef {
                package: Some(PackagePath {
                    segments: vec!["com".to_string(), "example".to_string()],
                    span: Span::dummy(),
                }),
                name: "A".to_string(),
                span: Span::dummy(),
            }),
        )],
    );

    // Building a type tree should fail with a circular dependency error
    let result = TypeTree::from_programs(&[a_program, b_program]);
    assert!(result.is_err());
    match result {
        Err(TypeTreeError::CircularDependency { .. }) => {
            // Expected error
        }
        _ => panic!("Expected CircularDependency error"),
    }
}

#[test]
fn test_import_resolution() {
    // Create programs with imports
    let user_program = Program {
        package: Some(PackageDecl {
            path: PackagePath {
                segments: vec!["com".to_string(), "example".to_string(), "user".to_string()],
                span: Span::dummy(),
            },
            span: Span::dummy(),
        }),
        imports: vec![],
        definitions: vec![Definition::Schema(SchemaDef {
            name: "User".to_string(),
            fields: vec![
                SchemaField {
                    name: "id".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::Primitive(PrimitiveType::String),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                },
                SchemaField {
                    name: "name".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::Primitive(PrimitiveType::String),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                },
            ],
            span: Span::dummy(),
        })],
        span: Span::dummy(),
    };

    let post_program = Program {
        package: Some(PackageDecl {
            path: PackagePath {
                segments: vec!["com".to_string(), "example".to_string(), "post".to_string()],
                span: Span::dummy(),
            },
            span: Span::dummy(),
        }),
        imports: vec![ImportStmt {
            path: "user.glass".to_string(),
            span: Span::dummy(),
        }],
        definitions: vec![Definition::Schema(SchemaDef {
            name: "Post".to_string(),
            fields: vec![
                SchemaField {
                    name: "id".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::Primitive(PrimitiveType::String),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                },
                SchemaField {
                    name: "author".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::SchemaRef(SchemaRef {
                            package: Some(PackagePath {
                                segments: vec![
                                    "com".to_string(),
                                    "example".to_string(),
                                    "user".to_string(),
                                ],
                                span: Span::dummy(),
                            }),
                            name: "User".to_string(),
                            span: Span::dummy(),
                        }),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                },
            ],
            span: Span::dummy(),
        })],
        span: Span::dummy(),
    };

    // Build a type tree from the programs with file paths
    let type_tree = TypeTree::from_programs_with_paths(&[
        (user_program, Some("user.glass".to_string())),
        (post_program, Some("post.glass".to_string())),
    ])
    .unwrap();

    // Check that the Post type depends on the User type
    let dependencies = type_tree.get_dependencies("com.example.post.Post").unwrap();
    assert!(dependencies.contains("com.example.user.User"));

    // Check that the User type is accessible from the Post file
    assert!(type_tree.is_type_accessible("post.glass", "com.example.user.User"));
}

#[test]
fn test_parse_and_build_type_tree() {
    // Parse a simple Glass program
    let source = r#"
    package com.example;

    schema User {
        id: string;
        name: string;
    }

    schema Post {
        id: string;
        author: User;
    }
    "#;

    let program = Parser::parse(source.to_string()).unwrap();

    // Build a type tree from the parsed program
    let type_tree = TypeTree::from_programs(&[program]).unwrap();

    // Check that the types exist in the tree
    assert!(type_tree.has_type("com.example.User"));
    assert!(type_tree.has_type("com.example.Post"));

    // Check that the Post type depends on the User type
    let dependencies = type_tree.get_dependencies("com.example.Post").unwrap();
    assert!(dependencies.contains("com.example.User"));
}

#[test]
fn test_get_all_affected_types() {
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

    let comment_program = create_test_program(
        "com.example",
        "Comment",
        vec![
            ("id", Type::Primitive(PrimitiveType::String)),
            (
                "post",
                Type::SchemaRef(SchemaRef {
                    package: Some(PackagePath {
                        segments: vec!["com".to_string(), "example".to_string()],
                        span: Span::dummy(),
                    }),
                    name: "Post".to_string(),
                    span: Span::dummy(),
                }),
            ),
        ],
    );

    // Build a type tree from the programs
    let type_tree =
        TypeTree::from_programs(&[user_program, post_program, comment_program]).unwrap();

    // Get all affected types for User
    let affected_types = type_tree.get_all_affected_types("com.example.User");
    assert!(affected_types.contains("com.example.User"));
    assert!(affected_types.contains("com.example.Post"));
    assert!(affected_types.contains("com.example.Comment"));

    // Get all affected types for Post
    let affected_types = type_tree.get_all_affected_types("com.example.Post");
    assert!(affected_types.contains("com.example.Post"));
    assert!(affected_types.contains("com.example.Comment"));
    assert!(!affected_types.contains("com.example.User"));

    // Get all affected types for Comment
    let affected_types = type_tree.get_all_affected_types("com.example.Comment");
    assert!(affected_types.contains("com.example.Comment"));
    assert!(!affected_types.contains("com.example.Post"));
    assert!(!affected_types.contains("com.example.User"));
}

#[test]
fn test_get_reachable_types() {
    // Create programs with imports
    let user_program = Program {
        package: Some(PackageDecl {
            path: PackagePath {
                segments: vec!["com".to_string(), "example".to_string(), "user".to_string()],
                span: Span::dummy(),
            },
            span: Span::dummy(),
        }),
        imports: vec![],
        definitions: vec![
            Definition::Schema(SchemaDef {
                name: "User".to_string(),
                fields: vec![
                    SchemaField {
                        name: "id".to_string(),
                        field_type: TypeWithSpan {
                            type_value: Type::Primitive(PrimitiveType::String),
                            span: Span::dummy(),
                        },
                        span: Span::dummy(),
                    },
                    SchemaField {
                        name: "name".to_string(),
                        field_type: TypeWithSpan {
                            type_value: Type::Primitive(PrimitiveType::String),
                            span: Span::dummy(),
                        },
                        span: Span::dummy(),
                    },
                ],
                span: Span::dummy(),
            }),
            Definition::Schema(SchemaDef {
                name: "Profile".to_string(),
                fields: vec![
                    SchemaField {
                        name: "bio".to_string(),
                        field_type: TypeWithSpan {
                            type_value: Type::Primitive(PrimitiveType::String),
                            span: Span::dummy(),
                        },
                        span: Span::dummy(),
                    },
                    SchemaField {
                        name: "user".to_string(),
                        field_type: TypeWithSpan {
                            type_value: Type::SchemaRef(SchemaRef {
                                package: Some(PackagePath {
                                    segments: vec![
                                        "com".to_string(),
                                        "example".to_string(),
                                        "user".to_string(),
                                    ],
                                    span: Span::dummy(),
                                }),
                                name: "User".to_string(),
                                span: Span::dummy(),
                            }),
                            span: Span::dummy(),
                        },
                        span: Span::dummy(),
                    },
                ],
                span: Span::dummy(),
            }),
        ],
        span: Span::dummy(),
    };

    let post_program = Program {
        package: Some(PackageDecl {
            path: PackagePath {
                segments: vec!["com".to_string(), "example".to_string(), "post".to_string()],
                span: Span::dummy(),
            },
            span: Span::dummy(),
        }),
        imports: vec![ImportStmt {
            path: "user.glass".to_string(),
            span: Span::dummy(),
        }],
        definitions: vec![Definition::Schema(SchemaDef {
            name: "Post".to_string(),
            fields: vec![
                SchemaField {
                    name: "id".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::Primitive(PrimitiveType::String),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                },
                SchemaField {
                    name: "author".to_string(),
                    field_type: TypeWithSpan {
                        type_value: Type::SchemaRef(SchemaRef {
                            package: Some(PackagePath {
                                segments: vec![
                                    "com".to_string(),
                                    "example".to_string(),
                                    "user".to_string(),
                                ],
                                span: Span::dummy(),
                            }),
                            name: "User".to_string(),
                            span: Span::dummy(),
                        }),
                        span: Span::dummy(),
                    },
                    span: Span::dummy(),
                },
            ],
            span: Span::dummy(),
        })],
        span: Span::dummy(),
    };

    // Build a type tree from the programs with file paths
    let type_tree = TypeTree::from_programs_with_paths(&[
        (user_program, Some("user.glass".to_string())),
        (post_program, Some("post.glass".to_string())),
    ])
    .unwrap();

    // Get reachable types from the user file
    let reachable_types = type_tree.get_reachable_types("user.glass");
    assert!(reachable_types.contains("com.example.user.User"));
    assert!(reachable_types.contains("com.example.user.Profile"));
    assert!(!reachable_types.contains("com.example.post.Post"));

    // Get reachable types from the post file
    let reachable_types = type_tree.get_reachable_types("post.glass");
    assert!(reachable_types.contains("com.example.post.Post"));
    assert!(reachable_types.contains("com.example.user.User"));
    assert!(reachable_types.contains("com.example.user.Profile"));
}
