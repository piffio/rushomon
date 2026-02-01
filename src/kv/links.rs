use worker::{kv::KvStore, Result};
use crate::models::LinkMapping;

/// KV key format: link:{org_id}:{short_code}
/// For Phase 1 (single domain), we use a simpler global format: {short_code}
fn make_key(_org_id: &str, short_code: &str) -> String {
    // Using global namespace for simplicity (Phase 1)
    // For multi-domain support, would use: format!("link:{}:{}", org_id, short_code)
    short_code.to_string()
}

/// Store a link mapping in KV
pub async fn store_link_mapping(
    kv: &KvStore,
    org_id: &str,
    short_code: &str,
    mapping: &LinkMapping,
) -> Result<()> {
    let key = make_key(org_id, short_code);
    kv.put(&key, mapping)?
        .execute()
        .await?;
    Ok(())
}

/// Get a link mapping from KV
pub async fn get_link_mapping(
    kv: &KvStore,
    short_code: &str,
) -> Result<Option<LinkMapping>> {
    // Note: For global namespace, we don't need org_id
    // For multi-tenant with org prefix, would need to iterate or use secondary lookup
    kv.get(short_code)
        .json::<LinkMapping>()
        .await
        .map_err(|e| worker::Error::RustError(format!("KV error: {:?}", e)))
}

/// Delete a link mapping from KV
pub async fn delete_link_mapping(
    kv: &KvStore,
    org_id: &str,
    short_code: &str,
) -> Result<()> {
    let key = make_key(org_id, short_code);
    kv.delete(&key).await?;
    Ok(())
}

/// Check if a short code already exists (collision detection)
pub async fn short_code_exists(
    kv: &KvStore,
    short_code: &str,
) -> Result<bool> {
    Ok(kv.get(short_code).text().await?.is_some())
}
