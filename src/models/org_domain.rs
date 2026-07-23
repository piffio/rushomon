use serde::{Deserialize, Deserializer, Serialize};

/// Custom deserializer to handle Cloudflare D1's quirk where SQLite booleans
/// (0/1) are returned across the WASM boundary as floating point numbers (0.0/1.0).
fn deserialize_d1_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(f != 0.0)
            } else if let Some(i) = n.as_i64() {
                Ok(i != 0)
            } else {
                Ok(false)
            }
        }
        serde_json::Value::String(s) => Ok(s == "1" || s.to_lowercase() == "true"),
        _ => Ok(false),
    }
}

/// A domain associated with an organization for just-in-time provisioning.
/// Users whose email domain matches a verified org domain are auto-joined
/// to that organization on sign-in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgDomain {
    pub id: String,
    pub org_id: String,
    pub domain: String,
    pub verification_method: String, // 'dns', 'oidc', etc
    pub verification_token: Option<String>,
    #[serde(deserialize_with = "deserialize_d1_bool")]
    pub is_verified: bool,
    pub created_at: i64,
    pub verified_at: Option<i64>,
}
