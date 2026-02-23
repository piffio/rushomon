use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=frontend/package.json");

    // Read version from Cargo.toml
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let cargo: toml::Value = toml::from_str(&cargo_toml).expect("Failed to parse Cargo.toml");

    let version = cargo
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("Version not found in Cargo.toml");

    println!("🔄 Syncing version {} to frontend", version);

    // Update frontend/package.json
    let package_json_path = Path::new("frontend/package.json");
    let package_json =
        fs::read_to_string(package_json_path).expect("Failed to read frontend/package.json");

    // Parse and update version
    let mut package: serde_json::Value =
        serde_json::from_str(&package_json).expect("Failed to parse frontend/package.json");

    if let Some(obj) = package.as_object_mut() {
        obj.insert(
            "version".to_string(),
            serde_json::Value::String(version.to_string()),
        );
    }

    // Write back to package.json
    let updated_json =
        serde_json::to_string_pretty(&package).expect("Failed to serialize updated package.json");

    fs::write(package_json_path, updated_json)
        .expect("Failed to write updated frontend/package.json");

    println!("✅ Frontend version synchronized to {}", version);
}
