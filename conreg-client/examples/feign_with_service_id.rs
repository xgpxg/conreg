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
    async fn ip(&self) -> Result<String, FeignError>;
}

#[tokio::main]
async fn main() {
    let client = ExampleClientImpl::default();

    let response = client.ip().await.unwrap();
    println!("ip -> {:?}", response);

    let response = client.json().await.unwrap();
    println!("json -> {:#?}", response);

    let response = client.image().await.unwrap();
    let path = Path::new("image.png");
    std::fs::write(&path, response).unwrap();
    println!("image saved -> {:?}", path.canonicalize().unwrap());

    let form = Form::new().text("custname", "Hello, this is form form!");
    let response = client.form(form).await.unwrap();
    println!("form -> {:?}", response);

    let response = client.headers("Hello, this is headers!").await.unwrap();
    println!("headers -> {:?}", response);
}
