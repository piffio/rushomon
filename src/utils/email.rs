use worker::{Env, Fetch, Method, Request, RequestInit, Result};

// ── Monthly stats email data structures ─────────────────────────────────────

/// A single link's performance summary for the monthly stats email.
#[derive(Debug, Clone)]
pub struct TopLinkSummary {
    pub short_code: String,
    pub title: Option<String>,
    pub clicks: i64,
}

/// Per-org analytics summary included in the monthly stats email.
#[derive(Debug, Clone)]
pub struct OrgMonthlySummary {
    /// Display name of the organization.
    pub org_name: String,
    /// Total active links in this org (used to distinguish "no links" vs "zero clicks").
    pub total_links: i64,
    /// Total clicks on this org's links in the previous calendar month.
    pub total_clicks: i64,
    /// Total clicks in the month before the previous one (for % comparison).
    /// Zero means no comparison data available.
    pub prev_month_clicks: i64,
    /// Top links by click count (up to 5). Empty when total_clicks == 0.
    pub top_links: Vec<TopLinkSummary>,
}

/// Send an organization invitation email via Mailgun
pub async fn send_org_invitation(
    env: &Env,
    to_email: &str,
    inviter_name: &str,
    org_name: &str,
    invite_url: &str,
) -> Result<()> {
    let api_key = env
        .var("MAILGUN_API_KEY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let base_url: String = env
        .var("MAILGUN_BASE_URL")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let domain = env
        .var("MAILGUN_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let from: String = env
        .var("MAILGUN_FROM")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| format!("invites@{}", domain));

    if api_key.is_empty() || domain.is_empty() {
        return Err(worker::Error::RustError(
            "Mailgun not configured: MAILGUN_API_KEY and MAILGUN_DOMAIN are required".to_string(),
        ));
    }

    let subject = format!(
        "{} invited you to join {} on Rushomon",
        inviter_name, org_name
    );

    let html_body = format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; color: #1f2937;">
  <h2 style="color: #ea580c;">You've been invited to join {org_name}</h2>
  <p><strong>{inviter_name}</strong> has invited you to join their organization <strong>{org_name}</strong> on Rushomon.</p>
  <p>Click the button below to accept the invitation. This invite expires in 7 days.</p>
  <p style="margin: 32px 0;">
    <a href="{invite_url}"
       style="background: linear-gradient(to right, #f97316, #ea580c); color: white; padding: 12px 24px;
              text-decoration: none; border-radius: 8px; font-weight: bold; display: inline-block;">
      Accept Invitation →
    </a>
  </p>
  <p style="color: #6b7280; font-size: 14px;">
    Or copy this link into your browser:<br>
    <a href="{invite_url}" style="color: #ea580c;">{invite_url}</a>
  </p>
  <hr style="border: none; border-top: 1px solid #e5e7eb; margin: 32px 0;">
  <p style="color: #9ca3af; font-size: 12px;">
    If you did not expect this invitation, you can safely ignore this email.
  </p>
</body>
</html>"#,
        org_name = org_name,
        inviter_name = inviter_name,
        invite_url = invite_url,
    );

    let text_body = format!(
        "{} has invited you to join {} on Rushomon.\n\nAccept the invitation here: {}\n\nThis invite expires in 7 days.\n\nIf you did not expect this invitation, you can safely ignore this email.",
        inviter_name, org_name, invite_url
    );

    send_via_mailgun(
        env, &api_key, &base_url, &domain, &from, to_email, &subject, &html_body, &text_body,
    )
    .await
}

