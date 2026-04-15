use crate::kv;
use crate::middleware::{RateLimitConfig, RateLimiter};
use crate::models::{AnalyticsEvent, link::LinkStatus};
use crate::repositories::LinkRepository;
use crate::utils::now_timestamp;
use std::future::Future;
use std::pin::Pin;
use worker::d1::D1Database;
use worker::*;

/// Get the frontend URL from environment, with localhost fallback for local dev
pub fn get_frontend_url(env: &Env) -> String {
    env.var("FRONTEND_URL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
}

/// Extract client IP from Cloudflare headers
fn get_client_ip(req: &Request) -> String {
    // Try CF-Connecting-IP first (most reliable with Cloudflare)
    if let Ok(Some(ip)) = req.headers().get("CF-Connecting-IP") {
        return ip;
    }

    // Fallback to X-Forwarded-For
    if let Ok(Some(forwarded)) = req.headers().get("X-Forwarded-For")
        && let Some(ip) = forwarded.split(',').next()
    {
        // Take first IP in the list
        return ip.trim().to_string();
    }

    // Fallback to X-Real-IP
    if let Ok(Some(ip)) = req.headers().get("X-Real-IP") {
        return ip;
    }

    // Last resort: use a placeholder (should never happen with Cloudflare)
    "unknown".to_string()
}

/// Hash an IP address for logging to avoid storing raw IPs
fn hash_ip(ip: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    ip.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Result of a redirect operation, containing the response and optional deferred analytics work.
pub struct RedirectResult {
    pub response: Response,
    /// Optional future for analytics logging, to be executed via `ctx.wait_until()`.
    pub analytics_future: Option<Pin<Box<dyn Future<Output = ()> + 'static>>>,
}

/// Sync a link's KV mapping from its D1 state.
pub async fn sync_link_mapping_from_link(
    db: &D1Database,
    kv_store: &worker::kv::KvStore,
    link: &crate::models::Link,
) -> Result<()> {
    LinkRepository::new()
        .sync_kv_from_link(db, kv_store, link)
        .await
}

/// Handle public short code redirects: GET /{short_code}
pub async fn handle_redirect(
    req: Request,
    ctx: RouteContext<()>,
    short_code: String,
) -> Result<RedirectResult> {
    let kv = ctx.kv("URL_MAPPINGS")?;

    let client_ip = get_client_ip(&req);
    let rate_limit_key = RateLimiter::ip_key("redirect", &client_ip);
    let rate_limit_config = RateLimitConfig::redirect();

    let kv_rate_limiting_enabled = ctx
        .env
        .var("ENABLE_KV_RATE_LIMITING")
        .map(|v| v.to_string() == "true")
        .unwrap_or(false);

    if let Err(err) = RateLimiter::check(
        &kv,
        &rate_limit_key,
        &rate_limit_config,
        kv_rate_limiting_enabled,
    )
    .await
    {
        let ip_hash = hash_ip(&client_ip);
        console_log!(
            "{}",
            serde_json::json!({
                "event": "rate_limit_hit",
                "endpoint": "redirect",
                "limit_type": "ip_per_code",
                "ip_hash": ip_hash,
                "short_code": short_code,
                "level": "warn"
            })
        );
        let mut response = Response::error(err.to_error_response(), 429)?;
        if let Some(retry_after) = err.retry_after() {
            response
                .headers_mut()
                .set("Retry-After", &retry_after.to_string())?;
        }
        return Ok(RedirectResult {
            response,
            analytics_future: None,
        });
    }

    let mapping = kv::get_link_mapping(&kv, &short_code).await?;

    let Some(mapping) = mapping else {
        let url = Url::parse(&format!("{}/404", get_frontend_url(&ctx.env)))?;
        return Ok(RedirectResult {
            response: Response::redirect_with_status(url, 302)?,
            analytics_future: None,
        });
    };

    if !matches!(mapping.status, LinkStatus::Active) {
        let url = Url::parse(&format!("{}/404", get_frontend_url(&ctx.env)))?;
        return Ok(RedirectResult {
            response: Response::redirect_with_status(url, 302)?,
            analytics_future: None,
        });
    }

    if let Some(expires_at) = mapping.expires_at {
        let now = now_timestamp();
        if now > expires_at {
            let url = Url::parse(&format!("{}/404", get_frontend_url(&ctx.env)))?;
            return Ok(RedirectResult {
                response: Response::redirect_with_status(url, 302)?,
                analytics_future: None,
            });
        }
    }

    let mut destination_url = Url::parse(&mapping.destination_url)?;

    if let Some(ref utm) = mapping.utm_params {
        let pairs: Vec<(&str, &str)> = [
            ("utm_source", utm.utm_source.as_deref()),
            ("utm_medium", utm.utm_medium.as_deref()),
            ("utm_campaign", utm.utm_campaign.as_deref()),
            ("utm_term", utm.utm_term.as_deref()),
            ("utm_content", utm.utm_content.as_deref()),
            ("utm_ref", utm.utm_ref.as_deref()),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.filter(|s| !s.is_empty()).map(|s| (k, s)))
        .collect();

        if !pairs.is_empty() {
            let mut q = destination_url.query_pairs_mut();
            for (k, v) in pairs {
                q.append_pair(k, v);
            }
        }
    }

    if mapping.forward_query_params
        && let Ok(incoming_url) = req.url()
    {
        let visitor_pairs: Vec<(String, String)> = incoming_url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        if !visitor_pairs.is_empty() {
            let mut q = destination_url.query_pairs_mut();
            for (k, v) in &visitor_pairs {
                q.append_pair(k, v);
            }
        }
    }

    let redirect_status = mapping.redirect_type.parse::<u16>().unwrap_or(301);
    let response = Response::redirect_with_status(destination_url, redirect_status)?;

    let referrer = req.headers().get("Referer").ok().flatten();
    let user_agent = req.headers().get("User-Agent").ok().flatten();
    let country = req.headers().get("CF-IPCountry").ok().flatten();
    let city = req.headers().get("CF-IPCity").ok().flatten();

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let link_id = mapping.link_id.clone();
    let now = now_timestamp();

    let analytics_future: Pin<Box<dyn Future<Output = ()> + 'static>> = Box::pin(async move {
        let repo = LinkRepository::new();
        let link = match repo.get_by_id_no_auth(&db, &link_id).await {
            Ok(Some(link)) => link,
            Ok(None) => {
                console_log!(
                    "{}",
                    serde_json::json!({
                        "event": "analytics_link_not_found",
                        "link_id": link_id,
                        "level": "warn"
                    })
                );
                return;
            }
            Err(_) => {
                return;
            }
        };

        if !matches!(link.status, LinkStatus::Active) {
            return;
        }

        let event = AnalyticsEvent {
            id: None,
            link_id: link_id.clone(),
            org_id: link.org_id,
            timestamp: now,
            referrer,
            user_agent,
            country,
            city,
        };

        if let Err(e) = repo.log_analytics_event(&db, &event).await {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "analytics_event_failed",
                    "link_id": link_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
        }
        if let Err(e) = repo.increment_click_count(&db, &link_id).await {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "click_count_failed",
                    "link_id": link_id,
                    "error": e.to_string(),
                    "level": "error"
                })
            );
        }
    });

    Ok(RedirectResult {
        response,
        analytics_future: Some(analytics_future),
    })
}
