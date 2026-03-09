use conreg_client::FeignError;
use conreg_feign_macro::{feign_client, get};
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
    conreg_client::init_from_file("./conreg-client/examples/bootstrap.yaml").await;
    let client = ExampleClientImpl::default();

    let response = client.hello().await.unwrap();
    println!("hello -> {:?}", response);
}