/// Send the monthly statistics summary email to a single user via Mailgun.
///
/// The email is always sent regardless of activity level:
/// - If an org has no links yet, a "create your first link" nudge is shown.
/// - If an org has links but zero clicks, stats are shown with an encouragement note.
/// - If an org has clicks, full stats and top-links table are shown.
///
/// `month_label` should be a human-readable string like "May 2026".
#[allow(clippy::too_many_arguments)]
pub async fn send_monthly_stats_email(
    env: &Env,
    to_email: &str,
    user_name: Option<&str>,
    month_label: &str,
    orgs_data: &[OrgMonthlySummary],
    frontend_url: &str,
) -> Result<()> {
    let api_key = env
        .var("MAILGUN_API_KEY")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let base_url = env
        .var("MAILGUN_BASE_URL")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let domain = env
        .var("MAILGUN_DOMAIN")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let from = env
        .var("MAILGUN_FROM")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| format!("noreply@{domain}"));

    if api_key.is_empty() || domain.is_empty() {
        return Err(worker::Error::RustError(
            "Mailgun not configured: MAILGUN_API_KEY and MAILGUN_DOMAIN are required".to_string(),
        ));
    }

    let greeting = user_name.unwrap_or("there");
    let subject = format!("Your {month_label} Rushomon recap");

    // ── Build per-org HTML sections ──────────────────────────────────────────
    let mut org_sections_html = String::new();
    let mut org_sections_text = String::new();

    for org in orgs_data {
        if org.total_links == 0 {
            // Org has no links at all — nudge user to create one
            org_sections_html.push_str(&format!(
                r#"
  <div style="border:1px solid #e5e7eb; border-radius:12px; padding:24px; margin-bottom:20px;">
    <h3 style="margin:0 0 8px 0; color:#111827; font-size:15px; font-weight:600;">{org_name}</h3>
    <p style="margin:0 0 20px 0; color:#6b7280; font-size:14px; line-height:1.5;">
      You haven't created any short links in this organization yet.
    </p>
    <a href="{frontend_url}/dashboard"
       style="background:#f97316; color:#ffffff; padding:10px 20px;
              text-decoration:none; border-radius:8px; font-weight:600;
              display:inline-block; font-size:14px;">
      Create your first link →
    </a>
  </div>"#,
                org_name = escape_html(&org.org_name),
                frontend_url = frontend_url,
            ));
            org_sections_text.push_str(&format!(
                "\n--- {} ---\nYou haven't created any short links yet. Get started: {}/dashboard\n",
                org.org_name, frontend_url,
            ));
        } else if org.total_clicks == 0 {
            // Has links but zero clicks
            let links_word = if org.total_links == 1 {
                "link"
            } else {
                "links"
            };
            org_sections_html.push_str(&format!(
                r#"
  <div style="border:1px solid #e5e7eb; border-radius:12px; padding:24px; margin-bottom:20px;">
    <h3 style="margin:0 0 2px 0; color:#111827; font-size:15px; font-weight:600;">{org_name}</h3>
    <p style="margin:0 0 16px 0; color:#9ca3af; font-size:13px;">{total_links} active {links_word}</p>
    <div style="border:1px solid #e5e7eb; border-radius:8px; padding:16px 20px; margin-bottom:16px;">
      <p style="margin:0; font-size:28px; font-weight:700; color:#111827; line-height:1.2;">0 clicks</p>
      <p style="margin:6px 0 0 0; color:#9ca3af; font-size:13px;">in {month_label}</p>
    </div>
    <p style="color:#6b7280; font-size:14px; margin:0 0 16px 0; line-height:1.5;">
      No clicks recorded last month — share your links to start seeing traffic!
    </p>
    <a href="{frontend_url}/dashboard/analytics"
       style="color:#f97316; font-size:14px; text-decoration:none; font-weight:500;">
      View analytics →
    </a>
  </div>"#,
                org_name = escape_html(&org.org_name),
                total_links = org.total_links,
                links_word = links_word,
                month_label = month_label,
                frontend_url = frontend_url,
            ));
            org_sections_text.push_str(&format!(
                "\n--- {} ---\n0 clicks in {} ({} active {}).\nShare your links to start seeing traffic!\nView analytics: {}/dashboard/analytics\n",
                org.org_name, month_label, org.total_links, links_word, frontend_url,
            ));
        } else {
            // Normal stats with clicks
            let trend_html = build_trend_html(org.total_clicks, org.prev_month_clicks);
            let trend_text = build_trend_text(org.total_clicks, org.prev_month_clicks);
            let top_links_html = build_top_links_html(&org.top_links);
            let top_links_text = build_top_links_text(&org.top_links);
            let links_word = if org.total_links == 1 {
                "link"
            } else {
                "links"
            };

            org_sections_html.push_str(&format!(
                r#"
  <div style="border:1px solid #e5e7eb; border-radius:12px; padding:24px; margin-bottom:20px;">
    <h3 style="margin:0 0 2px 0; color:#111827; font-size:15px; font-weight:600;">{org_name}</h3>
    <p style="margin:0 0 16px 0; color:#9ca3af; font-size:13px;">{total_links} active {links_word}</p>
    <div style="border:1px solid #e5e7eb; border-radius:8px; padding:16px 20px; margin-bottom:16px;">
      <table cellpadding="0" cellspacing="0" border="0">
        <tr>
          <td style="vertical-align:baseline; padding-right:12px;">
            <p style="margin:0; font-size:28px; font-weight:700; color:#111827; line-height:1.2;">{total_clicks} clicks</p>
            <p style="margin:6px 0 0 0; color:#9ca3af; font-size:13px;">in {month_label}</p>
          </td>
          <td style="vertical-align:middle;">{trend_html}</td>
        </tr>
      </table>
    </div>
    {top_links_html}
    <a href="{frontend_url}/dashboard/analytics"
       style="color:#f97316; font-size:14px; text-decoration:none; font-weight:500; display:inline-block; margin-top:12px;">
      View full analytics →
    </a>
  </div>"#,
                org_name = escape_html(&org.org_name),
                total_links = org.total_links,
                links_word = links_word,
                total_clicks = org.total_clicks,
                month_label = month_label,
                trend_html = trend_html,
                top_links_html = top_links_html,
                frontend_url = frontend_url,
            ));
            org_sections_text.push_str(&format!(
                "\n--- {} ---\n{} clicks in {} ({} active {}).{}\n{}\nView analytics: {}/dashboard/analytics\n",
                org.org_name,
                org.total_clicks,
                month_label,
                org.total_links,
                links_word,
                trend_text,
                top_links_text,
                frontend_url,
            ));
        }
    }

    // ── Assemble full email ──────────────────────────────────────────────────
    let settings_url = format!("{frontend_url}/settings");

    let html_body = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Your {month_label} Rushomon recap</title>
