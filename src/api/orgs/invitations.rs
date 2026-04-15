/// Org invitation handlers
///
/// POST /api/orgs/{id}/invitations - Create invitation
/// DELETE /api/orgs/{id}/invitations/{invitation_id} - Revoke invitation
/// POST /api/orgs/{id}/invitations/{invitation_id}/resend - Resend invitation
/// GET /api/invite/{token} - Get invite info (public)
/// POST /api/invite/{token}/accept - Accept invite
use crate::auth;
use crate::db;
use crate::models::Tier;
use crate::repositories::{OrgRepository, UserRepository};
use crate::router::get_frontend_url;
use crate::utils::AppError;
use crate::utils::email::send_org_invitation;
use worker::d1::D1Database;
use worker::*;

async fn require_owner_or_admin(
    repo: &OrgRepository,
    db: &D1Database,
    org_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let member = repo.get_member(db, org_id, user_id).await?;
    match &member {
        Some(m) if m.role == "owner" || m.role == "admin" => Ok(()),
        Some(_) => Err(AppError::Forbidden(
            "Only org owners and admins can manage invitations".to_string(),
        )),
        None => Err(AppError::NotFound("Organization not found".to_string())),
    }
}

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/invitations",
    tag = "Organizations",
    summary = "Invite a member",
    responses(
        (status = 200, description = "Invitation created"),
        (status = 400, description = "Invalid email"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Insufficient permissions or limit reached"),
        (status = 409, description = "Already member or pending invite"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_create_invitation(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_create_invitation(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_create_invitation(
    mut req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();
    require_owner_or_admin(&repo, &db, &org_id, &user_ctx.user_id).await?;

    let org = repo
        .get_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
    let tier = if let Some(ref ba_id) = org.billing_account_id {
        if let Ok(Some(ba)) = db::get_billing_account(&db, ba_id).await {
            Tier::from_str_value(&ba.tier).unwrap_or(Tier::Free)
        } else {
            Tier::Free
        }
    } else {
        Tier::Free
    };
    let limits = tier.limits();

    if let Some(max_members) = limits.max_members {
        let current_members = repo.count_members(&db, &org_id).await?;
        let pending_invites = repo.count_pending_invitations(&db, &org_id).await?;
        if current_members + pending_invites >= max_members {
            return Err(AppError::Forbidden(format!(
                "Member limit reached ({}/{})",
                current_members + pending_invites,
                max_members
            )));
        }
    }

    let body: serde_json::Value = req
        .json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid JSON".to_string()))?;
    let email = match body["email"].as_str() {
        Some(e) if !e.trim().is_empty() => e.trim().to_lowercase(),
        _ => return Err(AppError::BadRequest("Email is required".to_string())),
    };
    if !email.contains('@') || email.len() > 254 {
        return Err(AppError::BadRequest("Invalid email address".to_string()));
    }

    let user_repo = UserRepository::new();
    if let Some(existing_user) = user_repo.get_by_email(&db, &email).await?
        && repo
            .get_member(&db, &org_id, &existing_user.id)
            .await?
            .is_some()
    {
        return Err(AppError::Conflict("User is already a member".to_string()));
    }
    if repo.pending_invite_exists(&db, &org_id, &email).await? {
        return Err(AppError::Conflict(
            "A pending invitation already exists".to_string(),
        ));
    }

    let inviter = user_repo
        .get_user_by_id(&db, &user_ctx.user_id)
        .await?
        .ok_or_else(|| AppError::Internal("Inviter not found".to_string()))?;
    let inviter_name = inviter
        .name
        .as_deref()
        .unwrap_or(&inviter.email)
        .to_string();
    let invitation = repo
        .create_invitation(&db, &org_id, &user_ctx.user_id, &email, "member")
        .await?;

    let frontend_url = get_frontend_url(&ctx.env);
    let invite_url = format!("{}/invite/{}", frontend_url, invitation.id);
    if let Err(e) =
        send_org_invitation(&ctx.env, &email, &inviter_name, &org.name, &invite_url).await
    {
        console_log!(
            "{{\"event\":\"invitation_email_failed\",\"org_id\":\"{}\",\"email\":\"{}\",\"error\":\"{}\"}}",
            org_id,
            email,
            e
        );
    }
    Ok(Response::from_json(
        &serde_json::json!({ "invitation": invitation }),
    )?)
}

#[utoipa::path(
    delete,
    path = "/api/orgs/{id}/invitations/{invitation_id}",
    tag = "Organizations",
    summary = "Revoke an invitation",
    responses(
        (status = 200, description = "Invitation revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Invitation not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_revoke_invitation(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_revoke_invitation(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_revoke_invitation(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();
    let invitation_id = ctx
        .param("invitation_id")
        .ok_or_else(|| AppError::BadRequest("Missing invitation_id".to_string()))?
        .to_string();
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();
    require_owner_or_admin(&repo, &db, &org_id, &user_ctx.user_id).await?;

    let invitation = repo.get_invitation_by_token(&db, &invitation_id).await?;
    match invitation {
        Some(inv) if inv.org_id == org_id => {}
        Some(_) => {
            return Err(AppError::NotFound(
                "Invitation not found in this organization".to_string(),
            ));
        }
        None => return Err(AppError::NotFound("Invitation not found".to_string())),
    }
    repo.revoke_invitation(&db, &invitation_id).await?;
    Ok(Response::ok("Invitation revoked")?)
}

#[utoipa::path(
    post,
    path = "/api/orgs/{id}/invitations/{invitation_id}/resend",
    tag = "Organizations",
    summary = "Resend an invitation",
    responses(
        (status = 200, description = "Invitation resent"),
        (status = 400, description = "Invitation expired or accepted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Invitation not found"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_resend_invitation(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_resend_invitation(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_resend_invitation(
    req: Request,
    ctx: RouteContext<()>,
) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let org_id = ctx
        .param("id")
        .ok_or_else(|| AppError::BadRequest("Missing org id".to_string()))?
        .to_string();
    let invitation_id = ctx
        .param("invitation_id")
        .ok_or_else(|| AppError::BadRequest("Missing invitation_id".to_string()))?
        .to_string();
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();
    let user_repo = UserRepository::new();
    require_owner_or_admin(&repo, &db, &org_id, &user_ctx.user_id).await?;

    let invitation = repo
        .get_invitation_by_token(&db, &invitation_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Invitation not found".to_string()))?;
    if invitation.org_id != org_id {
        return Err(AppError::NotFound(
            "Invitation not found in this organization".to_string(),
        ));
    }
    let now = crate::utils::now_timestamp();
    if invitation.accepted_at.is_some() {
        return Err(AppError::BadRequest(
            "Invitation has already been accepted".to_string(),
        ));
    }
    if invitation.expires_at < now {
        return Err(AppError::BadRequest("Invitation has expired".to_string()));
    }

    let org = repo
        .get_by_id(&db, &org_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;
    let inviter = user_repo
        .get_user_by_id(&db, &user_ctx.user_id)
        .await?
        .ok_or_else(|| AppError::Internal("Inviter not found".to_string()))?;
    let inviter_name = inviter
        .name
        .as_deref()
        .unwrap_or(&inviter.email)
        .to_string();

    let frontend_url = get_frontend_url(&ctx.env);
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
        console_log!("{{\"event\":\"resend_failed\",\"error\":\"{}\"}}", e);
        return Err(AppError::Internal(
            "Failed to send invitation email".to_string(),
        ));
    }
    Ok(Response::from_json(
        &serde_json::json!({"success":true,"message":"Invitation email resent"}),
    )?)
}

#[utoipa::path(
    get,
    path = "/api/invite/{token}",
    tag = "Organizations",
    summary = "Get invite info",
    description = "Public endpoint - no authentication required",
    responses(
        (status = 200, description = "Invitation details"),
        (status = 404, description = "Not found, expired, or already accepted"),
    )
)]
pub async fn handle_get_invite_info(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let token = ctx
        .param("token")
        .ok_or_else(|| Error::RustError("Missing token".to_string()))?
        .to_string();
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();
    let user_repo = UserRepository::new();

    let invitation = match repo.get_invitation_by_token(&db, &token).await? {
        Some(inv) => inv,
        None => {
            return Response::from_json(&serde_json::json!({"valid":false,"reason":"not_found"}));
        }
    };
    let now = crate::utils::now_timestamp();
    if invitation.accepted_at.is_some() {
        return Response::from_json(
            &serde_json::json!({"valid":false,"reason":"already_accepted"}),
        );
    }
    if invitation.expires_at < now {
        return Response::from_json(&serde_json::json!({"valid":false,"reason":"expired"}));
    }
    let org = repo
        .get_by_id(&db, &invitation.org_id)
        .await?
        .ok_or_else(|| Error::RustError("Organization not found".to_string()))?;
    let inviter = user_repo
        .get_user_by_id(&db, &invitation.invited_by)
        .await?;
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
    responses(
        (status = 200, description = "Invitation accepted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Invitation sent to a different email address"),
        (status = 404, description = "Invitation not found"),
        (status = 409, description = "Invitation already accepted"),
        (status = 410, description = "Invitation expired"),
    ),
    security(("Bearer" = []), ("session_cookie" = []))
)]
pub async fn handle_accept_invite(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    Ok(inner_accept_invite(req, ctx)
        .await
        .unwrap_or_else(|e| e.into_response()))
}

async fn inner_accept_invite(req: Request, ctx: RouteContext<()>) -> Result<Response, AppError> {
    let user_ctx = auth::authenticate_request(&req, &ctx).await?;
    let token = ctx
        .param("token")
        .ok_or_else(|| AppError::BadRequest("Missing token".to_string()))?
        .to_string();
    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = OrgRepository::new();
    let user_repo = UserRepository::new();

    let invitation = repo
        .get_invitation_by_token(&db, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("Invitation not found".to_string()))?;
    let now = crate::utils::now_timestamp();

    if invitation.accepted_at.is_some() {
        return Err(AppError::Conflict(
            "This invitation has already been accepted".to_string(),
        ));
    }
    if invitation.expires_at < now {
        return Err(AppError::BadRequest(
            "This invitation has expired".to_string(),
        ));
    }

    let user = user_repo
        .get_user_by_id(&db, &user_ctx.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    if user.email.to_lowercase() != invitation.email.to_lowercase() {
        return Err(AppError::Forbidden(
            "This invitation was sent to a different email address".to_string(),
        ));
    }

    if repo
        .get_member(&db, &invitation.org_id, &user_ctx.user_id)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "You are already a member of this organization".to_string(),
        ));
    }

    repo.add_member(&db, &invitation.org_id, &user_ctx.user_id, &invitation.role)
        .await?;
    if repo
        .get_member(&db, &invitation.org_id, &user_ctx.user_id)
        .await?
        .is_none()
    {
        return Err(AppError::Internal(
            "Failed to add member to organization".to_string(),
        ));
    }
    repo.accept_invitation(&db, &token).await?;

    let _org = repo
        .get_by_id(&db, &invitation.org_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".to_string()))?;

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

    let tier = if let Some(ref ba_id) = _org.billing_account_id {
        if let Ok(Some(ba)) = db::get_billing_account(&db, ba_id).await {
            Tier::from_str_value(&ba.tier).unwrap_or(Tier::Free)
        } else {
            Tier::Free
        }
    } else {
        Tier::Free
    };

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
