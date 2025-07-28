use glass_parser::ast::types::{OptionType, PrimitiveType, Type, VectorType};

pub fn convert_ast_type_to_rust_type(ast_type: &Type) -> String {
    match ast_type {
        Type::Primitive(primitive) => convert_ast_primitive_to_string(primitive),
        Type::Option(option) => convert_ast_option_to_string(option),
        Type::Vector(vector) => convert_ast_vector_to_string(vector),
        Type::Schema(schema_ref) => schema_ref.0.to_owned(),
    }
}

fn convert_ast_primitive_to_string(primitive_type: &PrimitiveType) -> String {
    match primitive_type {
        PrimitiveType::String => "String".to_string(),
        PrimitiveType::U8 => "u8".to_string(),
        PrimitiveType::U16 => "u16".to_string(),
        PrimitiveType::U32 => "u32".to_string(),
        PrimitiveType::U64 => "u64".to_string(),
        PrimitiveType::U128 => "u128".to_string(),
        PrimitiveType::I8 => "i8".to_string(),
        PrimitiveType::I16 => "i16".to_string(),
        PrimitiveType::I32 => "i32".to_string(),
        PrimitiveType::I64 => "i64".to_string(),
        PrimitiveType::I128 => "i128".to_string(),
        PrimitiveType::F32 => "f32".to_string(),
        PrimitiveType::F64 => "f64".to_string(),
        PrimitiveType::Bool => "bool".to_string(),
    }
}

fn convert_ast_option_to_string(option_type: &OptionType) -> String {
    let inner_type = convert_ast_type_to_rust_type(&option_type.inner);
    format!("Option<{inner_type}>")
}

fn convert_ast_vector_to_string(vector_type: &VectorType) -> String {
    let inner_type = convert_ast_type_to_rust_type(&vector_type.inner);
    format!("Vec<{inner_type}>")
}
