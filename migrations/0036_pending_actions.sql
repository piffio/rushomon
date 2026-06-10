-- Migration: 0036_pending_actions
-- Generic table for deferred, token-confirmed actions (e.g. billing account ownership transfer).
-- Each row represents one pending action that requires out-of-band confirmation (typically email).

CREATE TABLE pending_actions (
  id            TEXT PRIMARY KEY,     -- UUID, also serves as the magic-link token
  action_type   TEXT NOT NULL,        -- discriminator: 'billing_account_transfer' (extendable)
  subject_id    TEXT NOT NULL,        -- indexed FK-equivalent; meaning depends on action_type
                                      --   billing_account_transfer → billing_account_id
  initiated_by  TEXT NOT NULL,        -- user_id of the person who started the action
  to_email      TEXT NOT NULL,        -- email address of the intended recipient
  payload       TEXT NOT NULL,        -- JSON blob with action-specific data
  created_at    INTEGER NOT NULL,
  expires_at    INTEGER NOT NULL,     -- absolute Unix timestamp (created_at + TTL)
  accepted_at   INTEGER,              -- NULL = pending; timestamp = accepted
  cancelled_at  INTEGER               -- NULL = not cancelled; timestamp = cancelled
);

-- Fast lookup for "is there already a pending action of this type for this subject?"
CREATE INDEX idx_pending_actions_subject ON pending_actions(action_type, subject_id);

-- Fast lookup by recipient email
CREATE INDEX idx_pending_actions_email ON pending_actions(to_email);
