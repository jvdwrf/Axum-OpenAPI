use proc_macro2::{Ident, Span};
use quote::ToTokens;

use syn::{
    parse::{Parse, ParseStream},
    token::{As, Brace, Mod},
    LitStr, Visibility,
};

/// The root of the parser.
#[derive(Debug)]
pub struct Root {
    pub spec_path: LitStr,
    pub items: Vec<Item>,
}

/// An item; either a [`ModuleItem`] or a [`MethodItem`].
#[derive(Debug)]
pub enum Item {
    Module(ModuleItem),
    Method(MethodItem),
}

/// A module like `pub mod api { ... }`
#[derive(Debug)]
pub struct ModuleItem {
    pub vis: Visibility,
    pub name: Ident,
    pub items: Vec<Item>,
}

/// A method like `GET /api/feed/get_posts as pub GetPosts;`
#[derive(Debug)]
pub struct MethodItem {
    pub method_ty: MethodType,
    pub path: MethodPath,
    pub struct_vis: Visibility,
    pub struct_name: Ident,
}

/// A path like `/api/{id}/test/{name}`
#[derive(Debug, Clone)]
pub struct MethodPath(pub Vec<(Ident, bool)>);

/// The type of a method, like `GET` or `POST`.
#[derive(Debug, Clone, Copy)]
pub enum MethodType {
    Get(Span),
    Post(Span),
    Put(Span),
    Delete(Span),
    Patch(Span),
    Head(Span),
    Options(Span),
    Trace(Span),
}

/// Keywords used in the parser.
pub mod kw {
    custom_keyword!(GET);
    custom_keyword!(POST);
    custom_keyword!(PUT);
    custom_keyword!(DELETE);
    custom_keyword!(PATCH);
    custom_keyword!(HEAD);
    custom_keyword!(OPTIONS);
    custom_keyword!(TRACE);
    custom_keyword!(path);
}

impl Parse for Item {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(module) = input.parse::<ModuleItem>() {
            return Ok(Self::Module(module));
        }
        Ok(Self::Method(input.parse::<MethodItem>()?))
    }
}

impl Parse for ModuleItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis = input.parse::<Visibility>()?;
        input.parse::<Mod>()?;
        let name = input.parse::<Ident>()?;
        let inner;
        braced!(inner in input);

        let mut items = Vec::new();
        while !inner.is_empty() {
            items.push(inner.parse::<Item>()?);
        }

        Ok(Self { vis, name, items })
    }
}

impl Parse for MethodItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ty = input.parse::<MethodType>()?;
        let path = input.parse::<MethodPath>()?;

        input.parse::<As>()?;
        let struct_vis = input.parse::<Visibility>()?;
        let name = input.parse::<Ident>()?;
        input.parse::<Token![;]>()?;

        Ok(Self {
            method_ty: ty,
            path,
            struct_vis,
            struct_name: name,
        })
    }
}

impl Parse for Root {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the path to the OpenAPI spec
        input.parse::<kw::path>()?;
        input.parse::<Token![=]>()?;
        let spec_path = input.parse::<LitStr>()?;
        input.parse::<Token![;]>()?;

        let mut items = Vec::new();
        while let Ok(item) = input.parse::<Item>() {
            items.push(item);
        }

        Ok(Self { spec_path, items })
    }
}

impl Parse for MethodPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut segments = Vec::new();
        while input.parse::<Token![/]>().is_ok() {
            if input.peek(Brace) {
                let inner;
                braced!(inner in input);
                let ident = inner.parse::<Ident>()?;
                segments.push((ident, true));
            } else {
                let ident = input.parse::<Ident>()?;
                segments.push((ident, false));
            }
        }
        Ok(Self(segments))
    }
}

impl Parse for MethodType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        match ident.to_string().as_str() {
            "GET" => Ok(MethodType::Get(ident.span())),
            "POST" => Ok(MethodType::Post(ident.span())),
            "PUT" => Ok(MethodType::Put(ident.span())),
            "DELETE" => Ok(MethodType::Delete(ident.span())),
            "PATCH" => Ok(MethodType::Patch(ident.span())),
            _ => Err(syn::Error::new(ident.span(), "Invalid method")),
        }
    }
}

impl std::fmt::Display for MethodType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodType::Get(_) => write!(f, "GET"),
            MethodType::Post(_) => write!(f, "POST"),
            MethodType::Put(_) => write!(f, "PUT"),
            MethodType::Delete(_) => write!(f, "DELETE"),
            MethodType::Patch(_) => write!(f, "PATCH"),
            MethodType::Head(_) => write!(f, "HEAD"),
            MethodType::Options(_) => write!(f, "OPTIONS"),
            MethodType::Trace(_) => write!(f, "TRACE"),
        }
    }
}

impl ToTokens for MethodType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            MethodType::Get(span) => tokens.extend(quote_spanned!(*span => get)),
            MethodType::Post(span) => tokens.extend(quote_spanned!(*span => post)),
            MethodType::Put(span) => tokens.extend(quote_spanned!(*span => put)),
            MethodType::Delete(span) => tokens.extend(quote_spanned!(*span => delete)),
            MethodType::Patch(span) => tokens.extend(quote_spanned!(*span => patch)),
            MethodType::Head(span) => tokens.extend(quote_spanned!(*span => head)),
            MethodType::Options(span) => tokens.extend(quote_spanned!(*span => options)),
            MethodType::Trace(span) => tokens.extend(quote_spanned!(*span => trace)),
        }
    }
}

impl MethodPath {
    pub fn span(&self) -> Span {
        self.0.first().unwrap().0.span()
    }
}
