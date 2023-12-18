mod schema;
use crate::{codegen, parsing};
use oas3::Spec;
use proc_macro2::{Ident, Span};
use schema::{compile_param, compile_schema};
use syn::Item;

pub struct Compiler {
    spec: Spec,
}

impl Compiler {
    pub fn compile(parser: parsing::Root, spec: Spec) -> syn::Result<codegen::Root> {
        let mut this = Self { spec };

        // Compile the schemas
        let mut items = Vec::new();
        items.push(codegen::Item::Module(this.compile_schemas_from_spec()?));

        // And then the other items
        for item in parser.items {
            let mut schemas = Vec::new();
            items.push(this.compile_item(item, &mut schemas, 0)?);
            for schema in schemas {
                items.push(codegen::Item::Schema(schema));
            }
        }

        Ok(codegen::Root { items })
    }

    fn compile_schemas_from_spec(&mut self) -> syn::Result<codegen::ModuleItem> {
        let mut items = Vec::new();
        // panic!("{:#?}", self.spec.components.as_ref().unwrap().schemas.clone());
        for (name, schema) in self.spec.components.as_ref().unwrap().schemas.clone() {
            // The depth does not matter, because we discard the type anyway
            let _ = compile_schema(schema, Some(&name), 1, &mut items)?;
        }

        Ok(codegen::ModuleItem {
            vis: parse_quote!(pub),
            name: Ident::new("schemas", Span::call_site()),
            items: items.into_iter().map(codegen::Item::Schema).collect(),
        })
    }

    fn compile_item(
        &mut self,
        item: parsing::Item,
        schemas: &mut Vec<Item>,
        depth: usize,
    ) -> syn::Result<codegen::Item> {
        match item {
            parsing::Item::Method(method) => Ok(codegen::Item::Method(
                self.compile_method(method, depth, schemas)?,
            )),
            parsing::Item::Module(module) => {
                Ok(codegen::Item::Module(self.compile_module(module, depth)?))
            }
        }
    }

    fn compile_module(
        &mut self,
        module: parsing::ModuleItem,
        depth: usize,
    ) -> syn::Result<codegen::ModuleItem> {
        let mut items = Vec::new();
        let mut schemas = Vec::new();

        for item in module.items {
            items.push(self.compile_item(item, &mut schemas, depth + 1)?);
        }

        for schema in schemas {
            items.push(codegen::Item::Schema(schema));
        }

        Ok(codegen::ModuleItem {
            vis: module.vis,
            name: module.name,
            items,
        })
    }

