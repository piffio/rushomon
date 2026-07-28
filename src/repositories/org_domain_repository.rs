/// Organization Domain Repository
///
/// Data access layer for organization domain verification records, used for
/// just-in-time (JIT) provisioning: users whose email domain matches a
/// verified org domain are auto-joined to that organization on sign-in.
///
/// Distinct from `CustomDomainRepository`, which handles vanity link domains.
use crate::models::{OrgDomain, Organization};
use crate::utils::now_timestamp;
use worker::Result;
use worker::d1::D1Database;

pub struct OrgDomainRepository;

impl OrgDomainRepository {
    pub fn new() -> Self {
        Self
    }

    /// Get an org domain record by domain name.
    /// Prioritizes the verified record if multiple orgs have challenged the same domain.
    pub async fn get_by_domain(&self, db: &D1Database, domain: &str) -> Result<Option<OrgDomain>> {
        let stmt = db.prepare(
            "SELECT id, org_id, domain, verification_method, verification_token, is_verified, created_at, verified_at
             FROM org_domains WHERE domain = ?1 ORDER BY is_verified DESC LIMIT 1",
        );
        stmt.bind(&[domain.into()])?.first::<OrgDomain>(None).await
    }

    /// Find an organization by one of its verified domains.
    pub async fn get_org_by_verified_domain(
        &self,
        db: &D1Database,
        domain: &str,
    ) -> Result<Option<Organization>> {
        let stmt = db.prepare(
            "SELECT o.* FROM organizations o
             JOIN org_domains od ON o.id = od.org_id
             WHERE od.domain = ?1 AND od.is_verified = 1",
        );
        stmt.bind(&[domain.into()])?
            .first::<Organization>(None)
            .await
    }

    /// List all domains for an organization.
    pub async fn list_by_org(&self, db: &D1Database, org_id: &str) -> Result<Vec<OrgDomain>> {
        let stmt = db.prepare(
            "SELECT id, org_id, domain, verification_method, verification_token, is_verified, created_at, verified_at
             FROM org_domains WHERE org_id = ?1 ORDER BY created_at DESC",
        );
        let results = stmt.bind(&[org_id.into()])?.all().await?;
        results.results::<OrgDomain>()
    }

    /// Add a new (unverified) domain challenge to an organization.
    ///
    /// Any existing unverified challenge for the same domain is deleted first,
    /// so a domain can only ever have one outstanding challenge — this prevents
    /// a second org from squatting on a challenge another org started.
    pub async fn add_challenge(
        &self,
        db: &D1Database,
        org_id: &str,
        domain: &str,
        token: &str,
    ) -> Result<OrgDomain> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_timestamp();
        let normalized_domain = domain.to_lowercase();

        // Clean up any existing unverified challenge for this domain
        let cleanup_stmt =
            db.prepare("DELETE FROM org_domains WHERE domain = ?1 AND is_verified = 0");
        cleanup_stmt
            .bind(&[normalized_domain.clone().into()])?
            .run()
            .await?;

        let stmt = db.prepare(
            "INSERT INTO org_domains (id, org_id, domain, verification_method, verification_token, is_verified, created_at)
             VALUES (?1, ?2, ?3, 'dns', ?4, 0, ?5)",
        );
        stmt.bind(&[
            id.clone().into(),
            org_id.into(),
            normalized_domain.clone().into(),
            token.into(),
            (now as f64).into(),
        ])?
        .run()
        .await?;

        Ok(OrgDomain {
            id,
            org_id: org_id.to_string(),
            domain: normalized_domain,
            verification_method: "dns".to_string(),
            verification_token: Some(token.to_string()),
            is_verified: false,
            created_at: now,
            verified_at: None,
        })
    }

    /// Mark a domain as verified.
    pub async fn mark_verified(&self, db: &D1Database, domain: &str) -> Result<()> {
        let now = now_timestamp();
        let stmt = db
            .prepare("UPDATE org_domains SET is_verified = 1, verified_at = ?1 WHERE domain = ?2");
        stmt.bind(&[(now as f64).into(), domain.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Delete a domain from an organization.
    pub async fn delete(&self, db: &D1Database, org_id: &str, domain: &str) -> Result<()> {
        let stmt = db.prepare("DELETE FROM org_domains WHERE org_id = ?1 AND domain = ?2");
        stmt.bind(&[org_id.into(), domain.into()])?.run().await?;
        Ok(())
    }
}
