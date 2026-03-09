use conreg_client::FeignError;
use conreg_feign_macro::{feign_client, get, post};
use reqwest::multipart::Form;
use rocket::http::hyper::body::Bytes;
use std::path::Path;
/// This example show you how to use `feign_client`
///
/// When adding `#[feign_client]` on a trait, an implementation named `<trait_name>Impl` is automatically generated.
///
/// # Run example
/// ```
/// cargo run --example feign_with_service_id -F feign,tracing -- --nocapture
#[feign_client(service_id = "test-server")]
trait ExampleClient {
    /// Request `GET` and return string.
    #[get("/hello")]
    async fn hello(&self) -> Result<String, FeignError>;
}

#[tokio::main]
async fn main() {
    let client = ExampleClientImpl::default();

    let response = client.hello().await.unwrap();
    println!("hello -> {:?}", response);

}
