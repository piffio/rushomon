/// Route table for the Rushomon API
///
/// Registers all API and redirect routes on the Worker router.
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use worker::*;

// Thread-local storage for deferred analytics futures from redirect handlers.
// Workers are single-threaded, so thread_local is safe and avoids passing Context through the Router.
thread_local! {
    pub static DEFERRED_ANALYTICS: RefCell<Option<Pin<Box<dyn Future<Output = ()> + 'static>>>> = RefCell::new(None);
}

/// Register all routes and run the router against the incoming request.
pub async fn run(req: Request, env: Env, is_frontend_domain: bool) -> Result<Response> {
    let router = Router::new();

    router
        // Public redirect routes - must come first to catch short codes
        .get_async("/:code", move |req, route_ctx| async move {
            let code = route_ctx
                .param("code")
                .ok_or_else(|| Error::RustError("Missing short code".to_string()))?
                .to_string();

            // Skip API routes and known frontend routes on the frontend domain.
            // Frontend routes (dashboard, auth, settings, admin, 404) must not be
            // treated as short codes — they should fall through to the SPA fallback.
            // Without this, /404 would redirect to /404 in an infinite loop.
            if code.starts_with("api") {
                return Response::error("Not found", 404);
            }
            if is_frontend_domain
                && matches!(
                    code.as_str(),
                    "dashboard"
                        | "auth"
                        | "settings"
                        | "admin"
                        | "404"
                        | "login"
                        | "billing"
                        | "pricing"
                )
            {
                return Response::error("Not found", 404);
            }

            let result = crate::api::links::handle_redirect(req, route_ctx, code).await?;
            if let Some(future) = result.analytics_future {
                DEFERRED_ANALYTICS.with(|cell| cell.replace(Some(future)));
            }
            Ok(result.response)
        })
        .head_async("/:code", move |req, route_ctx| async move {
            let code = route_ctx
                .param("code")
                .ok_or_else(|| Error::RustError("Missing short code".to_string()))?
                .to_string();

            if code.starts_with("api") {
                return Response::error("Not found", 404);
            }
            if is_frontend_domain
                && matches!(
                    code.as_str(),
                    "dashboard"
                        | "auth"
                        | "settings"
                        | "admin"
                        | "404"
                        | "login"
                        | "billing"
                        | "pricing"
                )
            {
                return Response::error("Not found", 404);
            }

            let result = crate::api::links::handle_redirect(req, route_ctx, code).await?;
            if let Some(future) = result.analytics_future {
                DEFERRED_ANALYTICS.with(|cell| cell.replace(Some(future)));
            }
            Ok(result.response)
        })
        // Auth routes (public)
        .get_async(
            "/api/auth/providers",
            crate::api::auth::providers::handle_list_auth_providers,
        )
        .get_async(
            "/api/auth/github",
            crate::api::auth::oauth::handle_github_login,
        )
        .get_async(
            "/api/auth/google",
            crate::api::auth::oauth::handle_google_login,
        )
        .get_async(
            "/api/auth/callback",
            crate::api::auth::oauth::handle_oauth_callback,
        )
        // Version endpoint (public)
        .get_async("/api/version", crate::api::version::handle_version)
        // API routes - authentication required
        .get_async(
            "/api/auth/me",
            crate::api::auth::session::handle_get_current_user,
        )
        .post_async(
            "/api/auth/refresh",
            crate::api::auth::session::handle_token_refresh,
        )
        .post_async("/api/auth/logout", crate::api::auth::session::handle_logout)
        .get_async("/api/usage", crate::api::analytics::usage::handle_get_usage)
        .post_async("/api/links", crate::api::links::handle_create_link)
        .get_async("/api/links", crate::api::links::handle_list_links)
        .get_async("/api/links/export", crate::api::links::handle_export_links)
        .post_async("/api/links/import", crate::api::links::handle_import_links)
        .get_async(
            "/api/links/by-code/:code",
            crate::api::links::handle_get_link_by_code,
        )
        .get_async(
            "/api/links/:id/analytics",
            crate::api::analytics::link::handle_get_link_analytics,
        )
        .get_async("/api/links/:id", crate::api::links::handle_get_link)
        .put_async("/api/links/:id", crate::api::links::handle_update_link)
        .delete_async("/api/links/:id", crate::api::links::handle_delete_link)
        .post_async(
            "/api/admin/billing-accounts/:id/reset-counter",
            crate::api::admin::counters::handle_admin_reset_monthly_counter,
        )
        // Admin moderation routes
        .get_async(
            "/api/admin/links",
            crate::api::links::handle_admin_list_links,
        )
        .put_async(
            "/api/admin/links/:id",
            crate::api::links::handle_admin_update_link_status,
        )
        .delete_async(
            "/api/admin/links/:id",
            crate::api::links::handle_admin_delete_link,
        )
        .post_async(
            "/api/admin/links/:id/sync-kv",
            crate::api::links::handle_admin_sync_link_kv,
        )
        .post_async(
            "/api/admin/blacklist",
            crate::api::admin::blacklist::handle_admin_block_destination,
        )
        .get_async(
            "/api/admin/blacklist",
            crate::api::admin::blacklist::handle_admin_get_blacklist,
        )
        .delete_async(
            "/api/admin/blacklist/:id",
            crate::api::admin::blacklist::handle_admin_remove_blacklist,
        )
        .put_async(
            "/api/admin/users/:id/suspend",
            crate::api::admin::users::handle_admin_suspend_user,
        )
        .put_async(
            "/api/admin/users/:id/unsuspend",
            crate::api::admin::users::handle_admin_unsuspend_user,
        )
        .delete_async(
            "/api/admin/users/:id",
            crate::api::admin::users::handle_admin_delete_user,
        )
        // Admin report management routes
        .get_async(
            "/api/admin/reports",
            crate::api::reports::admin::handle_admin_get_reports,
        )
        .get_async(
            "/api/admin/reports/:id",
            crate::api::reports::admin::handle_admin_get_report,
        )
        .put_async(
            "/api/admin/reports/:id",
            crate::api::reports::admin::handle_admin_update_report,
        )
        .get_async(
            "/api/admin/reports/pending/count",
            crate::api::reports::admin::handle_admin_get_pending_reports_count,
        )
        // Admin billing account routes
        .get_async(
            "/api/admin/billing-accounts",
            crate::api::admin::billing::handle_admin_list_billing_accounts,
        )
        .get_async(
            "/api/admin/billing-accounts/:id",
            crate::api::admin::billing::handle_admin_get_billing_account,
        )
        .put_async(
            "/api/admin/billing-accounts/:id/tier",
            crate::api::admin::billing::handle_admin_update_billing_account_tier,
        )
        .put_async(
            "/api/admin/billing-accounts/:id/subscription",
            crate::api::admin::billing::handle_admin_update_subscription_status,
        )
        // Admin settings routes
        .get_async(
            "/api/admin/settings",
            crate::api::settings::admin::handle_admin_get_settings,
        )
        .put_async(
            "/api/admin/settings",
            crate::api::settings::admin::handle_admin_update_setting,
        )
        // Admin discounts and products routes
        .get_async(
            "/api/admin/discounts",
            crate::api::billing::products::handle_admin_list_discounts,
        )
        .get_async(
            "/api/admin/products",
            crate::api::billing::products::handle_admin_list_products,
        )
        .post_async(
            "/api/admin/products/sync",
            crate::api::billing::products::handle_admin_sync_products,
        )
        .post_async(
            "/api/admin/products/save",
            crate::api::billing::products::handle_admin_save_products,
        )
        // Admin API keys routes
        .get_async(
            "/api/admin/api-keys",
            crate::api::admin::api_keys::handle_admin_list_api_keys,
        )
        .delete_async(
            "/api/admin/api-keys/:id",
            crate::api::admin::api_keys::handle_admin_revoke_api_key,
        )
        .post_async(
            "/api/admin/api-keys/:id/delete",
            crate::api::admin::api_keys::handle_admin_delete_api_key,
        )
        .post_async(
            "/api/admin/api-keys/:id/restore",
            crate::api::admin::api_keys::handle_admin_restore_api_key,
        )
        .post_async(
            "/api/admin/api-keys/:id/reactivate",
            crate::api::admin::api_keys::handle_admin_reactivate_api_key,
        )
        // Admin users routes
        .get_async(
            "/api/admin/users",
            crate::api::admin::users::handle_admin_list_users,
        )
        .get_async(
            "/api/admin/users/:id",
            crate::api::admin::users::handle_admin_get_user,
        )
        .put_async(
            "/api/admin/users/:id",
            crate::api::admin::users::handle_admin_update_user_role,
        )
        // Abuse report route (public, can be called by anyone)
        .post_async(
            "/api/reports/links",
            crate::api::reports::create::handle_report_link,
        )
        // Tags routes
        .get_async("/api/tags", crate::api::tags::handle_get_org_tags)
        .delete_async("/api/tags/:name", crate::api::tags::handle_delete_org_tag)
        .patch_async("/api/tags/:name", crate::api::tags::handle_rename_org_tag)
        // Public settings route
        .get_async(
            "/api/settings",
            crate::api::settings::public::handle_get_public_settings,
        )
        .post_async(
            "/api/settings/api-keys",
            crate::api::keys::handle_create_api_key,
        )
        .get_async(
            "/api/settings/api-keys",
            crate::api::keys::handle_list_api_keys,
        )
        .delete_async(
            "/api/settings/api-keys/:id",
            crate::api::keys::handle_revoke_api_key,
        )
        // Org management routes
        .get_async("/api/orgs", crate::api::orgs::handle_list_user_orgs)
        .post_async("/api/orgs", crate::api::orgs::handle_create_org)
        .get_async("/api/orgs/:id", crate::api::orgs::handle_get_org)
        .patch_async("/api/orgs/:id", crate::api::orgs::handle_update_org)
        .get_async(
            "/api/orgs/:id/settings",
            crate::api::orgs::handle_get_org_settings,
        )
        .patch_async(
            "/api/orgs/:id/settings",
            crate::api::orgs::handle_update_org_settings,
        )
        .delete_async("/api/orgs/:id", crate::api::orgs::handle_delete_org)
        .delete_async(
            "/api/orgs/:id/members/:user_id",
            crate::api::orgs::handle_remove_member,
        )
        .post_async(
            "/api/orgs/:id/invitations",
            crate::api::orgs::handle_create_invitation,
        )
        .delete_async(
            "/api/orgs/:id/invitations/:invitation_id",
            crate::api::orgs::handle_revoke_invitation,
        )
        .post_async(
            "/api/orgs/:id/invitations/:invitation_id/resend",
            crate::api::orgs::handle_resend_invitation,
        )
        .post_async("/api/auth/switch-org", crate::api::orgs::handle_switch_org)
        .post_async(
            "/api/orgs/:id/logo",
            crate::api::orgs::handle_upload_org_logo,
        )
        .get_async("/api/orgs/:id/logo", crate::api::orgs::handle_get_org_logo)
        .delete_async(
            "/api/orgs/:id/logo",
            crate::api::orgs::handle_delete_org_logo,
        )
        // Invite routes (GET is public, POST requires auth)
        .get_async(
            "/api/invite/:token",
            crate::api::orgs::handle_get_invite_info,
        )
        .post_async(
            "/api/invite/:token/accept",
            crate::api::orgs::handle_accept_invite,
        )
        // Billing routes
        .get_async(
            "/api/billing/status",
            crate::api::billing::handle_get_status,
        )
        .get_async(
            "/api/billing/pricing",
            crate::api::billing::pricing::handle_billing_pricing,
        )
        .post_async(
            "/api/billing/checkout",
            crate::api::billing::handle_create_checkout,
        )
        .post_async("/api/billing/webhook", crate::api::billing::handle_webhook)
        .post_async("/api/billing/portal", crate::api::billing::handle_portal)
        .post_async(
            "/api/admin/cron/trigger-downgrade",
            crate::api::billing::handle_cron_trigger_downgrade,
        )
        .post_async(
            "/api/admin/billing-accounts/:id/reset",
            crate::api::billing::handle_admin_reset_billing_account,
        )
        // Org analytics route
        .get_async(
            "/api/analytics/org",
            crate::api::analytics::org::handle_get_org_analytics,
        )
        // Title fetch route (public, can be called by anyone)
        .post_async("/api/fetch-title", crate::api::title_fetch::fetch_title)
        // Root redirect: redirect to frontend (e.g., rush.mn/ → rushomon.cc/)
        .get_async("/", |_req, ctx| async move {
            let url = Url::parse(&crate::utils::get_frontend_url(&ctx.env))?;
            Response::redirect_with_status(url, 301)
        })
        .run(req, env)
        .await
}
