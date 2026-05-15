/// KV sync helpers for custom domain dual-write
///
/// When a link is created, updated, or deleted, we must also update KV
/// for every active custom domain on the same org, using the key format:
///   {hostname}:{short_code}
use crate::models::LinkMapping;
use crate::repositories::CustomDomainRepository;
use worker::Result;
use worker::d1::D1Database;
use worker::kv::KvStore;

/// Sync KV entries for all active custom domains on an org for a specific short_code.
///
/// - `mapping = Some(m)` → write (or overwrite) the `{hostname}:{short_code}` key
/// - `mapping = None`    → delete the `{hostname}:{short_code}` key (link deleted/disabled)
pub async fn sync_custom_domain_kv(
    kv: &KvStore,
    db: &D1Database,
    org_id: &str,
    short_code: &str,
    mapping: Option<&LinkMapping>,
) -> Result<()> {
    let repo = CustomDomainRepository::new();
    let active_domains = repo.get_active_for_org(db, org_id).await?;

    for domain in &active_domains {
        match mapping {
            Some(m) => {
                super::links::store_link_mapping_for_domain(kv, &domain.hostname, short_code, m)
                    .await?;
            }
            None => {
                super::links::delete_link_mapping_for_domain(kv, &domain.hostname, short_code)
                    .await?;
            }
        }
    }

    Ok(())
}
