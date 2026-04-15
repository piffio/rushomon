use crate::auth;
use crate::db;
use crate::kv;
use crate::models::{
    Tier,
    link::{Link, LinkStatus},
};
use crate::repositories::tag_repository::validate_and_normalize_tags;
use crate::repositories::{LinkRepository, TagRepository};
use crate::utils::{generate_short_code, now_timestamp, validate_short_code, validate_url};
use chrono::Datelike;
use worker::d1::D1Database;
use worker::*;

#[derive(Debug, serde::Deserialize)]
struct ImportLinkRow {
    destination_url: String,
    short_code: Option<String>,
    title: Option<String>,
    tags: Option<Vec<String>>,
    expires_at: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct ImportRequest {
    links: Vec<ImportLinkRow>,
}

#[derive(Debug, serde::Serialize)]
struct ImportError {
    row: usize,
    destination_url: String,
    reason: String,
}

#[derive(Debug, serde::Serialize)]
struct ImportWarning {
    row: usize,
    destination_url: String,
    reason: String,
}

#[derive(Debug, serde::Serialize)]
struct ImportResponse {
    created: usize,
    skipped: usize,
    failed: usize,
    errors: Vec<ImportError>,
    warnings: Vec<ImportWarning>,
}

#[utoipa::path(
    post,
    path = "/api/links/import",
    tag = "Links",
    summary = "Import links from CSV",
    description = "Bulk-imports links from a CSV payload. Accepts a JSON array of rows parsed from CSV. Each row must have at least a destination_url. Returns counts of created, skipped (duplicate short codes), and failed rows",
    responses(
        (status = 200, description = "Import result with created/skipped/failed counts"),
        (status = 400, description = "Invalid request body"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_import_links(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let user_id = &user_ctx.user_id;
    let org_id = &user_ctx.org_id;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;

    let billing_account = db::get_billing_account_for_org(&db, org_id)
        .await?
        .ok_or_else(|| Error::RustError("No billing account found for organization".to_string()))?;
    let tier = Tier::from_str_value(&billing_account.tier);
    let limits = tier.as_ref().map(|t| t.limits());
    let is_pro_or_above = matches!(
        tier.as_ref(),
        Some(Tier::Pro) | Some(Tier::Business) | Some(Tier::Unlimited)
    );

    let body: ImportRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("Invalid JSON body", 400),
    };

    if body.links.is_empty() {
        return Response::from_json(&ImportResponse {
            created: 0,
            skipped: 0,
            failed: 0,
            errors: vec![],
            warnings: vec![],
        });
    }

    if body.links.len() > 50 {
        return Response::error("Maximum 50 links per import batch", 400);
    }

    let kv = ctx.kv("URL_MAPPINGS")?;
    let now = now_timestamp();
    let year_month = {
        let dt = chrono::Utc::now();
        format!("{}-{:02}", dt.year(), dt.month())
    };

    let mut created: usize = 0;
    let mut skipped: usize = 0;
    let mut failed: usize = 0;
    let mut errors: Vec<ImportError> = Vec::new();
    let mut warnings: Vec<ImportWarning> = Vec::new();

    let repo = LinkRepository::new();

    for (idx, row) in body.links.iter().enumerate() {
        let row_num = idx + 1;

        let destination_url = match validate_url(&row.destination_url) {
            Ok(url) => url,
            Err(e) => {
                failed += 1;
                errors.push(ImportError {
                    row: row_num,
                    destination_url: row.destination_url.clone(),
                    reason: format!("Invalid URL: {}", e),
                });
                continue;
            }
        };

        if db::is_destination_blacklisted(&db, &destination_url).await? {
            failed += 1;
            errors.push(ImportError {
                row: row_num,
                destination_url: destination_url.clone(),
                reason: "Destination URL is blocked".to_string(),
            });
            continue;
        }

        if let Some(ref tier_limits) = limits
            && let Some(max_links) = tier_limits.max_links_per_month
        {
            let can_create = db::increment_monthly_counter_for_billing_account(
                &db,
                &billing_account.id,
                &year_month,
                max_links,
            )
            .await?;
            if !can_create {
                failed += 1;
                errors.push(ImportError {
                    row: row_num,
                    destination_url: destination_url.clone(),
                    reason: "Monthly link limit reached".to_string(),
                });
                continue;
            }
        }

        let short_code: String;
        if is_pro_or_above && let Some(provided_code) = row.short_code.as_ref() {
            if let Err(e) = validate_short_code(provided_code) {
                skipped += 1;
                errors.push(ImportError {
                    row: row_num,
                    destination_url: destination_url.clone(),
                    reason: format!("Invalid short code: {}", e),
                });
                continue;
            }

            let mut resolved: Option<String> = None;
            for attempt in 0u32..=10 {
                let candidate = if attempt == 0 {
                    provided_code.clone()
                } else {
                    format!("{}-{}", provided_code, attempt)
                };
                if !kv::links::short_code_exists(&kv, &candidate).await? {
                    resolved = Some(candidate);
                    break;
                }
            }

            match resolved {
                Some(c) => short_code = c,
                None => {
                    let mut fallback: Option<String> = None;
                    for _ in 0..10u32 {
                        let candidate = generate_short_code();
                        if !kv::links::short_code_exists(&kv, &candidate).await? {
                            fallback = Some(candidate);
                            break;
                        }
                    }
                    match fallback {
                        Some(c) => {
                            warnings.push(ImportWarning {
                                row: row_num,
                                destination_url: destination_url.clone(),
                                reason: format!(
                                    "Short code '{}' conflicted with an existing link; a random code was assigned",
                                    provided_code
                                ),
                            });
                            short_code = c;
                        }
                        None => {
                            failed += 1;
                            errors.push(ImportError {
                                row: row_num,
                                destination_url: destination_url.clone(),
                                reason: "Failed to generate a unique short code after conflict"
                                    .to_string(),
                            });
                            continue;
                        }
                    }
                }
            }
        } else {
            let mut resolved: Option<String> = None;
            for _ in 0..10u32 {
                let candidate = generate_short_code();
                if !kv::links::short_code_exists(&kv, &candidate).await? {
                    resolved = Some(candidate);
                    break;
                }
            }
            match resolved {
                Some(c) => short_code = c,
                None => {
                    failed += 1;
                    errors.push(ImportError {
                        row: row_num,
                        destination_url: destination_url.clone(),
                        reason: "Failed to generate unique short code".to_string(),
                    });
                    continue;
                }
            }
        }

        let mut normalized_tags = if let Some(ref tags) = row.tags {
            validate_and_normalize_tags(tags).unwrap_or_default()
        } else {
            Vec::new()
        };

        if let Some(ref tier_limits) = limits
            && let Some(max_tags) = tier_limits.max_tags
        {
            let current_tag_count = TagRepository::new()
                .count_distinct_tags_for_billing_account(&db, &billing_account.id)
                .await?;

            let mut new_tag_count = 0;
            if !normalized_tags.is_empty() {
                let existing_tags_query = db.prepare(
                    "SELECT DISTINCT tag_name
                     FROM link_tags lt
                     JOIN organizations o ON lt.org_id = o.id
                     WHERE o.billing_account_id = ?1",
                );
                let existing_tags_result = existing_tags_query
                    .bind(&[billing_account.id.clone().into()])?
                    .all()
                    .await?;
                let existing_tags_set: std::collections::HashSet<String> = existing_tags_result
                    .results::<serde_json::Value>()?
                    .iter()
                    .filter_map(|row| row["tag_name"].as_str().map(|s| s.to_string()))
                    .collect();

                new_tag_count = normalized_tags
                    .iter()
                    .filter(|tag| !existing_tags_set.contains(*tag))
                    .count() as i64;
            }

            if current_tag_count + new_tag_count > max_tags {
                skipped += 1;
                warnings.push(ImportWarning {
                    row: row_num,
                    destination_url: destination_url.clone(),
                    reason: format!(
                        "Tags skipped: would exceed tag limit ({} max). Consider upgrading your plan.",
                        max_tags
                    ),
                });
                normalized_tags.clear();
            }
        }

        let title = row.title.as_ref().and_then(|t| {
            let trimmed = t.trim().to_string();
            if trimmed.is_empty() || trimmed.len() > 200 {
                None
            } else {
                Some(trimmed)
            }
        });

        let link_id = uuid::Uuid::new_v4().to_string();
        let link = Link {
            id: link_id.clone(),
            org_id: org_id.to_string(),
            short_code: short_code.clone(),
            destination_url: destination_url.clone(),
            title,
            created_by: user_id.to_string(),
            created_at: now,
            updated_at: None,
            expires_at: row.expires_at,
            status: LinkStatus::Active,
            click_count: 0,
            tags: normalized_tags.clone(),
            utm_params: None,
            forward_query_params: None,
            redirect_type: "301".to_string(),
        };

        repo.create(&db, &link).await?;

        if !normalized_tags.is_empty() {
            repo.set_tags(&db, &link_id, org_id, &normalized_tags)
                .await?;
        }

        let mapping = link.to_mapping(false);
        kv::store_link_mapping(&kv, org_id, &short_code, &mapping).await?;

        created += 1;
    }

    Response::from_json(&ImportResponse {
        created,
        skipped,
        failed,
        errors,
        warnings,
    })
}
