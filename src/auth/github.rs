use crate::auth::providers::NormalizedUser;
use serde::Deserialize;
use worker::{Error, Fetch, Headers, Method, Request, RequestInit, Result};

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubTokenResponse {
    pub access_token: String,
}

/// Exchange authorization code for GitHub access token
pub async fn exchange_code_for_token(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    token_url: &str,
) -> Result<String> {
    use wasm_bindgen::JsValue;

    let body = serde_json::json!({
        "client_id": client_id,
        "client_secret": client_secret,
        "code": code,
        "redirect_uri": redirect_uri,
    });

    let headers = Headers::new();
    headers.set("Accept", "application/json")?;
    headers.set("Content-Type", "application/json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&body.to_string())));

    let request = Request::new_with_init(token_url, &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::RustError(format!(
            "GitHub token exchange failed: {}",
            error_text
        )));
    }

    let token_response: GitHubTokenResponse = response.json().await?;
    Ok(token_response.access_token)
}

/// Fetch and normalize user profile from GitHub API
pub async fn fetch_user(access_token: &str, user_url: &str) -> Result<NormalizedUser> {
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", access_token))?;
    headers.set("User-Agent", "Rushomon")?;
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
            "GitHub user fetch failed: {}",
            error_text
        )));
    }

    let github_user: GitHubUser = response.json().await?;

    Ok(NormalizedUser {
        provider: "github".to_string(),
        provider_id: github_user.id.to_string(),
        email: github_user
            .email
            .unwrap_or_else(|| format!("{}@users.noreply.github.com", github_user.login)),
        name: Some(
            github_user
                .name
                .unwrap_or_else(|| github_user.login.clone()),
        ),
        avatar_url: github_user.avatar_url,
    })
}
