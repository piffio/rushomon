/// OAuth Service
///
/// Business logic for OAuth flow initiation and callback handling.
/// Coordinates between OAuth providers, user management, and session creation.
use crate::auth::oauth::{OAuthTokens, handle_oauth_callback, initiate_oauth};
use crate::models::{Organization, User};
use worker::{Request, Response, Result, RouteContext};

pub struct OAuthService;

impl OAuthService {
    pub fn new() -> Self {
        Self
    }

    /// Initiate GitHub OAuth login flow
    pub async fn initiate_github_login(
        &self,
        req: &Request,
        ctx: &RouteContext<()>,
    ) -> Result<Response> {
        let kv = ctx.kv("URL_MAPPINGS")?;
        let provider = &crate::auth::providers::GITHUB;

        if !provider.is_enabled(&ctx.env) {
            return Response::error("GitHub OAuth is not configured", 404);
        }

        let redirect_uri = self.oauth_redirect_uri(ctx)?;
        initiate_oauth(req, &kv, provider, &redirect_uri, &ctx.env).await
    }

    /// Initiate Google OAuth login flow
    pub async fn initiate_google_login(
        &self,
        req: &Request,
        ctx: &RouteContext<()>,
    ) -> Result<Response> {
        let kv = ctx.kv("URL_MAPPINGS")?;
        let provider = &crate::auth::providers::GOOGLE;

        if !provider.is_enabled(&ctx.env) {
            return Response::error("Google OAuth is not configured", 404);
        }

        let redirect_uri = self.oauth_redirect_uri(ctx)?;
        initiate_oauth(req, &kv, provider, &redirect_uri, &ctx.env).await
    }

    /// Handle OAuth callback (both GitHub and Google)
    pub async fn handle_callback(
        &self,
        req: &Request,
        code: &str,
        state: &str,
        ctx: &RouteContext<()>,
    ) -> Result<OAuthCallbackResult> {
        let kv = ctx.kv("URL_MAPPINGS")?;
        let db = ctx.env.get_binding::<worker::d1::D1Database>("rushomon")?;

        // Handle OAuth callback - returns both access and refresh tokens, plus optional redirect
        let (user, org, tokens, redirect) =
            handle_oauth_callback(req, code.to_string(), state.to_string(), &kv, &db, &ctx.env)
                .await?;

        Ok(OAuthCallbackResult {
            user,
            org,
            tokens,
            redirect,
        })
    }

    /// Generate OAuth redirect URI for the current environment
    fn oauth_redirect_uri(&self, ctx: &RouteContext<()>) -> Result<String> {
        let domain = ctx.env.var("DOMAIN")?.to_string();
        let scheme = if domain.starts_with("localhost") {
            "http"
        } else {
            "https"
        };
        Ok(format!("{}://{}/api/auth/callback", scheme, domain))
    }
}

impl Default for OAuthService {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a successful OAuth callback
#[allow(dead_code)]
pub struct OAuthCallbackResult {
    pub user: User,
    pub org: Organization,
    pub tokens: OAuthTokens,
    pub redirect: Option<String>,
}
