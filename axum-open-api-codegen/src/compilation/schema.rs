use convert_case::{Case, Casing};
use oas3::{
    spec::{ObjectOrReference, Parameter, SchemaType},
    Schema,
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use std::iter::repeat;
use syn::{Item, Type};

pub fn compile_param(param: Parameter, depth: usize, items: &mut Vec<Item>) -> syn::Result<Type> {
    let Some(schema) = param.schema else {
        return Err(err_call_site!(
            "Query parameter does not have a schema in OpenAPI spec: \n{param:#?}"
        ));
    };

    let ty = compile_schema(ObjectOrReference::Object(schema), None, depth, items)?;

    match param.required {
        Some(true) => Ok(ty),
        _ => Ok(parse_quote!(Option<#ty>)),
    }
}

/// Returns the type, while recursively compiling all schemas and adding any new types to the items
pub fn compile_schema(
    schema_ref: ObjectOrReference<Schema>,
    title: Option<&str>,
    depth: usize,
    items: &mut Vec<Item>,
) -> syn::Result<Type> {
    // If it's a reference, we can just use that as our type, and don't have to create an item
    let schema = match schema_ref {
        ObjectOrReference::Ref { ref_path } => return compile_schema_ref(&ref_path, depth),
        ObjectOrReference::Object(schema) => schema,
    };

    // We don't support `all_of`` or `any_of``
    if !schema.all_of.is_empty() || !schema.any_of.is_empty() {
        return Err(err_call_site!(
            "allOf and anyOf are not (yet) supported: \n{schema:#?}"
        ));
    }

    // handle `oneOf` by generating an enum
    if !schema.one_of.is_empty() {
        return compile_one_of(schema, title, depth, items);
    }

    // If it is not `oneOf`, `schema_type` must be set
    let schema_type = schema.schema_type.as_ref().ok_or(err_call_site!(
        "schema {schema:#?} is missing `schema_type` and `one_of`"
    ))?;

    // Now we go on to calculate the schema types
    match schema_type {
        SchemaType::Object => compile_object(schema, title, depth, items),
        SchemaType::Array => compile_array(schema, title, depth, items),
        SchemaType::String => compile_base_type(parse_quote!(String), title, &schema, items),
        SchemaType::Number => compile_base_type(parse_quote!(f64), title, &schema, items),
        SchemaType::Integer => compile_base_type(parse_quote!(i64), title, &schema, items),
        SchemaType::Boolean => compile_base_type(parse_quote!(bool), title, &schema, items),
    }
}

fn compile_schema_ref(ref_path: &str, depth: usize) -> syn::Result<Type> {
    let depth_prefix = repeat(quote!(super::)).take(depth).collect::<TokenStream>();
    let ref_name = ref_path.split('/').last().unwrap();
    let ident = Ident::new(ref_name, Span::call_site());
    Ok(parse_quote!(#depth_prefix schemas::#ident))
}

fn compile_one_of(
    schema: Schema,
    title: Option<&str>,
    depth: usize,
    items: &mut Vec<Item>,
) -> syn::Result<Type> {
    let ident = try_merge_titles(title, &schema)?;

    // First we collect all the variants
    let mut variants = Vec::new();
    for variant_schema in schema.one_of {
        let variant_ty = compile_schema(variant_schema, None, depth, items)?;
        let variant_ident = Ident::new(
            &variant_ty
                .to_token_stream()
                .to_string()
                .split("::")
                .last()
                .unwrap()
                .to_case(Case::UpperCamel),
            Span::call_site(),
        );
        variants.push(quote! { #variant_ident(#variant_ty) });
    }

    items.push(parse_quote! {
        /// Generated from OpenAPI schema
        #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
        pub enum #ident {
            #(#variants),*
        }
    });

    Ok(parse_quote!(#ident))
}

fn compile_array(
    schema: Schema,
    title: Option<&str>,
    depth: usize,
    items: &mut Vec<Item>,
) -> syn::Result<Type> {
    let merged_title = try_merge_titles(title, &schema);

    // if title == Some("NestedInlineSchema") {
    //     panic!()
    // }

    let item_ty = compile_schema(
        *schema
            .items
            .ok_or_else(|| err_call_site!("Array must contain `items` field"))?,
        None,
        depth,
        items,
    )?;
    let ty = parse_quote!(Vec<#item_ty>);
    match merged_title {
        Ok(ident) => {
            items.push(parse_quote! {
                /// Generated from OpenAPI schema
                pub type #ident = #ty;
            });
            Ok(parse_quote!(#ident))
        }
        Err(_) => Ok(ty),
    }
}

fn compile_object(
    schema: Schema,
    title: Option<&str>,
    depth: usize,
    items: &mut Vec<Item>,
) -> syn::Result<Type> {
    let ident = try_merge_titles(title, &schema)?;

    // First we parse all the fields
    let mut fields: Vec<TokenStream> = Vec::new();
    for (prop_name, prop_schema) in schema.properties {
        let prop_name = Ident::new(&prop_name, Span::call_site());
        let prop_ty = compile_schema(prop_schema, None, depth, items)?;
        // If the property is required, we don't wrap it in an Option
        if schema.required.contains(&prop_name.to_string()) {
            fields.push(quote! { pub #prop_name: #prop_ty});
        } else {
            fields.push(quote! { pub #prop_name: Option<#prop_ty>});
        }
    }

    items.push(parse_quote! {
        /// Generated from OpenAPI schema
        #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
        pub struct #ident {
            #(#fields),*
        }
    });

    Ok(parse_quote!(#ident))
}

fn compile_base_type(
    ty: Type,
    title: Option<&str>,
    schema: &Schema,
    items: &mut Vec<Item>,
) -> syn::Result<Type> {
    match try_merge_titles(title, schema) {
        // If the schema has a title, we create an alias
        Ok(ident) => {
            items.push(parse_quote! {
                /// Generated from OpenAPI schema
                pub type #ident = #ty;
            });
            Ok(parse_quote!(#ident))
        }
        // In this case, it's just like a reference, but to a base type.
        // Therefore, we don't need to create any type for it.
        Err(_) => Ok(ty),
    }
}

fn try_merge_titles(title: Option<&str>, schema: &Schema) -> syn::Result<Ident> {
    let title = title.or(schema.title.as_deref()).ok_or_else(|| {
        err_call_site!(
            "Anonymous schema's are not supported: 
Please add a `title`, or create a schema in `components/schemas` and reference it.
{schema:#?}"
        )
    })?;
    Ok(Ident::new(title, Span::call_site()))
}
