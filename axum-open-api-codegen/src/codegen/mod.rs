use proc_macro2::Ident;
use quote::ToTokens;
use syn::{Type, Visibility, Path};
use crate::parsing::MethodType;

/// The root of the codegen tree.
pub struct Root {
    pub items: Vec<Item>,
}

/// An item; either a [`ModuleItem`], [`MethodItem`] or a [`syn::Item`].
pub enum Item {
    Module(ModuleItem),
    Method(MethodItem),
    Schema(syn::Item),
}

/// A module like `pub mod api { ... }`
pub struct ModuleItem {
    pub name: Ident,
    pub vis: Visibility,
    pub items: Vec<Item>,
}

/// A method like 
/// ```no_run
/// pub struct GetPosts { ... }
/// impl FromRequest for GetPosts { ... }
/// ```
pub struct MethodItem {
    /// The http method
    pub method_ty: MethodType,
    /// The http path
    pub axum_path: String,
    pub oapi_path: String,

    /// The name of the generated struct
    pub struct_name: Ident,
    /// The visibility of the generated struct
    pub struct_vis: Visibility,

    /// The path parameters
    pub path_param_names: Vec<Ident>,
    pub path_param_types: Vec<Type>,
    /// The query parameters
    pub query_param_names: Vec<Ident>,
    pub query_param_types: Vec<Type>,

    /// The body extractor
    pub extractor: Option<Extractor>,

    /// the oapi summary
    pub summary: Option<String>,
    /// the oapi description
    pub description: Option<String>,
}

/// An extractor, like `let Json(body) = req.extract().await?;`
pub struct Extractor {
    pub body_ty: Type,
    pub body_ident: Ident,
    pub extractor_ty: Type,
    pub rejection_var: Path,
}

impl ToTokens for Root {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self { items } = self;
        tokens.extend(quote! { #(#items)* });
    }
}

impl ToTokens for Item {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Module(mod_) => mod_.to_tokens(tokens),
            Self::Method(method) => method.to_tokens(tokens),
            Self::Schema(schema) => schema.to_tokens(tokens),
        }
    }
}

impl ToTokens for ModuleItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            vis,
            items,
        } = self;

        tokens.extend(quote! {
            #vis mod #name {
                #(#items)*
            }
        });
    }
}

impl ToTokens for Extractor {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            body_ty: _,
            body_ident,
            extractor_ty,
            rejection_var,
        } = self;
        tokens.extend(quote!{
            let #extractor_ty(#body_ident) = match req.extract().await {
                Ok(body) => body,
                Err(e) => return Err(::axum_open_api::Rejection::#rejection_var(e)),
            };
        });
    }
}

impl ToTokens for MethodItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            method_ty,
            axum_path,
            oapi_path,
            struct_name,
            struct_vis,
            summary,
            description,
            path_param_names: path_param_idents,
            path_param_types,
            query_param_names: query_param_idents,
            query_param_types,
            extractor,
        } = self;

        let struct_doc: String = format!(
            "
Generated from OpenAPI spec.
- Method: `{method_ty} {oapi_path}`
- Summary: {summary:?}
- Description: {description:?}
    "
        );

        let body_field = extractor.as_ref().map(|extractor| {
            let body_ty = &extractor.body_ty;
            let _extractor_ty = &extractor.extractor_ty;
            quote!(pub body: #body_ty,)
        });
        let body_ident = extractor.as_ref().map(|_| {
            quote!(body)
        });
        let (from_req_trait, from_req_fn_name, req_type, extract_parts) = match &extractor {
            Some(_) => ( quote!(FromRequest), quote!(from_request), quote!(::axum::extract::Request), quote!(extract_parts) ),
            None => ( quote!(FromRequestParts), quote!(from_request_parts), quote!(&mut ::axum::http::request::Parts), quote!(extract) ),
        };

        tokens.extend(quote! {

            // Generate the struct first
            #[doc = #struct_doc]
            #[derive(Debug)]
            #struct_vis struct #struct_name {
                #(pub #path_param_idents: #path_param_types,)*
                #(pub #query_param_idents: #query_param_types,)*
                #body_field // add the body field only if it is extracted
            }

            // Implement the OapiPath trait for it
            impl ::axum_open_api::OapiPath for #struct_name {
                fn path() -> &'static str {
                    #axum_path
                }

                fn method_router<H, T, S>(handler: H) -> axum::routing::MethodRouter<S>
                where
                    H: axum::handler::Handler<T, S>,
                    T: 'static,
                    S: Clone + Send + Sync + 'static,
                {
                    axum::routing::MethodRouter::new().#method_ty(handler)
                }
            }

            // Implement FromRequest(Parts)
            #[axum::async_trait]
            impl<S: Send + Sync> ::axum::extract::#from_req_trait<S> for #struct_name {
                type Rejection = ::axum_open_api::Rejection;
    
                async fn #from_req_fn_name(
                    mut req: #req_type,
                    _state: &S,
                ) -> Result<Self, Self::Rejection> {
                    use axum::{
                        extract::{Path, Query},
                        RequestPartsExt, RequestExt
                    };
    
                    let Path((#(#path_param_idents),*)) = match req.#extract_parts().await {
                        Ok(params) => params,
                        Err(e) => return Err(::axum_open_api::Rejection::Path(e)),
                    };
    
                    #[derive(serde::Deserialize)]
                    struct __QueryGenerated__ {
                        #(#query_param_idents: #query_param_types,)*
                    }
    
                    let Query(__QueryGenerated__ { #(#query_param_idents),* }) = match req.#extract_parts().await {
                        Ok(query) => query,
                        Err(e) => return Err(::axum_open_api::Rejection::Query(e)),
                    };

                    #extractor
    
                    Ok(Self {
                        #(#path_param_idents,)*
                        #(#query_param_idents,)*
                        #body_ident // add the body field only if it is extracted
                    })
                }
            }
        
        });
    }
}
