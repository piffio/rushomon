use crate::auth;
use crate::db;
use crate::models::Tier;
use crate::utils::email::send_org_invitation;
use chrono::Datelike;
use worker::d1::D1Database;
use worker::*;

// ─── GET /api/orgs ─────────────────────────────────────────────────────────
/// List all organizations the current user belongs to
pub async fn handle_list_user_orgs(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let orgs = db::get_user_orgs(&db, &user_ctx.user_id).await?;

    Response::from_json(&serde_json::json!({
        "orgs": orgs,
        "current_org_id": user_ctx.org_id,
    }))
}

// ─── POST /api/orgs ─────────────────────────────────────────────────────────
/// Create a new organization (respects tier org limits)
pub async fn handle_create_org(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Get user's billing account (not org tier) to determine org creation limits
    let billing_account = db::get_user_billing_account(&db, &user_ctx.user_id)
        .await?
        .ok_or_else(|| Error::RustError("No billing account found".to_string()))?;

    let tier = Tier::from_str_value(&billing_account.tier).unwrap_or(Tier::Free);
    let limits = tier.limits();

    // Check org limits against billing account (not per-org)
    if let Some(max_orgs) = limits.max_orgs {
        let orgs_in_billing_account =
            db::count_orgs_in_billing_account(&db, &billing_account.id).await?;

        if orgs_in_billing_account >= max_orgs {
            return Response::error(
                format!(
                    "Organization limit reached ({}/{}). Upgrade your plan to create more organizations.",
                    orgs_in_billing_account, max_orgs
                ),
                403,
            );
        }
    }

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| Error::RustError("Invalid JSON body".to_string()))?;

    let name = match body["name"].as_str() {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => return Response::error("Organization name is required", 400),
    };
    if name.len() > 100 {
        return Response::error("Organization name must be 100 characters or less", 400);
    }

    // Create the org linked to user's billing account (inherits tier)
    let org = db::create_organization_with_billing_account(
        &db,
        &name,
        &user_ctx.user_id,
        &billing_account.id,
    )
    .await?;
    db::add_org_member(&db, &org.id, &user_ctx.user_id, "owner").await?;

    // Issue a new access token scoped to the new org
    let kv = ctx.kv("URL_MAPPINGS")?;
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let new_access_token = auth::session::create_access_token(
        &user_ctx.user_id,
        &org.id,
        &user_ctx.session_id,
        &user_ctx.role,
        &jwt_secret,
    )?;

    // Update session KV to the new org
    auth::session::store_session(&kv, &user_ctx.session_id, &user_ctx.user_id, &org.id).await?;

    let domain = ctx
        .env
        .var("DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "localhost:8787".to_string());
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let access_cookie = auth::session::create_access_cookie_with_scheme(&new_access_token, scheme);

    let mut response = Response::from_json(&serde_json::json!({
        "org": org,
        "role": "owner",
    }))?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    Ok(response)
}

