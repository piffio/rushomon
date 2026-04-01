//! Binary to generate OpenAPI specification for Rushomon API
//!
//! Usage: cargo run --bin generate-openapi --features openapi-gen

use rushomon::openapi;

fn main() {
    match openapi::generate_openapi_json() {
        Ok(json) => {
            println!("{}", json);
        }
        Err(e) => {
            eprintln!("Error generating OpenAPI spec: {}", e);
            std::process::exit(1);
        }
    }
}
