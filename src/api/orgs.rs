use crate::auth;
use crate::db;
use crate::models::Tier;
use crate::utils::email::send_org_invitation;
use chrono::Datelike;
use worker::d1::D1Database;
use worker::*;

/// Get the effective tier for an organization by looking up its billing account.
async fn get_org_tier(db: &D1Database, org: &crate::models::Organization) -> Tier {
    if let Some(ref billing_account_id) = org.billing_account_id
        && let Ok(Some(billing_account)) = db::get_billing_account(db, billing_account_id).await
    {
        return Tier::from_str_value(&billing_account.tier).unwrap_or(Tier::Free);
    }
    // Organization should always have a billing account after migration
    Tier::Free
}

#[utoipa::path(
    get,
    path = "/api/orgs",
    tag = "Organizations",
    summary = "List user organizations",
    description = "Returns all organizations the authenticated user belongs to, including their role in each org and the org's current billing tier. Also returns the active org_id from the current session",
    responses(
        (status = 200, description = "List of organizations with role and tier info"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_list_user_orgs(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let orgs = db::get_user_orgs(&db, &user_ctx.user_id).await?;

    // Add tier information to each org by looking up billing account
    let mut orgs_with_tier = Vec::new();
    for org in orgs {
        let tier = if let Some(org_details) = db::get_org_by_id(&db, &org.id).await? {
            if let Some(ref billing_account_id) = org_details.billing_account_id {
                if let Ok(Some(billing_account)) =
                    db::get_billing_account(&db, billing_account_id).await
                {
                    billing_account.tier
                } else {
                    "free".to_string()
                }
            } else {
                "free".to_string()
            }
        } else {
            "free".to_string()
        };

        orgs_with_tier.push(serde_json::json!({
            "id": org.id,
            "name": org.name,
            "tier": tier,
            "role": org.role,
            "joined_at": org.joined_at,
        }));
    }

    Response::from_json(&serde_json::json!({
        "orgs": orgs_with_tier,
        "current_org_id": user_ctx.org_id,
    }))
}

#[utoipa::path(
    post,
    path = "/api/orgs",
    tag = "Organizations",
    summary = "Create an organization",
    description = "Creates a new organization for the authenticated user. The user becomes the owner. Respects tier organization limits",
    responses(
        (status = 200, description = "New organization"),
        (status = 400, description = "Missing name"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Org limit reached for current tier"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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

#[utoipa::path(
    post,
    path = "/api/auth/switch-org",
    tag = "Organizations",
    summary = "Switch active organization",
    description = "Switches the authenticated user's active organization context. Re-issues a new access token (and refresh token) scoped to the chosen org. The user must be a member of the target org",
    responses(
        (status = 200, description = "Switched, new access token set in cookies"),
        (status = 400, description = "Missing or invalid org_id"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of the target org"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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

    // Get tier from billing account for API response
    let tier = get_org_tier(&db, &org).await;

    let mut response = Response::from_json(&serde_json::json!({
        "org": {
            "id": org.id,
            "name": org.name,
            "tier": tier.as_str(),
            "role": member_info.role,
        },
    }))?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/api/orgs/{id}",
    tag = "Organizations",
    summary = "Get organization",
    description = "Returns org details including the member list with roles and pending invitations. The caller must be a member of the org",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Organization with members and invitations"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this org"),
        (status = 404, description = "Organization not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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

    // Get tier from billing account for API response
    let tier = get_org_tier(&db, &org).await;

    // Owners and admins see pending invitations
    let pending_invitations = if member.role == "owner" || member.role == "admin" {
        db::list_org_invitations(&db, &org_id).await?
    } else {
        vec![]
    };

    let logo_url = db::get_org_logo_url(&db, &org_id).await.unwrap_or(None);

    Response::from_json(&serde_json::json!({
        "org": {
            "id": org.id,
            "name": org.name,
            "tier": tier.as_str(),
            "created_at": org.created_at,
            "role": member.role,
            "logo_url": logo_url,
        },
        "members": members,
        "pending_invitations": pending_invitations,
    }))
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{id}",
    tag = "Organizations",
    summary = "Update organization",
    description = "Renames an organization. Requires owner or admin role within the org",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Updated organization"),
        (status = 400, description = "Missing or empty name"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required"),
        (status = 404, description = "Organization not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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

    // Get tier from billing account for API response
    let tier = get_org_tier(&db, &updated_org).await;

    Response::from_json(&serde_json::json!({
        "org": {
            "id": updated_org.id,
            "name": updated_org.name,
            "tier": tier.as_str(),
            "created_at": updated_org.created_at,
            "billing_account_id": updated_org.billing_account_id,
        }
    }))
}

#[utoipa::path(
    get,
    path = "/api/orgs/{id}/settings",
    tag = "Organizations",
    summary = "Get org settings",
    description = "Returns organization-level settings. The query_forwarding_enabled setting is only available on Pro+ tiers",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Org settings"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this org"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_get_org_settings(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();

    // Only members of this org can read settings
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    if db::get_org_member(&db, &org_id, &user_ctx.user_id)
        .await?
        .is_none()
    {
        return Response::error("Organization not found", 404);
    }

    let forward_query_params = db::get_org_forward_query_params(&db, &org_id).await?;

    Response::from_json(&serde_json::json!({
        "forward_query_params": forward_query_params
    }))
}

#[utoipa::path(
    patch,
    path = "/api/orgs/{id}/settings",
    tag = "Organizations",
    summary = "Update org settings",
    description = "Updates organization-level settings. query_forwarding_enabled requires Pro+ tier. Caller must be owner or admin",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Updated settings"),
        (status = 400, description = "Invalid setting value"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required, or Pro+ required"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_update_org_settings(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify owner or admin
    let member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => {}
        Some(_) => {
            return Response::error(
                "Only org owners and admins can change organization settings",
                403,
            );
        }
        None => return Response::error("Organization not found", 404),
    }

    // Tier check: forward_query_params is Pro+ only
    let org = db::get_org_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;
    let tier = get_org_tier(&db, &org).await;
    let is_pro_or_above = matches!(tier, Tier::Pro | Tier::Business | Tier::Unlimited);

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| Error::RustError("Invalid JSON body".to_string()))?;

    if let Some(forward) = body["forward_query_params"].as_bool() {
        if forward && !is_pro_or_above {
            return Response::error(
                "Query parameter forwarding requires a Pro plan or above.",
                403,
            );
        }
        db::set_org_forward_query_params(&db, &org_id, forward).await?;
    } else {
        return Response::error("forward_query_params (boolean) is required", 400);
    }

    let updated_forward = db::get_org_forward_query_params(&db, &org_id).await?;

    Response::from_json(&serde_json::json!({
        "forward_query_params": updated_forward
    }))
}

// ─── DELETE /api/orgs/:id/members/:user_id ──────────────────────────────────
/// Remove a member from an org
/// - Owners can remove anyone except the last owner
/// - Admins can remove members but not owners
/// - Anyone can remove themselves
#[utoipa::path(
    delete,
    path = "/api/orgs/{id}/members/{user_id}",
    tag = "Organizations",
    summary = "Remove a member",
    description = "Removes a user from the organization. Owners can remove anyone except the last owner. Admins can remove members but not owners. Any member can remove themselves",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("user_id" = String, Path, description = "User ID to remove"),
    ),
    responses(
        (status = 200, description = "Member removed"),
        (status = 400, description = "Cannot remove last owner"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Insufficient role"),
        (status = 404, description = "Member not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/invitations",
    tag = "Organizations",
    summary = "Invite a member",
    description = "Sends an email invitation to join the organization. Requires owner or admin role. Respects tier member limits. Sends an invitation email with a unique token",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Invitation created and email sent"),
        (status = 400, description = "Invalid email or missing fields"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required, or member limit reached"),
        (status = 409, description = "Already a member or invitation pending"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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

    // Get org and check tier limits from billing account
    let org = db::get_org_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;

    let tier = get_org_tier(&db, &org).await;
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

#[utoipa::path(
    delete,
    path = "/api/orgs/{id}/invitations/{invitation_id}",
    tag = "Organizations",
    summary = "Revoke an invitation",
    description = "Cancels a pending invitation. Requires owner or admin role",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("invitation_id" = String, Path, description = "Invitation ID"),
    ),
    responses(
        (status = 200, description = "Invitation revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required"),
        (status = 404, description = "Invitation not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/invitations/{invitation_id}/resend",
    tag = "Organizations",
    summary = "Resend an invitation",
    description = "Resends the invitation email for a pending invitation. Requires owner or admin role",
    params(
        ("id" = String, Path, description = "Organization ID"),
        ("invitation_id" = String, Path, description = "Invitation ID"),
    ),
    responses(
        (status = 200, description = "Invitation email resent"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required"),
        (status = 404, description = "Invitation not found or already accepted"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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

#[utoipa::path(
    get,
    path = "/api/invite/{token}",
    tag = "Organizations",
    summary = "Get invite info",
    description = "Validates an invitation token and returns the organization name and inviter details. Public endpoint — no authentication required. Used by the accept-invite page to show context before the user logs in",
    params(
        ("token" = String, Path, description = "Invitation token"),
    ),
    responses(
        (status = 200, description = "Invitation details"),
        (status = 404, description = "Token not found, expired, or already accepted"),
    )
)]
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

#[utoipa::path(
    post,
    path = "/api/invite/{token}/accept",
    tag = "Organizations",
    summary = "Accept an invitation",
    description = "Accepts a pending invitation. Validates the token, verifies the caller's email matches the invited email, adds the user to the organization as the invited role, marks the invitation as accepted, and re-issues a new access token scoped to the new org",
    params(
        ("token" = String, Path, description = "Invitation token"),
    ),
    responses(
        (status = 200, description = "Invitation accepted, new access token set"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Caller email does not match invitation"),
        (status = 404, description = "Invitation not found or already accepted"),
        (status = 409, description = "Already a member of this org"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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
    // Get tier from billing account for API response
    let tier = get_org_tier(&db, &_org).await;

    let mut response = Response::from_json(&serde_json::json!({
        "org": {
            "id": _org.id,
            "name": _org.name,
            "tier": tier.as_str(),
            "role": invitation.role,
        },
    }))?;
    response.headers_mut().set("Set-Cookie", &access_cookie)?;
    Ok(response)
}

// ─── POST /api/orgs/:id/logo ─────────────────────────────────────────────────
/// Upload an org logo (owner/admin + Pro+ only).
/// Accepts multipart/form-data with a field named "logo".
/// Max 500 KB; accepted: image/png, image/jpeg, image/webp, image/svg+xml.
#[utoipa::path(
    post,
    path = "/api/orgs/{id}/logo",
    tag = "Organizations",
    summary = "Upload org logo",
    description = "Uploads an organization logo. Accepts multipart/form-data with a field named 'logo'. Max 500 KB. Accepted formats: image/png, image/jpeg, image/webp, image/svg+xml. Requires owner or admin role and Pro+ tier",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Logo uploaded, returns the URL"),
        (status = 400, description = "Missing file, too large, or unsupported format"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required, or Pro+ required"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_upload_org_logo(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify owner or admin
    let member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => {}
        Some(_) => {
            return Response::error("Only org owners and admins can upload a logo", 403);
        }
        None => return Response::error("Organization not found", 404),
    }

    // Tier check: Pro+ only
    let org = db::get_org_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;
    let tier = get_org_tier(&db, &org).await;
    if !matches!(tier, Tier::Pro | Tier::Business | Tier::Unlimited) {
        return Response::error("Custom org logo requires a Pro plan or above.", 403);
    }

    // Parse multipart body
    let form_data = req
        .form_data()
        .await
        .map_err(|_| Error::RustError("Failed to parse multipart form data".to_string()))?;

    let file_entry = form_data
        .get("logo")
        .ok_or_else(|| Error::RustError("Missing 'logo' field in form data".to_string()))?;

    let file = match file_entry {
        worker::FormEntry::File(f) => f,
        worker::FormEntry::Field(_) => {
            return Response::error("'logo' field must be a file upload", 400);
        }
    };

    let content_type = file.type_();
    let allowed_types = ["image/png", "image/jpeg", "image/webp", "image/svg+xml"];
    if !allowed_types.contains(&content_type.as_str()) {
        return Response::error("Invalid file type. Allowed: PNG, JPEG, WebP, SVG", 400);
    }

    let bytes = file
        .bytes()
        .await
        .map_err(|_| Error::RustError("Failed to read file bytes".to_string()))?;

    const MAX_BYTES: usize = 500 * 1024; // 500 KB
    if bytes.len() > MAX_BYTES {
        return Response::error("Logo file must be 500 KB or smaller", 400);
    }

    // Store in R2
    let bucket = ctx.env.bucket("ASSETS_BUCKET")?;
    let r2_key = format!("logos/{}", org_id);
    bucket
        .put(&r2_key, bytes)
        .custom_metadata([("content-type".to_string(), content_type.clone())])
        .execute()
        .await
        .map_err(|e| Error::RustError(format!("Failed to store logo: {e}")))?;

    // Persist the logo URL in D1 (path served via GET /api/orgs/:id/logo)
    let logo_url = format!("/api/orgs/{org_id}/logo");
    db::set_org_logo_url(&db, &org_id, Some(&logo_url)).await?;

    let origin = req.headers().get("Origin").ok().flatten();
    let response = Response::from_json(&serde_json::json!({ "logo_url": logo_url }))?;
    Ok(crate::add_cors_headers(response, origin, &ctx.env))
}

#[utoipa::path(
    get,
    path = "/api/orgs/{id}/logo",
    tag = "Organizations",
    summary = "Get org logo",
    description = "Serves the organization logo from R2 storage. Public endpoint — no authentication required",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Logo image"),
        (status = 404, description = "No logo found for this org"),
    )
)]
pub async fn handle_get_org_logo(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();

    let bucket = ctx.env.bucket("ASSETS_BUCKET")?;
    let r2_key = format!("logos/{}", org_id);

    let object = bucket
        .get(&r2_key)
        .execute()
        .await
        .map_err(|e| Error::RustError(format!("Failed to read logo: {e}")))?;

    match object {
        Some(obj) => {
            let metadata = obj.custom_metadata().unwrap_or_default();
            let content_type = metadata
                .get("content-type")
                .cloned()
                .unwrap_or_else(|| "image/png".to_string());

            let body = obj
                .body()
                .ok_or_else(|| Error::RustError("Empty object body".to_string()))?
                .bytes()
                .await
                .map_err(|e| Error::RustError(format!("Failed to read body: {e}")))?;

            let mut response = Response::from_bytes(body)?;
            let headers = response.headers_mut();
            headers.set("Content-Type", &content_type)?;
            headers.set("Cache-Control", "public, max-age=86400")?;
            // Public image resource — allow any origin so <img> and QR libraries can load it
            headers.set("Access-Control-Allow-Origin", "*")?;
            Ok(response)
        }
        None => Response::error("Logo not found", 404),
    }
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{id}/logo",
    tag = "Organizations",
    summary = "Delete org logo",
    description = "Removes the organization logo from R2 storage. Requires owner or admin role and Pro+ tier",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Logo deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner or admin required, or Pro+ required"),
        (status = 404, description = "No logo to delete"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_delete_org_logo(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };

    let org_id = ctx
        .param("id")
        .ok_or_else(|| Error::RustError("Missing org id".to_string()))?
        .to_string();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    // Verify owner or admin
    let member = db::get_org_member(&db, &org_id, &user_ctx.user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => {}
        Some(_) => {
            return Response::error("Only org owners and admins can delete the logo", 403);
        }
        None => return Response::error("Organization not found", 404),
    }

    // Delete from R2
    let bucket = ctx.env.bucket("ASSETS_BUCKET")?;
    let r2_key = format!("logos/{}", org_id);
    bucket
        .delete(&r2_key)
        .await
        .map_err(|e| Error::RustError(format!("Failed to delete logo: {e}")))?;

    // Clear URL in D1
    db::set_org_logo_url(&db, &org_id, None).await?;

    let origin = req.headers().get("Origin").ok().flatten();
    let response = Response::ok("Logo deleted")?;
    Ok(crate::add_cors_headers(response, origin, &ctx.env))
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{id}",
    tag = "Organizations",
    summary = "Delete an organization",
    description = "Permanently deletes an organization. Requires owner role and the user must belong to at least one other org. Links can either be migrated to another org or deleted",
    params(
        ("id" = String, Path, description = "Organization ID"),
    ),
    responses(
        (status = 200, description = "Organization deleted"),
        (status = 400, description = "Cannot delete last org or invalid migration target"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Owner required"),
        (status = 404, description = "Organization not found"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
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
                    let resolved_forward = link.forward_query_params.unwrap_or(false);
                    let mapping = link.to_mapping(resolved_forward);
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