// ─── POST /api/auth/switch-org ──────────────────────────────────────────────
/// Switch the active org context: re-issues access token for the chosen org
pub async fn handle_switch_org(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| Error::RustError("Invalid JSON body".to_string()))?;

    let target_org_id = match body["org_id"].as_str() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return Response::error("org_id is required", 400),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify the user is actually a member of the target org
    let member = db::get_org_member(&db, &target_org_id, &user_ctx.user_id).await?;
    if member.is_none() {
        return Response::error("You are not a member of this organization", 403);
    }

    let org = db::get_org_by_id(&db, &target_org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    let kv = ctx.kv("URL_MAPPINGS")?;
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();

    let new_access_token = auth::session::create_access_token(
        &user_ctx.user_id,
        &target_org_id,
        &user_ctx.session_id,
        &user_ctx.role,
        &jwt_secret,
    )?;

    // Update session KV to reflect the new active org
    auth::session::store_session(&kv, &user_ctx.session_id, &user_ctx.user_id, &target_org_id)
        .await?;

    let domain = ctx
        .env
        .var("DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "localhost:8787".to_string());
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let access_cookie = auth::session::create_access_cookie_with_scheme(&new_access_token, scheme);

    let member_info = member.unwrap();
    let mut response = Response::from_json(&serde_json::json!({
        "org": {
            "id": org.id,
            "name": org.name,
            "tier": org.tier,
            "role": member_info.role,
        },
    }))?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    Ok(response)
}

// ─── GET /api/orgs/:id ──────────────────────────────────────────────────────
/// Get org details (members + pending invitations)
pub async fn handle_get_org(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify user is a member of this org
    let member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    let member = match member {
        Some(m) => m,
        None => return Response::error("Organization not found", 404),
    };

    let org = db::get_org_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    let members = db::get_org_members(&db, &org_id).await?;

    // Owners and admins see pending invitations
    let pending_invitations = if member.role == "owner" || member.role == "admin" {
        db::list_org_invitations(&db, &org_id).await?
    } else {
        vec![]
    };

    Response::from_json(&serde_json::json!({
        "org": {
            "id": org.id,
            "name": org.name,
            "tier": org.tier,
            "created_at": org.created_at,
            "role": member.role,
        },
        "members": members,
        "pending_invitations": pending_invitations,
    }))
}

// ─── PATCH /api/orgs/:id ────────────────────────────────────────────────────
/// Rename an organization (owner or admin only)
pub async fn handle_update_org(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify user is an owner or admin of this org
    let member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => {}
        Some(_) => {
            return Response::error(
                "Only org owners and admins can rename the organization",
                403,
            );
        }
        None => return Response::error("Organization not found", 404),
    }

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| Error::RustError("Invalid JSON body".to_string()))?;

    let name = match body["name"].as_str() {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => return Response::error("Organization name is required", 400),
    };
    if name.len() > 100 {
        return Response::error("Organization name must be 100 characters or less", 400);
    }

    db::update_org_name(&db, &org_id, &name).await?;

    let updated_org = db::get_org_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found after update".to_string()))?;

    Response::from_json(&serde_json::json!({ "org": updated_org }))
}

