use crate::auth;
use crate::repositories::LinkRepository;
use worker::d1::D1Database;
use worker::*;

/// Escape a single CSV field: wraps in double-quotes if the value contains
/// a comma, double-quote, or newline; doubles any embedded double-quotes.
pub fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[utoipa::path(
    get,
    path = "/api/links/export",
    tag = "Links",
    summary = "Export links as CSV",
    description = "Exports all active links for the authenticated organization as a CSV file. Returns a text/csv response with a Content-Disposition attachment header",
    responses(
        (status = 200, description = "CSV file download"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("Bearer" = []),
        ("session_cookie" = [])
    )
)]
pub async fn handle_export_links(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let user_ctx = match auth::authenticate_request(&req, &ctx).await {
        Ok(ctx) => ctx,
        Err(e) => return Ok(e.into_response()),
    };
    let org_id = &user_ctx.org_id;

    let db = ctx.env.get_binding::<D1Database>("rushomon")?;
    let repo = LinkRepository::new();

    let mut links = repo.get_all_for_export(&db, org_id).await?;

    let link_ids: Vec<String> = links.iter().map(|l| l.id.clone()).collect();
    let tags_map = repo.get_tags_for_links(&db, &link_ids).await?;
    for link in &mut links {
        link.tags = tags_map.get(&link.id).cloned().unwrap_or_default();
    }

    let mut csv = String::from(
        "short_code,destination_url,title,tags,status,click_count,created_at,expires_at,utm_source,utm_medium,utm_campaign,utm_term,utm_content,utm_ref,forward_query_params\n",
    );

    for link in &links {
        let title = link.title.as_deref().unwrap_or("");
        let tags_str = link.tags.join("|");
        let created_at = chrono::DateTime::from_timestamp(link.created_at, 0)
            .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();
        let expires_at = link
            .expires_at
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            .unwrap_or_default();
        let utm_source = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_source.as_deref())
            .unwrap_or("");
        let utm_medium = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_medium.as_deref())
            .unwrap_or("");
        let utm_campaign = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_campaign.as_deref())
            .unwrap_or("");
        let utm_term = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_term.as_deref())
            .unwrap_or("");
        let utm_content = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_content.as_deref())
            .unwrap_or("");
        let utm_ref = link
            .utm_params
            .as_ref()
            .and_then(|u| u.utm_ref.as_deref())
            .unwrap_or("");
        let forward_query = link
            .forward_query_params
            .map(|v| if v { "true" } else { "false" })
            .unwrap_or("");

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&link.short_code),
            csv_escape(&link.destination_url),
            csv_escape(title),
            csv_escape(&tags_str),
            csv_escape(link.status.as_str()),
            link.click_count,
            created_at,
            expires_at,
            csv_escape(utm_source),
            csv_escape(utm_medium),
            csv_escape(utm_campaign),
            csv_escape(utm_term),
            csv_escape(utm_content),
            csv_escape(utm_ref),
            forward_query,
        ));
    }

    let date_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let filename = format!("rushomon-links-{}.csv", date_str);
    let mut response = Response::ok(csv)?;
    response
        .headers_mut()
        .set("Content-Type", "text/csv; charset=utf-8")?;
    response.headers_mut().set(
        "Content-Disposition",
        &format!("attachment; filename=\"{}\"", filename),
    )?;
    Ok(response)
}
