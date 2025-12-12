use std::env;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root_dir = Path::new(&manifest_dir);
    let web_dir = root_dir.parent().unwrap().join("web");

    println!("cargo:rustc-env=WEB_DIR={}", web_dir.display());
}