</head>
<body style="margin:0; padding:0; background:#ffffff; font-family:-apple-system,BlinkMacSystemFont,Roboto,Helvetica,Arial,sans-serif; color:#111827;">
  <div style="max-width:600px; margin:0 auto; padding:40px 24px 48px;">

    <!-- Logo -->
    <div style="margin-bottom:32px;">
      <table cellpadding="0" cellspacing="0" border="0">
        <tr>
          <td style="vertical-align:middle;">
            <div style="width:32px; height:32px;
                        background:linear-gradient(135deg, #f97316 0%, #ea580c 100%);
                        border-radius:8px; text-align:center; line-height:32px;
                        font-size:14px; font-weight:700; color:#ffffff;">R</div>
          </td>
          <td style="vertical-align:middle; padding-left:8px;">
            <span style="font-size:24px; font-weight:700; color:#111827; line-height:1;">Rushomon</span>
          </td>
        </tr>
      </table>
    </div>

    <!-- Heading -->
    <h1 style="font-size:24px; font-weight:700; color:#111827; margin:0 0 6px 0; line-height:1.3;">
      Your {month_label} recap, {greeting}!
    </h1>
    <p style="color:#6b7280; font-size:15px; margin:0 0 32px 0; line-height:1.5;">
      Here's how your short links performed last month.
    </p>

    {org_sections}

    <!-- Footer -->
    <div style="border-top:1px solid #e5e7eb; margin-top:40px; padding-top:24px;">
      <p style="color:#9ca3af; font-size:12px; margin:0; line-height:1.6;">
        You're receiving this because you have monthly stats emails enabled on
        <a href="{frontend_url}" style="color:#f97316; text-decoration:none;">Rushomon</a>.
        To stop receiving these emails, visit your
        <a href="{settings_url}" style="color:#f97316; text-decoration:none;">account settings</a>.
      </p>
    </div>

  </div>
</body>
</html>"#,
        month_label = month_label,
        greeting = escape_html(greeting),
        org_sections = org_sections_html,
        frontend_url = frontend_url,
        settings_url = settings_url,
    );

    let text_body = format!(
        "Your {month_label} Rushomon recap, {greeting}!\n\nHere's how your short links performed last month.\n{org_sections}\n\n---\nTo stop receiving these emails, visit {settings_url}",
        month_label = month_label,
        greeting = greeting,
        org_sections = org_sections_text,
        settings_url = settings_url,
    );

    send_via_mailgun(
        env, &api_key, &base_url, &domain, &from, to_email, &subject, &html_body, &text_body,
    )
    .await
}

// ── Private helpers ──────────────────────────────────────────────────────────

/// Escape the minimum set of HTML special characters.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Build the trend badge HTML (e.g. "↑12%" in green, "↓5%" in red, "New" in blue).
fn build_trend_html(current: i64, previous: i64) -> String {
    if previous == 0 {
        return r#"<span style="background:#fff7ed; color:#f97316; font-size:12px; font-weight:600;
                               padding:3px 9px; border-radius:9999px; white-space:nowrap;">New</span>"#.to_string();
    }
    let pct = ((current - previous) as f64 / previous as f64 * 100.0).round() as i64;
    if pct >= 0 {
        format!(
            r#"<span style="background:#dcfce7; color:#15803d; font-size:12px; font-weight:600;
                            padding:3px 9px; border-radius:9999px; white-space:nowrap;">↑{pct}%</span>"#
        )
    } else {
        format!(
            r#"<span style="background:#fee2e2; color:#b91c1c; font-size:12px; font-weight:600;
                            padding:3px 9px; border-radius:9999px; white-space:nowrap;">↓{}%</span>"#,
            pct.unsigned_abs()
        )
    }
}