// ─── DELETE /api/orgs/:id/members/:user_id ──────────────────────────────────
/// Remove a member from an org
/// - Owners can remove anyone except the last owner
/// - Admins can remove members but not owners
/// - Anyone can remove themselves
pub async fn handle_remove_member(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();
    let target_user_id = ctx
        .param("user_id")
        .ok_or_else(|| Error::RustError("Missing user_id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let requester_member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    let requester = match requester_member {
        Some(m) => m,
        None => return Response::error("Organization not found", 404),
    };

    let is_self_removal = target_user_id == user_ctx.user_id;

    // Check the target is actually in this org
    let target_member = db::get_org_member(&db, &org_id, &target_user_id).await?;
    if target_member.is_none() {
        return Response::error("Member not found in this organization", 404);
    }
    let target_role = target_member.unwrap().role;

    // Permission checks for removing others
    if !is_self_removal {
        match requester.role.as_str() {
            "owner" => {
                // Owners can remove anyone
            }
            "admin" => {
                // Admins can remove members but not owners or other admins
                if target_role == "owner" {
                    return Response::error("Admins cannot remove owners", 403);
                }
                if target_role == "admin" {
                    return Response::error("Admins cannot remove other admins", 403);
                }
            }
            _ => {
                // Regular members can only remove themselves
                return Response::error("Only org owners and admins can remove members", 403);
            }
        }
    }

    // Prevent removing the last owner
    if target_role == "owner" {
        let owner_count = db::count_org_owners(&db, &org_id).await?;
        if owner_count <= 1 {
            return Response::error(
                "Cannot remove the last owner. Transfer ownership first.",
                400,
            );
        }
    }

    db::remove_org_member(&db, &org_id, &target_user_id).await?;

    Response::ok("Member removed")
}

// ─── POST /api/orgs/:id/invitations ─────────────────────────────────────────
/// Invite a user by email to an org (owner/admin only, respects tier member limits)
pub async fn handle_create_invitation(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify user is an owner or admin of this org
    let member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => {}
        Some(_) => return Response::error("Only org owners and admins can invite members", 403),
        None => return Response::error("Organization not found", 404),
    }

    // Get org and its tier limits
    let org = db::get_org_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    let tier = Tier::from_str_value(&org.tier).unwrap_or(Tier::Free);
    let limits = tier.limits();

    // Check member limits if tier has them
    if let Some(max_members) = limits.max_members {
        let current_members = db::count_org_members(&db, &org_id).await?;
        let pending_invites = db::count_pending_invitations(&db, &org_id).await?;
        let total_committed = current_members + pending_invites;

        if total_committed >= max_members {
            return Response::error(
                format!(
                    "Member limit reached ({}/{}). Upgrade your plan to invite more members.",
                    total_committed, max_members
                ),
                403,
            );
        }
    }

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| Error::RustError("Invalid JSON body".to_string()))?;

    let email = match body["email"].as_str() {
        Some(e) if !e.trim().is_empty() => e.trim().to_lowercase(),
        _ => return Response::error("Email is required", 400),
    };

    // Validate basic email format
    if !email.contains('@') || email.len() > 254 {
        return Response::error("Invalid email address", 400);
    }

    // Check not already a member
    if let Some(existing_user) = db::get_user_by_email(&db, &email).await?
        && db::get_org_member(&db, &org_id, &existing_user.id)
            .await?
            .is_some()
    {
        return Response::error("This user is already a member of the organization", 409);
    }

    // Check no pending invite already exists
    if db::pending_invite_exists(&db, &org_id, &email).await? {
        return Response::error("A pending invitation already exists for this email", 409);
    }

    // Get inviter's info for the email
    let inviter = db::get_user_by_id(&db, &user_ctx.user_id)
        .await?
        .ok_or_else(|| Error::RustError("Inviter not found".to_string()))?;
    let inviter_name = inviter
        .name
        .as_deref()
        .unwrap_or(&inviter.email)
        .to_string();

    let invitation =
        db::create_org_invitation(&db, &org_id, &user_ctx.user_id, &email, "member").await?;

    // Send the invitation email
    let frontend_url = crate::router::get_frontend_url(&ctx.env);
    let invite_url = format!("{}/invite/{}", frontend_url, invitation.id);

    if let Err(e) =
        send_org_invitation(&ctx.env, &email, &inviter_name, &org.name, &invite_url).await
    {
        console_log!(
            "{}",
            serde_json::json!({
                "event": "invitation_email_failed",
                "org_id": org_id,
                "email": email,
                "error": e.to_string(),
                "level": "error",
            })
        );
        // Still return success – the invitation is stored; admin can resend later
    }

    Response::from_json(&serde_json::json!({ "invitation": invitation }))
}

