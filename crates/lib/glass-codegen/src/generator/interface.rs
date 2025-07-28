use glass_parser::ast::interface::{Function, FunctionParam, FunctionReturn, Interface};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn generate_interface(interface: &Interface) -> TokenStream {
    let interface_name = format_ident!("{}", interface.name);
    let generated_associated_types = generated_associated_types(&interface.functions);
    let generated_functions = generate_functions(&interface.functions);

    let generated = quote! {
        #[async_trait::async_trait]
        pub trait #interface_name {
            #(#generated_associated_types)*

            #(#generated_functions)*
        }
    };

    generated
}

fn generated_associated_types(functions: &[Function]) -> Vec<TokenStream> {
    let mut generated_associated_types = Vec::new();

    let error_type = quote! {
        type Error: Send + Sync + serde::Serialize + serde::Deserialize<'static> + 'static;
    };
    generated_associated_types.push(error_type);

    let has_input_streams = functions
        .iter()
        .any(|f| matches!(f.param, FunctionParam::Stream(_)));

    let has_output_streams = functions
        .iter()
        .any(|f| matches!(f.return_type, Some(FunctionReturn::Stream(_))));

    if has_input_streams {
        let generated = quote! {
            type InputStream<T>: futures::stream::Stream<Item = T> + Send + Sync
            where
                T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync;
        };
        generated_associated_types.push(generated);
    }

    if has_output_streams {
        let generated = quote! {
            type OutputStream<T>: futures::stream::Stream<Item = T> + Send + Sync
            where
                T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync;
        };
        generated_associated_types.push(generated);
    }

    generated_associated_types
}

fn generate_functions(functions: &[Function]) -> Vec<TokenStream> {
    let mut generated_functions = Vec::with_capacity(functions.len());
    for function in functions {
        let function_name = format_ident!("{}", function.name);

        let generated_param = match &function.param {
            FunctionParam::Stream(inner_type) => {
                let inner_type_name =
                    crate::generator::util::convert_ast_type_to_rust_type(inner_type);
                let inner_type_ident: TokenStream = inner_type_name.parse().unwrap();
                quote! {
                    &self, request: Self::InputStream<#inner_type_ident>,
                }
            }
            FunctionParam::Simple(inner) => {
                let inner_type_name = crate::generator::util::convert_ast_type_to_rust_type(inner);
                let inner_type_ident: TokenStream = inner_type_name.parse().unwrap();
                quote! {
                    &self, request: #inner_type_ident
                }
            }
        };

        let generated_return = if let Some(return_type) = &function.return_type {
            match return_type {
                FunctionReturn::Stream(inner_type) => {
                    let inner_type_name =
                        crate::generator::util::convert_ast_type_to_rust_type(inner_type);
                    let inner_type_ident: TokenStream = inner_type_name.parse().unwrap();
                    quote! {
                        Result<Self::OutputStream<#inner_type_ident>, Self::Error>
                    }
                }
                FunctionReturn::Simple(inner_type) => {
                    let inner_type_name =
                        crate::generator::util::convert_ast_type_to_rust_type(inner_type);
                    let inner_type_ident: TokenStream = inner_type_name.parse().unwrap();
                    quote! {
                        Result<#inner_type_ident, Self::Error>
                    }
                }
            }
        } else {
            quote! {
                Result<(), Self::Error>
            }
        };

        let where_clauses = generate_where_clauses(function);
        let generated = if where_clauses.is_empty() {
            quote! {
                async fn #function_name(#generated_param) -> #generated_return;
            }
        } else {
            quote! {
                async fn #function_name(#generated_param) -> #generated_return
                where
                    #(#where_clauses),*;
            }
        };

        generated_functions.push(generated);
    }

    generated_functions
}

fn generate_where_clauses(function: &Function) -> Vec<TokenStream> {
    let mut where_clauses = Vec::new();

    // Add bounds for simple parameter types
    if let FunctionParam::Simple(param_type) = &function.param {
        let type_name = crate::generator::util::convert_ast_type_to_rust_type(param_type);
        let type_ident = format_ident!("{}", type_name);
        where_clauses.push(quote! {
            #type_ident: serde::Serialize + serde::de::DeserializeOwned + Send + Sync
        });
    }

    // Add bounds for simple return types
    if let Some(FunctionReturn::Simple(return_type)) = &function.return_type {
        let type_name = crate::generator::util::convert_ast_type_to_rust_type(return_type);
        let type_ident = format_ident!("{}", type_name);
        where_clauses.push(quote! {
            #type_ident: serde::Serialize + serde::de::DeserializeOwned + Send + Sync
        });
    }

    where_clauses
}
