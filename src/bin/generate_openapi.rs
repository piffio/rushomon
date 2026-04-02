//! Binary to generate OpenAPI specification for Rushomon API
//!
//! Usage: cargo run --bin generate-openapi --features openapi-gen
//!
//! Environment variables:
//!   OPENAPI_VERSION - Override the version in the spec (e.g., "main", "unreleased")
//!                     If not set, uses the version from Cargo.toml

use rushomon::openapi;
use serde_json::Value;

fn main() {
    match openapi::generate_openapi_json() {
        Ok(json) => {
            // Allow overriding the version via environment variable
            let output = if let Ok(custom_version) = std::env::var("OPENAPI_VERSION") {
                // Parse JSON, update version, and re-serialize
                let mut spec: Value = serde_json::from_str(&json).expect("Invalid JSON generated");
                if let Some(info) = spec.get_mut("info") {
                    if let Some(info_obj) = info.as_object_mut() {
                        info_obj.insert("version".to_string(), Value::String(custom_version));
                    }
                }
                serde_json::to_string_pretty(&spec).expect("Failed to serialize JSON")
            } else {
                json
            };
            println!("{}", output);
        }
        Err(e) => {
            eprintln!("Error generating OpenAPI spec: {}", e);
            std::process::exit(1);
        }
    }
}
