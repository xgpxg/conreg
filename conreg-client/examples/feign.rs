use reqwest::multipart::{Form, Part};
use rocket::http::hyper::body::Bytes;
use conreg_client::FeignError;
use conreg_client::lb::LoadBalanceClient;
use conreg_feign_macro::{feign_client, get, post};
use conreg_client::lb;

/// An example feign client with httpbin
#[feign_client(service_id = "echo", url = "https://httpbin.org")]
trait ExampleClient {
    /// 测试post
    #[get("/ip")]
    async fn ip(&self) -> Result<String, FeignError>;
    /// 测试post
    #[get(path = "/json")]
    async fn json(&self) -> Result<serde_json::Value, FeignError>;

    #[get(path = "/image",headers("Accept=*/*"))]
    async fn image(&self) -> Result<Bytes, FeignError>;

    #[post(path = "/forms/post",headers("Accept=*/*"))]
    async fn form(&self, form: Form) -> Result<String, FeignError>;
}

#[tokio::main]
async fn main() {
    let client = ExampleClientImpl::default();

    let response = client.ip().await.unwrap();
    println!("ip -> {:?}", response);

    let response = client.json().await.unwrap();
    println!("json -> {:#?}", response);

    let response = client.image().await.unwrap();
    println!("image -> {:#?}", response);

    //
    // let form = Form::new()
    //     .text("fileName", "test.txt")
    //     .part("file", Part::bytes(b"test".to_vec()).file_name("test.txt"));
    //
    // let client = OneapiClientImpl::new(LoadBalanceClient::new());
    //
    // let response = client.upload(form).await.unwrap();
    // println!("{:?}", response);

}

