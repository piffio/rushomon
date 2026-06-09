/// Ownership Transfer Service
///
/// Business logic for transferring billing account ownership between users.
///
/// # Flow (owner-initiated, email-confirmed)
///
/// 1. BA owner calls `initiate_transfer` → creates a `pending_actions` row and sends an email.
/// 2. Recipient clicks the link → calls `accept_transfer` (must be logged in with matching email).
/// 3. Atomically: BA `owner_user_id` is updated, former owner is downgraded to Admin in all orgs
///    under the BA, and the new owner is promoted to Owner in those orgs.
///
/// # Flow (admin force-transfer)
///
/// A system admin calls `admin_force_transfer` to reassign a BA immediately with no email step.
use crate::models::pending_action::{BillingTransferPayload, PendingAction, action_type};
use crate::repositories::org_repository::OrgRepository;
use crate::repositories::{BillingRepository, PendingActionsRepository, UserRepository};
use crate::utils::get_frontend_url;
use crate::utils::now_timestamp;
use worker::d1::D1Database;
use worker::{Env, console_log};

/// Transfer TTL: 7 days in seconds (matches org invitation TTL).
const TRANSFER_TTL_SECONDS: i64 = 7 * 24 * 3600;

/// Errors that can occur during an ownership transfer.
#[derive(Debug)]
pub enum TransferError {
    NotFound(String),
    Forbidden(String),
    BadRequest(String),
    Gone(String),
    Internal(String),
}

impl TransferError {
    pub fn status_code(&self) -> u16 {
        match self {
            TransferError::NotFound(_) => 404,
            TransferError::Forbidden(_) => 403,
            TransferError::BadRequest(_) => 400,
            TransferError::Gone(_) => 410,
            TransferError::Internal(_) => 500,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            TransferError::NotFound(m)
            | TransferError::Forbidden(m)
            | TransferError::BadRequest(m)
            | TransferError::Gone(m)
            | TransferError::Internal(m) => m.as_str(),
        }
    }
}

impl From<worker::Error> for TransferError {
    fn from(e: worker::Error) -> Self {
        TransferError::Internal(e.to_string())
    }
}

/// Public info about a pending transfer — returned to the acceptance page.
#[derive(Debug, serde::Serialize)]
pub struct TransferInfo {
    pub token: String,
    pub billing_account_id: String,
    pub billing_account_tier: String,
    pub from_user_name: Option<String>,
    pub from_user_email: String,
    pub to_email: String,
    pub expires_at: i64,
}

pub struct OwnershipTransferService;

impl OwnershipTransferService {
    pub fn new() -> Self {
        Self
    }

    // ─── Initiate transfer ────────────────────────────────────────────────────

