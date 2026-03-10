//! # Conreg Feign
//!
//! This crate provides procedural macros for creating Feign-like declarative HTTP clients.
//!
//! # Feign Client Macro
//!
//! Used to create declarative HTTP clients similar to Java Feign. By annotating a trait with this macro,
//! implementation code for HTTP requests is automatically generated, enabling RESTful communication between microservices.
//!
//! ## Parameters
//!
//! - `service_id`: **Required**, the unique identifier of the service, used for service discovery and load balancing.
//! - `base_path`: *Optional*, the base path prefix that will be prepended to all request paths.
//! - `url`: *Optional*, directly specifies the base URL for requests; if set, `service_id` and `base_path` will be ignored.
//!
//! Example:
//! ```rust
//! #[feign_client(service_id = "user-service", base_path = "/api")]
//! trait UserService{
//!   #[get("/api/users/{id}")]
//!   async fn get_user(&self, id: i32) -> Result<String, FeignError>;
//!
//!   // ... other methods
//! }
//!
//! // Then, you can use the generated client like this:
//! let client = UserServiceImpl::default();
//! let user = client.get_user(1).await?;
//! ```
//!
//! # Http Method Macros
//!
//! ## Supported HTTP Method Annotations
//!
//! - `#[get]`
//! - `#[post]`
//! - `#[put]`
//! - `#[delete]`
//! - `#[patch]`
//!
//! ## Parameter Binding Methods
//!
//! ### Path Parameters
//! Use `{param_name}` placeholders in the path. When a method parameter name matches the placeholder name, it is automatically bound.
//!
//! Example:
//! ```rust
//! #[get("/api/users/{id}")]
//! async fn ip(&self, id: i32) -> Result<String, FeignError>;
//! ```
//!
//! ### Query Parameters
//! Use `query = "{param}"` to specify query parameter templates.
//!
//! Example:
//! ```rust
//! #[get(path = "/api/users", query = "id={id}")]
//! async fn query(&self, id: i32) -> Result<String, FeignError>;
//! ````
//!
//! ### Form Parameters
//! Use `form = "{param}"` to specify form data, supporting `application/x-www-form-urlencoded` and `multipart/form-data`.
//!
//! Example:
//! ```rust
//! #[post(path = "/api/login", form = "{loginForm}")]
//! async fn login(&self, loginForm: reqwest::multipart::Form) -> Result<String, FeignError>;
//! ```
//!
//! ### Body Parameter
//! Use `body = "{param}"` to specify the raw string body.
//!
//! Example:
//! ```rust
//! #[post(path = "/api/post", body = "{data}")]
//! async fn post(&self, data: String) -> Result<String, FeignError>;
//! ```
//!
//! ### JSON Parameter
//! Use `json = "{param}"` to specify JSON data; it will be automatically serialized and `Content-Type: application/json` will be set.
//!
//! Note: need serde_json
//!
//! Example:
//! ```rust
//! #[post(path = "/api/post", json = "{data}")]
//! async fn post(&self, data: serde_json::Value) -> Result<String, FeignError> {}
//! ```
//!
//! ### Header Parameter
//! Use `headers("Key=Value", ...)` or `headers("Key={param}", ...)` to specify request headers, supporting both static values and dynamic parameters.
//!
//! Example:
//! ```rust
//! #[get(path = "/api/users", headers("Authorization: Bearer {token}", "Accept: application/json"))]
//! async fn get_users(&self, token: String) -> Result<String, FeignError>
//! ```

use proc_macro::TokenStream;

mod feign_client;

#[proc_macro_attribute]
pub fn feign_client(args: TokenStream, input: TokenStream) -> TokenStream {
    feign_client::feign_client_impl(args, input)
}

/// GET request annotation
#[proc_macro_attribute]
pub fn get(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// POST request annotation
#[proc_macro_attribute]
pub fn post(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// PUT request annotation
#[proc_macro_attribute]
pub fn put(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// DELETE request annotation
#[proc_macro_attribute]
pub fn delete(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// PATCH request annotation
#[proc_macro_attribute]
pub fn patch(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}