// ─── DELETE /api/orgs/:id/invitations/:invitation_id ────────────────────────
/// Revoke a pending invitation (owner or admin only)
pub async fn handle_revoke_invitation(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();
    let invitation_id = ctx
        .param("invitation_id")
        .ok_or_else(|| Error::RustError("Missing invitation_id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify user is an owner or admin of this org
    let member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => {}
        Some(_) => {
            return Response::error("Only org owners and admins can revoke invitations", 403);
        }
        None => return Response::error("Organization not found", 404),
    }

    // Make sure the invitation belongs to this org
    let invitation = db::get_invitation_by_token(&db, &invitation_id).await?;
    match invitation {
        Some(inv) if inv.org_id == org_id => {}
        Some(_) => return Response::error("Invitation not found in this organization", 404),
        None => return Response::error("Invitation not found", 404),
    }

    db::revoke_invitation(&db, &invitation_id).await?;
    Response::ok("Invitation revoked")
}

// ─── POST /api/orgs/:id/invitations/:invitation_id/resend ────────────────────
/// Resend a pending invitation email (owner or admin only)
pub async fn handle_resend_invitation(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();
    let invitation_id = ctx
        .param("invitation_id")
        .ok_or_else(|| Error::RustError("Missing invitation_id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify user is an owner or admin of this org
    let member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => {}
        Some(_) => {
            return Response::error("Only org owners and admins can resend invitations", 403);
        }
        None => return Response::error("Organization not found", 404),
    }

    // Get the invitation and verify it belongs to this org
    let invitation = match db::get_invitation_by_token(&db, &invitation_id).await? {
        Some(inv) if inv.org_id == org_id => inv,
        Some(_) => return Response::error("Invitation not found in this organization", 404),
        None => return Response::error("Invitation not found", 404),
    };

    // Check invitation hasn't expired or been accepted
    let now = crate::utils::now_timestamp();
    if invitation.accepted_at.is_some() {
        return Response::error("Invitation has already been accepted", 400);
    }
    if invitation.expires_at < now {
        return Response::error("Invitation has expired", 400);
    }

    // Get org and inviter info for the email
    let org = db::get_org_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    let inviter = db::get_user_by_id(&db, &user_ctx.user_id)
        .await?
        .ok_or_else(|| Error::RustError("Inviter not found".to_string()))?;
    let inviter_name = inviter
        .name
        .as_deref()
        .unwrap_or(&inviter.email)
        .to_string();

    // Send the invitation email
    let frontend_url = crate::router::get_frontend_url(&ctx.env);
    let invite_url = format!("{}/invite/{}", frontend_url, invitation.id);

    if let Err(e) = send_org_invitation(
        &ctx.env,
        &invitation.email,
        &inviter_name,
        &org.name,
        &invite_url,
    )
    .await
    {
        console_log!(
            "{}",
            serde_json::json!({
                "event": "invitation_email_failed",
                "org_id": org_id,
                "email": invitation.email,
                "error": e.to_string(),
                "level": "error",
            })
        );
        return Response::error("Failed to send invitation email", 500);
    }

    Response::from_json(&serde_json::json!({
        "success": true,
        "message": "Invitation email resent",
    }))
}

// ─── GET /api/invite/:token (public) ────────────────────────────────────────
/// Validate an invite token and return org/inviter info (no auth required)
pub async fn handle_get_invite_info(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let token = ctx
        .param("token")
        .ok_or_else(|| Error::RustError("Missing token".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let invitation = match db::get_invitation_by_token(&db, &token).await? {
        Some(inv) => inv,
        None => {
            return Response::from_json(
                &serde_json::json!({ "valid": false, "reason": "not_found" }),
            );
        }
    };

    let now = crate::utils::now_timestamp();

    if invitation.accepted_at.is_some() {
        return Response::from_json(
            &serde_json::json!({ "valid": false, "reason": "already_accepted" }),
        );
    }
    if invitation.expires_at < now {
        return Response::from_json(&serde_json::json!({ "valid": false, "reason": "expired" }));
    }

    let org = db::get_org_by_id(&db, &invitation.org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    let inviter = db::get_user_by_id(&db, &invitation.invited_by).await?;
    let inviter_name = inviter
        .and_then(|u| u.name.or(Some(u.email)))
        .unwrap_or_else(|| "A team member".to_string());

    Response::from_json(&serde_json::json!({
        "valid": true,
        "org_name": org.name,
        "invited_by": inviter_name,
        "email": invitation.email,
        "expires_at": invitation.expires_at,
    }))
}

// ─── POST /api/invite/:token/accept (auth required) ─────────────────────────
/// Accept an invitation: validates token, adds user to org, re-issues access token
pub async fn handle_accept_invite(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let token = ctx
        .param("token")
        .ok_or_else(|| Error::RustError("Missing token".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let invitation = match db::get_invitation_by_token(&db, &token).await? {
        Some(inv) => inv,
        None => return Response::error("Invitation not found", 404),
    };

    let now = crate::utils::now_timestamp();

    if invitation.accepted_at.is_some() {
        return Response::error("This invitation has already been accepted", 409);
    }
    if invitation.expires_at < now {
        return Response::error("This invitation has expired", 410);
    }

    // Get the accepting user and verify their email matches the invite
    let user = db::get_user_by_id(&db, &user_ctx.user_id)
        .await?
        .ok_or_else(|| Error::RustError("User not found".to_string()))?;

    if user.email.to_lowercase() != invitation.email.to_lowercase() {
        return Response::error("This invitation was sent to a different email address", 403);
    }

    // Check not already a member
    if db::get_org_member(&db, &invitation.org_id, &user_ctx.user_id)
        .await?
        .is_some()
    {
        return Response::error("You are already a member of this organization", 409);
    }

    // Add user to org and mark invitation accepted
    // First add the member, then mark the invitation as accepted
    db::add_org_member(&db, &invitation.org_id, &user_ctx.user_id, &invitation.role).await?;

    // Verify the member was actually added
    if db::get_org_member(&db, &invitation.org_id, &user_ctx.user_id)
        .await?
        .is_none()
    {
        return Response::error("Failed to add member to organization", 500);
    }

    // Now mark the invitation as accepted
    db::accept_invitation(&db, &token).await?;

    let _org = db::get_org_by_id(&db, &invitation.org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    // Issue new access token scoped to the newly joined org
    let kv = ctx.kv("URL_MAPPINGS")?;
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let new_access_token = auth::session::create_access_token(
        &user_ctx.user_id,
        &invitation.org_id,
        &user_ctx.session_id,
        &user_ctx.role,
        &jwt_secret,
    )?;

    auth::session::store_session(
        &kv,
        &user_ctx.session_id,
        &user_ctx.user_id,
        &invitation.org_id,
    )
    .await?;

    let domain = ctx
        .env
        .var("DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "localhost:8787".to_string());
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let access_cookie = auth::session::create_access_cookie_with_scheme(&new_access_token, scheme);

    // Return JSON response with org info
    let mut response = Response::from_json(&serde_json::json!({
        "org": {
            "id": _org.id,
            "name": _org.name,
            "tier": _org.tier,
            "role": invitation.role,
        },
    }))?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    Ok(response)
}

// ─── DELETE /api/orgs/:id ───────────────────────────────────────────────────
/// Delete an organization (owner only, must have multiple orgs)
/// Supports migrating links to another org or deleting them
pub async fn handle_delete_org(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // ... (rest of the code remains the same)
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let kv = ctx.kv("URL_MAPPINGS")?;

    // Verify user is an owner of this org
    let member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" => {}
        Some(_) => return Response::error("Only org owners can delete the organization", 403),
        None => return Response::error("Organization not found", 404),
    }

    // Verify user has multiple orgs
    let owned_orgs = db::count_user_owned_orgs(&db, &user_ctx.user_id).await?;
    if owned_orgs <= 1 {
        return Response::error("Cannot delete your only organization", 400);
    }

    // Parse request body for deletion options
    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| Error::RustError("Invalid JSON body".to_string()))?;

    let action = body["action"]
        .as_str()
        .ok_or_else(|| Error::RustError("action is required (delete or migrate)".to_string()))?;

    match action {
        "delete" => {
            // Get all link IDs and their short codes for KV cleanup
            let link_ids = db::get_org_link_ids(&db, &org_id).await?;

            // Delete from KV first (makes links unreachable)
            for link_id in &link_ids {
                // Get the short_code from the link
                if let Some(link) = db::get_link_by_id(&db, link_id, &org_id).await? {
                    let _ = kv.delete(&link.short_code).await; // Ignore KV errors
                }
            }

            // Hard delete links and analytics from D1
            // (analytics first, then links, to satisfy FK constraints)
            db::delete_org_links(&db, &org_id).await?;
        }
        "migrate" => {
            let target_org_id = body["target_org_id"].as_str().ok_or_else(|| {
                Error::RustError("target_org_id is required when action is migrate".to_string())
            })?;

            // Verify user is owner of target org
            let target_member = db::get_org_member(&db, target_org_id, &user_ctx.user_id).await?;
            match &target_member {
                Some(m) if m.role == "owner" => {}
                Some(_) => {
                    return Response::error("You must be an owner of the target organization", 403);
                }
                None => return Response::error("Target organization not found", 404),
            }

            // Get target org and check capacity at billing account level
            let target_org = db::get_org_by_id(&db, target_org_id)
                .await?
                .ok_or_else(|| Error::RustError("Target organization not found".to_string()))?;

            let target_billing_account_id =
                target_org.billing_account_id.as_ref().ok_or_else(|| {
                    Error::RustError("Target organization has no billing account".to_string())
                })?;
            let target_billing_account = db::get_billing_account(&db, target_billing_account_id)
                .await?
                .ok_or_else(|| Error::RustError("Billing account not found".to_string()))?;

            let target_tier =
                Tier::from_str_value(&target_billing_account.tier).unwrap_or(Tier::Free);
            let target_limits = target_tier.limits();

            // Count links in source org
            let source_link_count = db::count_org_links(&db, &org_id).await?;

            // Check if target billing account has enough capacity
            if let Some(max_links) = target_limits.max_links_per_month {
                // Get current month's usage at billing account level
                let now = chrono::Utc::now();
                let year_month = format!("{}-{:02}", now.year(), now.month());
                let target_current_usage = db::get_monthly_counter_for_billing_account(
                    &db,
                    target_billing_account_id,
                    &year_month,
                )
                .await?;

                let target_available = max_links - target_current_usage;

                if source_link_count > target_available {
                    return Response::error(
                        format!(
                            "Target billing account has insufficient capacity. Available slots: {}, Required: {}",
                            target_available, source_link_count
                        ),
                        400,
                    );
                }

                // Update target billing account's monthly counter
                db::increment_monthly_counter_for_billing_account(
                    &db,
                    target_billing_account_id,
                    &year_month,
                    source_link_count,
                )
                .await?;
            }

            // Migrate links in D1
            db::migrate_org_links(&db, &org_id, target_org_id).await?;

            // Update KV mappings (they store org_id in the LinkMapping)
            let link_ids = db::get_org_link_ids(&db, target_org_id).await?;
            for link_id in &link_ids {
                if let Some(link) = db::get_link_by_id(&db, link_id, target_org_id).await? {
                    let mapping = link.to_mapping();
                    let _ = kv.put(&link.short_code, mapping)?.execute().await; // Ignore KV errors
                }
            }
        }
        _ => {
            return Response::error("Invalid action. Must be 'delete' or 'migrate'", 400);
        }
    }

    // Delete the organization (cascades to members, invitations, counters)
    db::delete_organization(&db, &org_id).await?;

    // Find another owned org to switch to
    let user_orgs = db::get_user_orgs(&db, &user_ctx.user_id).await?;
    let fallback_org = user_orgs
        .into_iter()
        .find(|o| o.role == "owner" && o.id != org_id)
        .ok_or_else(|| Error::RustError("No other owned organization found".to_string()))?;

    // Issue new access token scoped to the fallback org
    let jwt_secret = ctx.env.secret("JWT_SECRET")?.to_string();
    let new_access_token = auth::session::create_access_token(
        &user_ctx.user_id,
        &fallback_org.id,
        &user_ctx.session_id,
        &user_ctx.role,
        &jwt_secret,
    )?;

    auth::session::store_session(
        &kv,
        &user_ctx.session_id,
        &user_ctx.user_id,
        &fallback_org.id,
    )
    .await?;

    let domain = ctx
        .env
        .var("DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "localhost:8787".to_string());
    let scheme = if domain.starts_with("localhost") {
        "http"
    } else {
        "https"
    };
    let access_cookie = auth::session::create_access_cookie_with_scheme(&new_access_token, scheme);

    let mut response = Response::from_json(&serde_json::json!({
        "success": true,
        "switched_to_org": {
            "id": fallback_org.id,
            "name": fallback_org.name,
        },
    }))?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    Ok(response)
}
