use crate::prelude::*;
use glass_parser::ast::schema::Schema;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn generate_schema(schema: &Schema) -> TokenStream {
    let schema_name = format_ident!("{}", schema.name);

    let mut fields = Vec::new();
    for field in &schema.fields {
        let field_name = format_ident!("{}", field.name);
        let field_type = crate::generator::util::convert_ast_type_to_rust_type(&field.ty);
        let field_type: TokenStream = field_type.parse().unwrap();

        let generated = quote! {
            pub #field_name: #field_type,
        };

        fields.push(generated);
    }

    let generated = quote! {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub struct #schema_name {
            #(#fields)*
        }
    };

    generated
}
