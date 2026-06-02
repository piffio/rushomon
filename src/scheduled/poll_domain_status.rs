//! Scheduled job: poll Cloudflare for SaaS to update pending domain statuses.
//!
//! Runs every 15 minutes and checks all `pending` custom domains against the
//! Cloudflare API. Domains that become `active` get their KV entries synced.

use crate::models::custom_domain::STATUS_ACTIVE;
use crate::repositories::CustomDomainRepository;
use crate::utils::cf_saas;
use worker::d1::D1Database;
use worker::kv::KvStore;
use worker::*;

/// Poll all pending custom domains and update their statuses.
/// Returns the number of domains processed and the number of status changes.
pub async fn run(db: &D1Database, kv: &KvStore, env: &Env) -> (usize, usize) {
    poll_pending_domains(db, kv, env).await
}

/// Poll all pending custom domains and update their statuses.
/// Returns (domains_processed, status_changes).
pub async fn poll_pending_domains(db: &D1Database, kv: &KvStore, env: &Env) -> (usize, usize) {
    let repo = CustomDomainRepository::new();

    let pending = match repo.get_all_pending(db).await {
        Ok(p) => p,
        Err(e) => {
            console_error!("[domains] Failed to fetch pending domains: {}", e);
            return (0, 0);
        }
    };

    if pending.is_empty() {
        return (0, 0);
    }

    console_log!("[domains] Polling {} pending domain(s)", pending.len());
    let mut changes = 0;

    for domain in &pending {
        let cf_id = match domain.cf_hostname_id.as_deref() {
            Some(id) => id,
            None => {
                continue;
            }
        };

        let cf_result = match cf_saas::get_custom_hostname(env, cf_id).await {
            Ok(r) => r,
            Err(e) => {
                console_error!(
                    "[domains] CF API error for domain {}: {}",
                    domain.hostname,
                    e
                );
                continue;
            }
        };

        let new_status = match &cf_result {
            Some(cf) => match cf.status.as_str() {
                "active" => STATUS_ACTIVE,
                "pending" | "pending_validation" => "pending",
                _ => "failed",
            },
            None => "pending",
        };

        if new_status == domain.status.as_str() {
            continue;
        }

        let verified_at = if new_status == STATUS_ACTIVE {
            Some(crate::utils::now_timestamp())
        } else {
            None
        };

        let updated_cf_id = cf_result.as_ref().map(|cf| cf.id.as_str());

        if let Err(e) = repo
            .update_status(db, &domain.id, new_status, updated_cf_id, verified_at)
            .await
        {
            console_error!(
                "[domains] Failed to update status for domain {}: {}",
                domain.hostname,
                e
            );
            continue;
        }

        console_log!(
            "[domains] Domain {} status: {} → {}",
            domain.hostname,
            domain.status,
            new_status
        );
        changes += 1;

        // If domain just became active, write KV entries for all active org links
        if new_status == STATUS_ACTIVE
            && let Err(e) = backfill_kv_for_domain(db, kv, &domain.org_id, &domain.hostname).await
        {
            console_error!(
                "[domains] KV backfill failed for domain {}: {}",
                domain.hostname,
                e
            );
        }
    }

    (pending.len(), changes)
}

/// Write {hostname}:{short_code} KV entries for all active links in an org.
async fn backfill_kv_for_domain(
    db: &D1Database,
    kv: &KvStore,
    org_id: &str,
    hostname: &str,
) -> worker::Result<()> {
    use crate::models::link::LinkStatus;
    use crate::repositories::OrgRepository;

    let links = OrgRepository::new().get_links(db, org_id).await?;

    for link in links {
        if matches!(link.status, LinkStatus::Active) {
            let resolved_forward = link.forward_query_params.unwrap_or(false);
            let mapping = link.to_mapping(resolved_forward);
            super::super::kv::links::store_link_mapping_for_domain(
                kv,
                hostname,
                &link.short_code,
                &mapping,
            )
            .await?;
        }
    }

    Ok(())
}
