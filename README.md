# Axum-OpenAPI
Automatic code-generation for Axum using the OpenAPI v3.1 specification.

This crate automatically generates structs for every path in the OpenAPI specification. These structs automatically deserialize the request's path-parameters, query-parameters and optionally the request's body as well. The generated structs implement:
- `axum::FromRequestParts`/`axum::FromRequest`: This allows the struct to be used as a standard axum extractor.
-  `axum_open_api::OapiPath`: This trait defines the struct's path, e.g. `GET /user/username`, as statically defined parameters. It allows the struct to be used with the trait `OapiRoute`.

All defined schema's (either inline or in `/components/schemas`) are automatically converted to rust structs that implement `serde::Serialize` and `serde::Deserialize`.

## Note
- The extractor **must** be the last extractor of a route for it the handler to implement `OapiPath`. If this is not possible, then one has to manually register the handler.

# Example
`my-api.yaml`
```yaml
openapi: 3.1.0
info:
  title: My API
  version: 0.0.1
components:
  schemas:
    User:
      type: object
      required:
        - "id"
        - "username"
      properties:
        id:
          type: integer
        username:
          type: string
        description:
          type: string
paths:
  /users/{user_id}:
    get: 
      summary: Returns the user's information.
      parameters:
        - in: path
          name: user_id
          schema:
            type: string
          required: true
          description: The user ID
        - in: query
          name: description
          schema:
            type: bool
          description: Whether to fetch the user description
      responses:
        '200':
          description: An array of tags that make up this user's feed
          content:
            application/json:
              schema: 
                $ref: /components/schemas/User
```

`main.rs`
```rust
validate_routes!(
    path = "../my-api.yaml";

    mod users {
        GET     /users/{user_id}    as pub GetUser;
        POST    /users              as pub PostUser;
    }
);

impl users::GetUser {
    pub async fn handle(self) -> Json(schemas::User) {
        let Self {
            user_id: String,
            description: bool,
        }
        todo!()
    }
}

impl users::PostUser {
    pub async fn handle(self) {
        let Self {
            body: schemas::User {
                id: i64,
                username: String,
                description: Option<String>,
            }
        }
        todo!()
    }
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .oapi_route(users::GetUser::handle)
        .oapi_route(users::PostUser::handle);

    let _ = axum::serve(router).await;
}
```

# OpenAPI 3.1 support
- Basic types (`string` as `String`, `integer` as `i64`, `number` as `f64`, `boolean` as `bool`).
- `$ref` references.
- `array` as `Vec<T>`.
- `oneOf` as `enum`.
- `object` as `struct`.
- `required` fields with `Option<T>`.
- Inline schema creation with `title` attribute.
- Automatic `requestBody` deserialization with:
  - `application/json` as `axum::extract::Json`.
  - `application/x-www-form-urlencoded` as `axum::extract::Form`.
  - `multipart/form-data` as `axum::extract::Multipart`.
  - `text/*` as `String`.
  - `*/*` as  `Binary`.
- Path-parameters and query-parameters.
- Get, post, put, delete, patch, head and options.

## Not supported
- `anyOf` and `allOf`.
- Custom body deserializers.
- Custom types to replace the basic types. (e.g. `i32` instead of `i64`).
- Validation (e.g. `min`, `max`, `regex` etc.).
- Dynamic `dictionary` objects.
- Default values.
- Enforcing that the handler-methods return proper types.
- Fields named `body` overlap with any body extractor's