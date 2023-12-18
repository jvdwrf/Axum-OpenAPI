#[macro_use]
extern crate syn;
#[macro_use]
extern crate quote;
#[macro_use]
mod err;
mod codegen;
mod compilation;
mod parsing;
use compilation::Compiler;
use parsing::Root;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use std::fs::File;

/// # OpenAPI Codegen
/// This macro generates code for Axum from an OpenAPI spec.
///
/// ## Supported
/// - Basic datatypes: string, number, integer, boolean, array, object
/// - required
/// - oneOf (enums)
/// - named components
/// - path parameters
/// - MIME extractors: application/json, application/x-www-form-urlencoded, text/*,
/// multipart/form-data. Everything else is treated as bytes.
/// - Http methods: POST, GET, PUT, DELETE, PATCH, HEAD, TRACE, OPTIONS
///
/// ## Not supported
/// - additionalProperties (yet)
/// - allOf, anyOf
///
/// ## Note
/// - Anonymous schemas must have a title
/// - Requires crate `axum` in path (v0.9 is supported)
///
/// # Example
/// See crate documentation of `axum-open-api` for examples
#[proc_macro]
pub fn validate_routes(item: TokenStream) -> TokenStream {
    fn _validate_routes(item: Root) -> syn::Result<TokenStream2> {
        // Working directory of cargo and rust-analyzer is different.
        // This is a hack to get around that, and have it work with both.
        let spec_path_str = item.spec_path.value();
        let file: File = match File::open(&spec_path_str) {
            Ok(file) => file,
            Err(_) => File::open(format!("../{spec_path_str}")).map_err(|_| {
                err!(item.spec_path, "File does not exist at path: {spec_path_str}")
            })?,
        };
        let spec = oas3::from_reader(file)
            .map_err(|e| err!(item.spec_path, "Could not parse OpenAPI spec: {e}"))?;

        let compiler = Compiler::compile(item, spec)?;

        Ok(compiler.into_token_stream())
    }

    match _validate_routes(parse_macro_input!(item as Root)) {
        Ok(code) => code.into_token_stream().into(),
        Err(e) => e.into_compile_error().into(),
    }
}
