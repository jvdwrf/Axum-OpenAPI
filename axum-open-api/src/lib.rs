use axum::{
    extract::rejection::{
        BytesRejection, FormRejection, JsonRejection, PathRejection, QueryRejection,
        StringRejection,
    },
    handler::Handler,
    response::{IntoResponse, Response},
    routing::MethodRouter,
    Router,
};
pub use axum_open_api_codegen::validate_routes;

//-------------------------------------
// library
//-------------------------------------

pub trait OapiRouter {
    type State: Clone + Send + Sync + 'static;

    fn oapi_route<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, Self::State>,
        T: 'static + OapiPath;
}

impl<S: Send + Sync + Clone + 'static> OapiRouter for Router<S> {
    type State = S;

    fn oapi_route<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static + OapiPath,
    {
        self.route(T::path(), T::method_router(handler))
    }
}

pub trait OapiPath {
    fn path() -> &'static str;
    fn method_router<H, T, S>(handler: H) -> MethodRouter<S>
    where
        H: Handler<T, S>,
        T: 'static,
        S: Clone + Send + Sync + 'static;
}

macro_rules! impl_oapi_path_for {
    ($($os:ident),*) => {
        impl<P: OapiPath, $($os),*> OapiPath for ($($os,)* P,) {
            fn path() -> &'static str {
                P::path()
            }

            fn method_router<H, T, S>(handler: H) -> MethodRouter<S>
            where
                H: Handler<T, S>,
                T: 'static,
                S: Clone + Send + Sync + 'static,
            {
                P::method_router(handler)
            }
        }
    };
}

impl_oapi_path_for!();
impl_oapi_path_for!(O1);
impl_oapi_path_for!(O1, O2);
impl_oapi_path_for!(O1, O2, O3);
impl_oapi_path_for!(O1, O2, O3, O4);
impl_oapi_path_for!(O1, O2, O3, O4, O5);
impl_oapi_path_for!(O1, O2, O3, O4, O5, O6);
impl_oapi_path_for!(O1, O2, O3, O4, O5, O6, O7);
impl_oapi_path_for!(O1, O2, O3, O4, O5, O6, O7, O8);
impl_oapi_path_for!(O1, O2, O3, O4, O5, O6, O7, O8, O9);
impl_oapi_path_for!(O1, O2, O3, O4, O5, O6, O7, O8, O9, O10);

#[derive(Debug)]
pub enum Rejection {
    Query(QueryRejection),
    Path(PathRejection),
    Json(JsonRejection),
    Form(FormRejection),
    String(StringRejection),
    Bytes(BytesRejection),
    Other(Box<dyn DynRejection>),
}

macro_rules! rejection_from {
    ($ty:ty, $var:ident) => {
        impl From<$ty> for Rejection {
            fn from(e: $ty) -> Self {
                Rejection::$var(e)
            }
        }
    };
}

rejection_from!(QueryRejection, Query);
rejection_from!(PathRejection, Path);
rejection_from!(JsonRejection, Json);
rejection_from!(FormRejection, Form);
rejection_from!(StringRejection, String);
rejection_from!(BytesRejection, Bytes);
rejection_from!(Box<dyn DynRejection>, Other);

pub trait DynRejection: IntoResponse + std::fmt::Debug + Send + Sync + 'static {
    fn boxed_into_response(self: Box<Self>) -> Response;
}
impl<T> DynRejection for T
where
    T: IntoResponse + std::fmt::Debug + Send + Sync + 'static,
{
    fn boxed_into_response(self: Box<Self>) -> Response {
        self.into_response()
    }
}

impl IntoResponse for Rejection {
    fn into_response(self) -> Response {
        match self {
            Rejection::Query(e) => e.into_response(),
            Rejection::Path(e) => e.into_response(),
            Rejection::Json(e) => e.into_response(),
            Rejection::Form(e) => e.into_response(),
            Rejection::String(e) => e.into_response(),
            Rejection::Bytes(e) => e.into_response(),
            Rejection::Other(e) => e.boxed_into_response(),
        }
    }
}
