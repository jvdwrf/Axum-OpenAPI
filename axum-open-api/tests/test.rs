use axum_open_api_codegen::validate_routes;
use schemas::NestedInlineObject;
validate_routes!(
    path = "axum-open-api/tests/test-api.yaml";

    // pub mod feed {
    //     GET     /api/feed/get_posts         as pub GetPosts;
    //     GET     /api/feed/get_tags          as pub GetTags;
    //     POST    /api/feed/add_tag           as pub AddTag;
    //     DELETE  /api/feed/remove_tag        as pub RemoveTag;
    // }

    // pub mod account {
    //     POST    /api/account/create         as pub CreateAccount;
    // }
);

#[test]
fn type_alias_schemas() {
    let _: String = schemas::StringAlias::from("hello");
    let _: i64 = schemas::IntegerAlias::from(42);
    let _: bool = schemas::BooleanAlias::from(true);
    let _: f64 = schemas::NumberAlias::from(0.1);
    let _: Vec<String> = schemas::StringVectorAlias::from(vec!["hello".to_string()]);
}

#[test]
fn object_schema() {
    let _ = schemas::ObjectSchema {
        id: Some(10),
        req_id: 20,
        name_ref: Some("hello".to_string()),
        inline_object: Some(NestedInlineObject { id: Some(10) }),
    };
}

#[test]
fn one_of_schema() {
    let _ = schemas::OneOfSchema::NumberTitle(1.0);
    let _ = schemas::OneOfSchema::String("hello".to_string());
    let x = schemas::OneOfSchema::BooleanAlias(true);
    match x {
        schemas::OneOfSchema::BooleanAlias(_) => {}
        schemas::OneOfSchema::NumberTitle(_) => unreachable!(),
        schemas::OneOfSchema::String(_) => unreachable!(),
    }
}