    /// Initiate a billing account ownership transfer.
    ///
    /// - `initiator_id` must be the current `owner_user_id` of the billing account.
    /// - `to_email` must belong to an existing Rushomon user who is a member of at least
    ///   one org linked to this billing account.
    /// - Any existing pending transfer for this BA is cancelled first (one at a time).
    /// - Sends a confirmation email to `to_email`.
    pub async fn initiate_transfer(
        &self,
        db: &D1Database,
        env: &Env,
        ba_id: &str,
        initiator_id: &str,
        to_email: &str,
    ) -> Result<PendingAction, TransferError> {
        let billing_repo = BillingRepository::new();
        let user_repo = UserRepository::new();
        let pending_repo = PendingActionsRepository::new();

        // 1. Load the billing account and verify the initiator is the owner.
        let ba = billing_repo
            .get_by_id(db, ba_id)
            .await?
            .ok_or_else(|| TransferError::NotFound("Billing account not found".to_string()))?;

        if ba.owner_user_id != initiator_id {
            return Err(TransferError::Forbidden(
                "Only the billing account owner can initiate a transfer".to_string(),
            ));
        }

        // 2. Verify recipient exists and is a member of at least one org under this BA.
        let recipient = user_repo.get_by_email(db, to_email).await?.ok_or_else(|| {
            TransferError::BadRequest(
                "No Rushomon account found for that email address".to_string(),
            )
        })?;

        if recipient.id == initiator_id {
            return Err(TransferError::BadRequest(
                "Cannot transfer ownership to yourself".to_string(),
            ));
        }

        let is_member = self
            .is_member_of_billing_account(db, ba_id, &recipient.id)
            .await?;

        if !is_member {
            return Err(TransferError::BadRequest(
                "The recipient must already be a member of one of the organizations in this billing account".to_string(),
            ));
        }

        // 3. Cancel any existing pending transfer for this BA.
        pending_repo
            .cancel_all_for_subject(db, action_type::BILLING_ACCOUNT_TRANSFER, ba_id)
            .await?;

        // 4. Create the new pending action.
        let payload = serde_json::to_string(&BillingTransferPayload {
            billing_account_id: ba_id.to_string(),
            from_user_id: initiator_id.to_string(),
        })
        .map_err(|e| TransferError::Internal(e.to_string()))?;

        let action = pending_repo
            .create(
                db,
                action_type::BILLING_ACCOUNT_TRANSFER,
                ba_id,
                initiator_id,
                to_email,
                &payload,
                TRANSFER_TTL_SECONDS,
            )
            .await?;

        // 5. Send the confirmation email (non-blocking on failure).
        let initiator = user_repo.get_user_by_id(db, initiator_id).await?;
        let initiator_name = initiator
            .as_ref()
            .and_then(|u| u.name.clone())
            .or_else(|| initiator.as_ref().map(|u| u.email.clone()))
            .unwrap_or_else(|| "Someone".to_string());

        let accept_url = format!("{}/billing-transfer/{}", get_frontend_url(env), action.id);

        if let Err(e) = crate::utils::email::send_ownership_transfer_request(
            env,
            to_email,
            &initiator_name,
            &ba.tier,
            &accept_url,
        )
        .await
        {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "ownership_transfer_email_failed",
                    "billing_account_id": ba_id,
                    "to_email": to_email,
                    "error": e.to_string(),
                    "level": "warn"
                })
            );
        }

        Ok(action)
    }

    // ─── Get transfer info (public) ───────────────────────────────────────────

    /// Return public information about a pending transfer for the acceptance page.
    /// Does NOT require authentication.
    pub async fn get_transfer_info(
        &self,
        db: &D1Database,
        token: &str,
    ) -> Result<TransferInfo, TransferError> {
        let pending_repo = PendingActionsRepository::new();
        let billing_repo = BillingRepository::new();
        let user_repo = UserRepository::new();

        let action = pending_repo
            .get_by_token(db, token)
            .await?
            .ok_or_else(|| TransferError::NotFound("Transfer not found".to_string()))?;

        self.validate_pending(&action)?;

        let payload: BillingTransferPayload = serde_json::from_str(&action.payload)
            .map_err(|e| TransferError::Internal(e.to_string()))?;

        let ba = billing_repo
            .get_by_id(db, &payload.billing_account_id)
            .await?
            .ok_or_else(|| TransferError::NotFound("Billing account not found".to_string()))?;

        let from_user = user_repo.get_user_by_id(db, &payload.from_user_id).await?;

        Ok(TransferInfo {
            token: token.to_string(),
            billing_account_id: payload.billing_account_id,
            billing_account_tier: ba.tier,
            from_user_name: from_user.as_ref().and_then(|u| u.name.clone()),
            from_user_email: from_user
                .map(|u| u.email)
                .unwrap_or_else(|| "unknown".to_string()),
            to_email: action.to_email,
            expires_at: action.expires_at,
        })
    }

    // ─── Accept transfer ──────────────────────────────────────────────────────

    /// Accept a pending ownership transfer.
    ///
    /// - The acceptor must be logged in (authenticated) — `acceptor_user_id` is from the JWT.
    /// - `acceptor_email` must match the `to_email` stored in the pending action.
    /// - Atomically reassigns the BA owner and adjusts org roles.
    pub async fn accept_transfer(
        &self,
        db: &D1Database,
        env: &Env,
        token: &str,
        acceptor_user_id: &str,
        acceptor_email: &str,
    ) -> Result<(), TransferError> {
        let pending_repo = PendingActionsRepository::new();
        let billing_repo = BillingRepository::new();
        let user_repo = UserRepository::new();

        let action = pending_repo
            .get_by_token(db, token)
            .await?
            .ok_or_else(|| TransferError::NotFound("Transfer not found".to_string()))?;

        self.validate_pending(&action)?;

        // Email must match.
        if action.to_email.to_lowercase() != acceptor_email.to_lowercase() {
            return Err(TransferError::Forbidden(
                "This transfer was not addressed to your account".to_string(),
            ));
        }

        let payload: BillingTransferPayload = serde_json::from_str(&action.payload)
            .map_err(|e| TransferError::Internal(e.to_string()))?;

        // Perform the ownership swap.
        self.execute_transfer(
            db,
            &payload.billing_account_id,
            &payload.from_user_id,
            acceptor_user_id,
        )
        .await?;

        // Mark the action accepted.
        pending_repo.accept(db, token).await?;

        // Send confirmation emails (non-blocking on failure).
        let former_owner = user_repo.get_user_by_id(db, &payload.from_user_id).await?;
        let new_owner = user_repo.get_user_by_id(db, acceptor_user_id).await?;

        let ba = billing_repo
            .get_by_id(db, &payload.billing_account_id)
            .await?;
        let tier = ba.as_ref().map(|b| b.tier.as_str()).unwrap_or("free");

        let dashboard_url = format!("{}/dashboard", get_frontend_url(env));

        for (recipient_email, recipient_name, is_new_owner) in [
            (
                former_owner
                    .as_ref()
                    .map(|u| u.email.as_str())
                    .unwrap_or(""),
                former_owner.as_ref().and_then(|u| u.name.as_deref()),
                false,
            ),
            (
                new_owner.as_ref().map(|u| u.email.as_str()).unwrap_or(""),
                new_owner.as_ref().and_then(|u| u.name.as_deref()),
                true,
            ),
        ] {
            if recipient_email.is_empty() {
                continue;
            }
            if let Err(e) = crate::utils::email::send_ownership_transfer_confirmation(
                env,
                recipient_email,
                recipient_name,
                tier,
                is_new_owner,
                &dashboard_url,
            )
            .await
            {
                console_log!(
                    "{}",
                    serde_json::json!({
                        "event": "ownership_transfer_confirmation_email_failed",
                        "billing_account_id": payload.billing_account_id,
                        "to_email": recipient_email,
                        "error": e.to_string(),
                        "level": "warn"
                    })
                );
            }
        }

        console_log!(
            "{}",
            serde_json::json!({
                "event": "billing_account_ownership_transferred",
                "billing_account_id": payload.billing_account_id,
                "from_user_id": payload.from_user_id,
                "to_user_id": acceptor_user_id,
                "level": "info"
            })
        );

        Ok(())
    }

    // ─── Cancel transfer ──────────────────────────────────────────────────────

    /// Cancel an outstanding transfer.
    ///
    /// Only the current BA owner (or a system admin — checked in the handler) can cancel.
    pub async fn cancel_transfer(
        &self,
        db: &D1Database,
        ba_id: &str,
        requester_id: &str,
    ) -> Result<(), TransferError> {
        let billing_repo = BillingRepository::new();
        let pending_repo = PendingActionsRepository::new();

        let ba = billing_repo
            .get_by_id(db, ba_id)
            .await?
            .ok_or_else(|| TransferError::NotFound("Billing account not found".to_string()))?;

        if ba.owner_user_id != requester_id {
            return Err(TransferError::Forbidden(
                "Only the billing account owner can cancel a transfer".to_string(),
            ));
        }

        pending_repo
            .cancel_all_for_subject(db, action_type::BILLING_ACCOUNT_TRANSFER, ba_id)
            .await?;

        Ok(())
    }

    // ─── Admin force-transfer ─────────────────────────────────────────────────

    /// Atomically transfer a billing account to a new owner, bypassing email confirmation.
    ///
    /// The `to_user_id` must already be a member of one of the BA's organizations.
    /// Any pending transfer for this BA is cancelled.
    ///
    /// IMPORTANT: caller must have verified `requester` is a system admin before calling this.
    pub async fn admin_force_transfer(
        &self,
        db: &D1Database,
        ba_id: &str,
        to_user_id: &str,
        _admin_id: &str,
    ) -> Result<(), TransferError> {
        let billing_repo = BillingRepository::new();
        let pending_repo = PendingActionsRepository::new();

        let ba = billing_repo
            .get_by_id(db, ba_id)
            .await?
            .ok_or_else(|| TransferError::NotFound("Billing account not found".to_string()))?;

        if ba.owner_user_id == to_user_id {
            return Err(TransferError::BadRequest(
                "That user is already the billing account owner".to_string(),
            ));
        }

        let is_member = self
            .is_member_of_billing_account(db, ba_id, to_user_id)
            .await?;

        if !is_member {
            return Err(TransferError::BadRequest(
                "The target user must be a member of one of the organizations in this billing account".to_string(),
            ));
        }

        // Cancel any outstanding pending transfer first.
        pending_repo
            .cancel_all_for_subject(db, action_type::BILLING_ACCOUNT_TRANSFER, ba_id)
            .await?;

        self.execute_transfer(db, ba_id, &ba.owner_user_id, to_user_id)
            .await?;

        console_log!(
            "{}",
            serde_json::json!({
                "event": "billing_account_ownership_force_transferred",
                "billing_account_id": ba_id,
                "from_user_id": ba.owner_user_id,
                "to_user_id": to_user_id,
                "admin_id": _admin_id,
                "level": "info"
            })
        );

        Ok(())
    }

    // ─── Internal helpers ─────────────────────────────────────────────────────

    /// Check whether a user is a member of at least one org linked to a billing account.
    async fn is_member_of_billing_account(
        &self,
        db: &D1Database,
        ba_id: &str,
        user_id: &str,
    ) -> Result<bool, TransferError> {
        let result = db
            .prepare(
                "SELECT COUNT(*) as count
                 FROM org_members om
                 JOIN organizations o ON o.id = om.org_id
                 WHERE o.billing_account_id = ?1
                   AND om.user_id = ?2",
            )
            .bind(&[ba_id.into(), user_id.into()])
            .map_err(|e| TransferError::Internal(e.to_string()))?
            .first::<serde_json::Value>(None)
            .await?;

        let count = result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64;

        Ok(count > 0)
    }

    /// Validate that a pending action is still open (not expired, accepted, or cancelled).
    fn validate_pending(&self, action: &PendingAction) -> Result<(), TransferError> {
        let now = now_timestamp();

        if action.cancelled_at.is_some() {
            return Err(TransferError::Gone(
                "This transfer request has been cancelled".to_string(),
            ));
        }
        if action.accepted_at.is_some() {
            return Err(TransferError::Gone(
                "This transfer has already been completed".to_string(),
            ));
        }
        if action.expires_at <= now {
            return Err(TransferError::Gone(
                "This transfer request has expired".to_string(),
            ));
        }
        Ok(())
    }

    /// Core transfer logic:
    ///
    /// 1. (Pre-flight) Check whether the former owner will still own any other BA after
    ///    the transfer. If not, they need a new personal BA + org.
    /// 2. Update `billing_accounts.owner_user_id` — this is the ONLY write to
    ///    `billing_accounts`. `organizations.billing_account_id` is intentionally
    ///    never touched: every org keeps its link to the same BA; only the BA owner changes.
    /// 3. Demote the former owner from `owner` → `admin` in every org under the BA.
    /// 4. Promote the new owner to `owner` in every org under the BA they are a member of.
    /// 5. Safety net (only when the former owner owns no other BA after the transfer):
    ///    create a brand-new free-tier BA and a brand-new personal org for them, add them
    ///    as owner of that org, and update `users.org_id` so the `get_for_user` path also
    ///    resolves correctly for them going forward.
    ///    Crucially we NEVER re-link an existing org to the new BA — that would strip the
    ///    tier from any shared orgs that should stay under the transferred BA.
    async fn execute_transfer(
        &self,
        db: &D1Database,
        ba_id: &str,
        from_user_id: &str,
        to_user_id: &str,
    ) -> Result<(), TransferError> {
        let billing_repo = BillingRepository::new();
        let org_repo = OrgRepository::new();
        let user_repo = UserRepository::new();

        // ── Pre-flight: does the former owner own any OTHER BA besides this one? ──
        //
        // Must be checked BEFORE we change owner_user_id so the count is accurate.
        let other_ba_count: i64 = db
            .prepare(
                "SELECT COUNT(*) as count
                 FROM billing_accounts
                 WHERE owner_user_id = ?1 AND id != ?2",
            )
            .bind(&[from_user_id.into(), ba_id.into()])
            .map_err(|e| TransferError::Internal(e.to_string()))?
            .first::<serde_json::Value>(None)
            .await?
            .and_then(|v| v["count"].as_f64())
            .unwrap_or(0.0) as i64;

        let former_owner_needs_new_ba = other_ba_count == 0;

        // Load the former owner record now; we need it in the safety-net step.
        let former_user = user_repo.get_user_by_id(db, from_user_id).await?;

        // ── Step 1: Transfer BA ownership ─────────────────────────────────────
        //
        // ONLY write to billing_accounts. organizations.billing_account_id stays
        // untouched — every org linked to this BA continues to benefit from its tier.
        billing_repo.update_owner(db, ba_id, to_user_id).await?;

        // ── Step 2: Update org member roles ───────────────────────────────────
        let org_ids = self.get_org_ids_for_billing_account(db, ba_id).await?;

        for org_id in &org_ids {
            // Demote former owner → admin (only if they currently hold the owner role).
            let former_member = org_repo.get_member(db, org_id, from_user_id).await?;
            if let Some(m) = former_member
                && m.role == "owner"
            {
                org_repo
                    .update_member_role(db, org_id, from_user_id, "admin")
                    .await?;
            }

            // Promote new owner → owner (only if they are already a member).
            let new_member = org_repo.get_member(db, org_id, to_user_id).await?;
            if new_member.is_some() {
                org_repo
                    .update_member_role(db, org_id, to_user_id, "owner")
                    .await?;
            }
        }

        // ── Step 3: Safety net — give the former owner a new personal BA + org ─
        //
        // We create a completely NEW org (not re-link any existing org) so that
        // shared orgs under the transferred BA are never affected.
        // We also update users.org_id so that legacy get_for_user paths resolve
        // to the new BA rather than one of the now-transferred orgs.
        if former_owner_needs_new_ba && let Some(ref user) = former_user {
            let new_ba = billing_repo.create(db, from_user_id, "free").await?;

            let personal_org_name = user
                .name
                .as_deref()
                .map(|n| format!("{n}'s Workspace"))
                .unwrap_or_else(|| "Personal Workspace".to_string());

            let new_org = org_repo
                .create_with_billing_account(db, &personal_org_name, from_user_id, &new_ba.id)
                .await?;

            org_repo
                .add_member(db, &new_org.id, from_user_id, "owner")
                .await?;

            // Update users.org_id so get_for_user / billing page resolve correctly.
            db.prepare("UPDATE users SET org_id = ?1 WHERE id = ?2")
                .bind(&[new_org.id.clone().into(), from_user_id.into()])
                .map_err(|e| TransferError::Internal(e.to_string()))?
                .run()
                .await?;

            console_log!(
                "{}",
                serde_json::json!({
                    "event": "safety_net_personal_workspace_created",
                    "new_billing_account_id": new_ba.id,
                    "new_org_id": new_org.id,
                    "for_user_id": from_user_id,
                    "level": "info"
                })
            );
        }

        Ok(())
    }

    /// Get all org IDs linked to a billing account.
    async fn get_org_ids_for_billing_account(
        &self,
        db: &D1Database,
        ba_id: &str,
    ) -> Result<Vec<String>, TransferError> {
        let rows = db
            .prepare("SELECT id FROM organizations WHERE billing_account_id = ?1")
            .bind(&[ba_id.into()])
            .map_err(|e| TransferError::Internal(e.to_string()))?
            .all()
            .await?
            .results::<serde_json::Value>()?;

        Ok(rows
            .iter()
            .filter_map(|v| v["id"].as_str().map(|s| s.to_string()))
            .collect())
    }

    /// Return true if the user is the `owner_user_id` of at least one billing account.
    #[allow(dead_code)]
    async fn owns_any_billing_account(
        &self,
        db: &D1Database,
        user_id: &str,
    ) -> Result<bool, TransferError> {
        let result = db
            .prepare("SELECT COUNT(*) as count FROM billing_accounts WHERE owner_user_id = ?1")
            .bind(&[user_id.into()])
            .map_err(|e| TransferError::Internal(e.to_string()))?
            .first::<serde_json::Value>(None)
            .await?;

        let count = result.and_then(|v| v["count"].as_f64()).unwrap_or(0.0) as i64;
        Ok(count > 0)
    }
}
