use std::fs;
use std::path::Path;
use std::process::Command;

fn set_build_info() {
    // Set build timestamp
    let timestamp = Command::new("date")
        .args(&["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Set git commit
    let git_commit = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);
    println!("cargo:rustc-env=GIT_COMMIT={}", git_commit);

    println!(
        "🔧 Build info set: timestamp={}, commit={}",
        timestamp, git_commit
    );
}

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=frontend/package.json");

    // Set build information for version API
    set_build_info();

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
