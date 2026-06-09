/// Pending Actions Repository
///
/// Data access layer for the `pending_actions` table — generic storage for
/// deferred, token-confirmed operations (e.g. billing account ownership transfer).
use crate::models::pending_action::PendingAction;
use crate::utils::now_timestamp;
use worker::Result;
use worker::d1::D1Database;

pub struct PendingActionsRepository;

impl PendingActionsRepository {
    pub fn new() -> Self {
        Self
    }

    /// Create a new pending action record.
    ///
    /// `ttl_seconds` is added to `now` to compute `expires_at`.
    /// Returns the newly created `PendingAction`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        db: &D1Database,
        action_type: &str,
        subject_id: &str,
        initiated_by: &str,
        to_email: &str,
        payload_json: &str,
        ttl_seconds: i64,
    ) -> Result<PendingAction> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_timestamp();
        let expires_at = now + ttl_seconds;

        db.prepare(
            "INSERT INTO pending_actions
               (id, action_type, subject_id, initiated_by, to_email, payload, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&[
            id.clone().into(),
            action_type.into(),
            subject_id.into(),
            initiated_by.into(),
            to_email.into(),
            payload_json.into(),
            (now as f64).into(),
            (expires_at as f64).into(),
        ])?
        .run()
        .await?;

        Ok(PendingAction {
            id,
            action_type: action_type.to_string(),
            subject_id: subject_id.to_string(),
            initiated_by: initiated_by.to_string(),
            to_email: to_email.to_string(),
            payload: payload_json.to_string(),
            created_at: now,
            expires_at,
            accepted_at: None,
            cancelled_at: None,
        })
    }

    /// Retrieve a pending action by its token (id).
    pub async fn get_by_token(
        &self,
        db: &D1Database,
        token: &str,
    ) -> Result<Option<PendingAction>> {
        db.prepare(
            "SELECT id, action_type, subject_id, initiated_by, to_email, payload,
                    created_at, expires_at, accepted_at, cancelled_at
             FROM pending_actions
             WHERE id = ?1",
        )
        .bind(&[token.into()])?
        .first::<PendingAction>(None)
        .await
    }

    /// Get the most recent open (non-accepted, non-cancelled, non-expired) action
    /// of a given type for a given subject.
    ///
    /// Used to enforce "one pending transfer per billing account at a time".
    #[allow(dead_code)]
    pub async fn get_pending_for_subject(
        &self,
        db: &D1Database,
        action_type: &str,
        subject_id: &str,
    ) -> Result<Option<PendingAction>> {
        let now = now_timestamp();
        db.prepare(
            "SELECT id, action_type, subject_id, initiated_by, to_email, payload,
                    created_at, expires_at, accepted_at, cancelled_at
             FROM pending_actions
             WHERE action_type = ?1
               AND subject_id  = ?2
               AND accepted_at  IS NULL
               AND cancelled_at IS NULL
               AND expires_at   > ?3
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(&[action_type.into(), subject_id.into(), (now as f64).into()])?
        .first::<PendingAction>(None)
        .await
    }

    /// Mark a pending action as accepted (sets `accepted_at` to now).
    pub async fn accept(&self, db: &D1Database, token: &str) -> Result<()> {
        let now = now_timestamp();
        db.prepare("UPDATE pending_actions SET accepted_at = ?1 WHERE id = ?2")
            .bind(&[(now as f64).into(), token.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Mark a single pending action as cancelled (sets `cancelled_at` to now).
    #[allow(dead_code)]
    pub async fn cancel(&self, db: &D1Database, token: &str) -> Result<()> {
        let now = now_timestamp();
        db.prepare("UPDATE pending_actions SET cancelled_at = ?1 WHERE id = ?2")
            .bind(&[(now as f64).into(), token.into()])?
            .run()
            .await?;
        Ok(())
    }

    /// Cancel all open pending actions of a given type for a given subject.
    ///
    /// Used when re-initiating a transfer: the old pending request is superseded.
    pub async fn cancel_all_for_subject(
        &self,
        db: &D1Database,
        action_type: &str,
        subject_id: &str,
    ) -> Result<()> {
        let now = now_timestamp();
        db.prepare(
            "UPDATE pending_actions
             SET cancelled_at = ?1
             WHERE action_type  = ?2
               AND subject_id   = ?3
               AND accepted_at  IS NULL
               AND cancelled_at IS NULL",
        )
        .bind(&[(now as f64).into(), action_type.into(), subject_id.into()])?
        .run()
        .await?;
        Ok(())
    }
}