/// Build a plain-text trend string.
fn build_trend_text(current: i64, previous: i64) -> String {
    if previous == 0 {
        return " (New)".to_string();
    }
    let pct = ((current - previous) as f64 / previous as f64 * 100.0).round() as i64;
    if pct >= 0 {
        format!(" (↑{pct}% vs last month)")
    } else {
        format!(" (↓{}% vs last month)", pct.unsigned_abs())
    }
}

/// Build the top-links table HTML.
fn build_top_links_html(links: &[TopLinkSummary]) -> String {
    if links.is_empty() {
        return String::new();
    }
    let mut rows = String::new();
    for link in links {
        let label = link
            .title
            .as_deref()
            .filter(|t| !t.is_empty())
            .unwrap_or(&link.short_code);
        rows.push_str(&format!(
            r#"<tr>
          <td style="padding:8px 0; color:#374151; font-size:13px; border-bottom:1px solid #f3f4f6;">{label}</td>
          <td style="padding:8px 0; color:#6b7280; font-size:13px; border-bottom:1px solid #f3f4f6; text-align:right; white-space:nowrap;">/{short_code}</td>
          <td style="padding:8px 0; font-size:13px; font-weight:600; color:#111827; border-bottom:1px solid #f3f4f6; text-align:right; padding-left:16px;">{clicks}</td>
        </tr>"#,
            label = escape_html(label),
            short_code = escape_html(&link.short_code),
            clicks = link.clicks,
        ));
    }
    format!(
        r#"<table style="width:100%; border-collapse:collapse; margin-bottom:8px;">
      <thead>
        <tr>
          <th style="text-align:left; color:#9ca3af; font-size:11px; font-weight:600; padding-bottom:6px; text-transform:uppercase; letter-spacing:.05em;">Link</th>
          <th style="text-align:right; color:#9ca3af; font-size:11px; font-weight:600; padding-bottom:6px; text-transform:uppercase; letter-spacing:.05em;"></th>
          <th style="text-align:right; color:#9ca3af; font-size:11px; font-weight:600; padding-bottom:6px; text-transform:uppercase; letter-spacing:.05em; padding-left:16px;">Clicks</th>
        </tr>
      </thead>
      <tbody>{rows}</tbody>
    </table>"#
    )
}

/// Build the top-links plain-text block.
fn build_top_links_text(links: &[TopLinkSummary]) -> String {
    if links.is_empty() {
        return String::new();
    }
    let mut out = "\nTop links:\n".to_string();
    for link in links {
        let label = link
            .title
            .as_deref()
            .filter(|t| !t.is_empty())
            .unwrap_or(&link.short_code);
        out.push_str(&format!(
            "  {} (/{}) — {} clicks\n",
            label, link.short_code, link.clicks
        ));
    }
    out
}

/// Send a message via the Mailgun REST API.
#[allow(clippy::too_many_arguments)]
async fn send_via_mailgun(
    _env: &Env,
    api_key: &str,
    base_url: &str,
    domain: &str,
    from: &str,
    to: &str,
    subject: &str,
    html: &str,
    text: &str,
) -> Result<()> {
    let form_body = format!(
        "from={}&to={}&subject={}&html={}&text={}",
        urlencoding::encode(from),
        urlencoding::encode(to),
        urlencoding::encode(subject),
        urlencoding::encode(html),
        urlencoding::encode(text),
    );

    let mailgun_url = format!("{}/v3/{}/messages", base_url, domain);
    let credentials = format!("api:{}", api_key);
    let auth_header = format!("Basic {}", base64_encode(credentials.as_bytes()));

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_body(Some(wasm_bindgen::JsValue::from_str(&form_body)));

    let mut request = Request::new_with_init(&mailgun_url, &init)?;
    request.headers_mut()?.set("Authorization", &auth_header)?;
    request
        .headers_mut()?
        .set("Content-Type", "application/x-www-form-urlencoded")?;

    let response = Fetch::Request(request).send().await?;

    if response.status_code() >= 400 {
        return Err(worker::Error::RustError(format!(
            "Mailgun API error: status {}",
            response.status_code()
        )));
    }

    Ok(())
}

fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as u32;
        let b1 = if i + 1 < input.len() {
            input[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            input[i + 2] as u32
        } else {
            0
        };

        result.push(ALPHABET[((b0 >> 2) & 0x3f) as usize] as char);
        result.push(ALPHABET[(((b0 << 4) | (b1 >> 4)) & 0x3f) as usize] as char);
        if i + 1 < input.len() {
            result.push(ALPHABET[(((b1 << 2) | (b2 >> 6)) & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
        if i + 2 < input.len() {
            result.push(ALPHABET[(b2 & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
        i += 3;
    }
    result
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }
}
