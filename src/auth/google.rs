use crate::auth::providers::NormalizedUser;
use serde::Deserialize;
use worker::*;

#[derive(Debug, Deserialize)]
pub struct GoogleTokenResponse {
    pub access_token: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUser {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// Exchange authorization code for Google access token
pub async fn exchange_code_for_token(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    token_url: &str,
) -> Result<String> {
    use wasm_bindgen::JsValue;

    let body = format!(
        "client_id={}&client_secret={}&code={}&redirect_uri={}&grant_type=authorization_code",
        urlencoding::encode(client_id),
        urlencoding::encode(client_secret),
        urlencoding::encode(code),
        urlencoding::encode(redirect_uri)
    );

    let headers = Headers::new();
    headers.set("Accept", "application/json")?;
    headers.set("Content-Type", "application/x-www-form-urlencoded")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&body)));

    let request = Request::new_with_init(token_url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::RustError(format!(
            "Google token exchange failed: {}",
            error_text
        )));
    }

    let token_response: GoogleTokenResponse = response.json().await?;
    Ok(token_response.access_token)
}

/// Fetch and normalize user profile from Google userinfo endpoint
pub async fn fetch_user(access_token: &str, user_url: &str) -> Result<NormalizedUser> {
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", access_token))?;
    headers.set("Accept", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);

    let request = Request::new_with_init(user_url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::RustError(format!(
            "Google user fetch failed: {}",
            error_text
        )));
    }

    let google_user: GoogleUser = response.json().await?;

    let email = google_user.email.ok_or_else(|| {
        Error::RustError(
            "Google account has no email address. Ensure the 'email' scope is granted.".to_string(),
        )
    })?;

    Ok(NormalizedUser {
        provider: "google".to_string(),
        provider_id: google_user.sub,
        email,
        name: google_user.name,
        avatar_url: google_user.picture,
    })
}