    fn compile_method(
        &mut self,
        method: parsing::MethodItem,
        depth: usize,
        schemas: &mut Vec<Item>,
    ) -> syn::Result<codegen::MethodItem> {
        let path_item = self
            .spec
            .paths
            .get(&method.path.to_oapi_path())
            .ok_or_else(|| err!(&method.path, "Path not found in OpenAPI spec"))?;

        let operation = match method.method_ty {
            parsing::MethodType::Get(_) => path_item.get.as_ref(),
            parsing::MethodType::Post(_) => path_item.post.as_ref(),
            parsing::MethodType::Put(_) => path_item.put.as_ref(),
            parsing::MethodType::Delete(_) => path_item.delete.as_ref(),
            parsing::MethodType::Patch(_) => path_item.patch.as_ref(),
            parsing::MethodType::Head(_) => path_item.head.as_ref(),
            parsing::MethodType::Trace(_) => path_item.trace.as_ref(),
            parsing::MethodType::Options(_) => path_item.options.as_ref(),
        }
        .ok_or_else(|| err!(&method.path, "Method not found in OpenAPI spec"))?;

        // Get the path parameters
        let mut path_param_types = Vec::new();
        for param_ident in method.path.path_param_idents() {
            // 1. Find it in the spec
            let path_param = operation
                .parameters
                .iter()
                .map(|p| p.resolve(&self.spec).unwrap())
                .find(|p| param_ident == p.name)
                .ok_or_else(|| {
                    err!(param_ident, "Path parameter {param_ident} not found in OpenAPI spec")
                })?;
            // 2. Check that it's a path parameter
            if path_param.location != "path" {
                return Err(err!(
                    param_ident,
                    "Path parameter {param_ident} is not in: `path` in OpenAPI spec"
                ));
            }
            // 3. Add it to the schema map
            path_param_types.push(compile_param(path_param, depth, schemas)?);
        }

        // Get the query parameters
        let mut query_param_names = Vec::new();
        let mut query_param_types = Vec::new();
        for query_param in operation
            .parameters
            .iter()
            .map(|p| p.resolve(&self.spec).unwrap())
            .filter(|p| p.location == "query")
        {
            query_param_names.push(Ident::new(&query_param.name, Span::call_site()));
            query_param_types.push(compile_param(query_param, depth, schemas)?);
        }

        // Get the body-extractor if it exists
        let extractor = if let Some(req_body) =
            operation.request_body.as_ref().map(|b| b.resolve(&self.spec).unwrap())
        {
            if req_body.content.len() != 1 {
                return Err(err_call_site!("Exactly one media type is supported: \n{req_body:#?}"));
            }
            let (media_type_name, media_type) = req_body.content.first_key_value().unwrap();
            let media_schema = media_type.schema.clone().ok_or_else(|| {
                err_call_site!("Schema not found in media type: \n{media_type:#?}")
            })?;
            Some(
                match (
                    media_type_name.split('/').next().unwrap(),
                    media_type_name.split('/').last().unwrap(),
                ) {
                    ("application", "json") => {
                        let body_ty = compile_schema(media_schema, None, depth, schemas)?;
                        codegen::Extractor {
                            body_ident: parse_quote!(body),
                            extractor_ty: parse_quote!(::axum::extract::Json),
                            rejection_var: parse_quote!(Json),
                            body_ty,
                        }
                    }
                    ("application", "x-www-form-urlencoded") => {
                        let body_ty = compile_schema(media_schema, None, depth, schemas)?;
                        codegen::Extractor {
                            body_ident: parse_quote!(body),
                            extractor_ty: parse_quote!(::axum::extract::Form),
                            rejection_var: parse_quote!(Form),
                            body_ty,
                        }
                    }
                    ("multipart", "form-data") => codegen::Extractor {
                        body_ident: parse_quote!(body),
                        extractor_ty: parse_quote!(::axum::extract::Multipart),
                        rejection_var: parse_quote!(Multipart),
                        body_ty: parse_quote!(::axum::extract::Multipart),
                    },
                    ("text", _) => codegen::Extractor {
                        body_ident: parse_quote!(body),
                        extractor_ty: parse_quote!(::axum::extract::Text),
                        rejection_var: parse_quote!(Text),
                        body_ty: parse_quote!(::axum::extract::Text),
                    },
                    _ => codegen::Extractor {
                        body_ident: parse_quote!(body),
                        extractor_ty: parse_quote!(::axum::extract::Bytes),
                        rejection_var: parse_quote!(Bytes),
                        body_ty: parse_quote!(::axum::extract::Bytes),
                    },
                },
            )
        } else {
            None
        };

        Ok(codegen::MethodItem {
            method_ty: method.method_ty,
            axum_path: method.path.to_axum_path(),
            oapi_path: method.path.to_oapi_path(),
            struct_name: method.struct_name,
            struct_vis: method.struct_vis,
            path_param_names: method.path.path_param_idents().collect(),
            path_param_types,
            query_param_names,
            query_param_types,
            extractor,         // todo
            summary: None,     // todo
            description: None, // todo
        })
    }
}

impl parsing::MethodPath {
    pub fn to_axum_path(&self) -> String {
        let mut path = String::new();

        for (ident, is_param) in &self.0 {
            if *is_param {
                path.push_str(&format!("/:{}", ident));
            } else {
                path.push_str(&format!("/{}", ident));
            }
        }

        path
    }

    pub fn to_oapi_path(&self) -> String {
        let mut path = String::new();

        for (ident, is_param) in &self.0 {
            if *is_param {
                path.push_str(&format!("/{{{}}}", ident));
            } else {
                path.push_str(&format!("/{}", ident));
            }
        }

        path
    }

    fn path_param_idents(&self) -> impl Iterator<Item = Ident> + '_ {
        self.0
            .iter()
            .filter_map(|(ident, is_param)| if *is_param { Some(ident.clone()) } else { None })
    }
}
