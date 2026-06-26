use rocket::{Config, get};
use conreg_client::{init_from_file};

/// # Run example
/// ```
/// cargo run --example test_server -F tracing -- --nocapture
/// ```
#[tokio::main]
async fn main() {
    init_from_file("./conreg-client/examples/bootstrap.yaml").await;

    rocket::build()
        .configure(Config {
            port: 8080,
            cli_colors: false,
            ..Config::debug_default()
        })
        .mount("/", rocket::routes![test])
        .launch()
        .await
        .unwrap();
}

#[get("/hello")]
fn test() -> &'static str {
    "world"
}
