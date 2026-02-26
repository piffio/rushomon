use worker::{Env, Fetch, Method, Request, RequestInit, Result};

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

    // Build multipart/form-data body manually (Cloudflare Workers doesn't support FormData in fetch)
    let form_body = format!(
        "from={}&to={}&subject={}&html={}&text={}",
        urlencoding::encode(&from),
        urlencoding::encode(to_email),
        urlencoding::encode(&subject),
        urlencoding::encode(&html_body),
        urlencoding::encode(&text_body),
    );

    let mailgun_url = format!("{}/v3/{}/messages", base_url, domain);

    // Basic auth: "api:{api_key}" base64-encoded
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
