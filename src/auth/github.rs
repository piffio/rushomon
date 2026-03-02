use crate::auth::providers::NormalizedUser;
use serde::Deserialize;
use worker::{Error, Fetch, Headers, Method, Request, RequestInit, Result, console_log};

/// Check if an email is a GitHub noreply address
pub fn is_noreply_email(email: &str) -> bool {
    email.ends_with("@users.noreply.github.com")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_noreply_email_detects_github_noreply() {
        assert!(is_noreply_email("user@users.noreply.github.com"));
        assert!(is_noreply_email("piffio@users.noreply.github.com"));
        assert!(is_noreply_email("test123@users.noreply.github.com"));

        assert!(!is_noreply_email("user@gmail.com"));
        assert!(!is_noreply_email("user@github.com"));
        assert!(!is_noreply_email("user@users.noreply.github.com.org"));
        assert!(!is_noreply_email("user@noreply.github.com"));
    }

    #[test]
    fn test_is_noreply_email_edge_cases() {
        // Empty string
        assert!(!is_noreply_email(""));

        // Partial matches
        assert!(!is_noreply_email("prefix-users.noreply.github.com"));
        assert!(!is_noreply_email("users.noreply.github.com-suffix"));

        // Case sensitivity - ends_with is case-sensitive
        assert!(!is_noreply_email("USER@USERS.NOREPLY.GITHUB.COM"));
        assert!(is_noreply_email("user@users.noreply.github.com"));

        // Exact matches with different usernames
        assert!(is_noreply_email("a@users.noreply.github.com"));
        assert!(is_noreply_email(
            "very-long-username-123@users.noreply.github.com"
        ));
    }
}

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubEmail {
    pub email: String,
    pub verified: bool,
    pub primary: bool,
    pub visibility: Option<String>,
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

/// Fetch user emails from GitHub API
pub async fn fetch_user_emails(access_token: &str) -> Result<Vec<GitHubEmail>> {
    let headers = Headers::new();
    headers.set("Authorization", &format!("token {}", access_token))?;
    headers.set("User-Agent", "Rushomon")?;
    headers.set("Accept", "application/vnd.github.v3+json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);

    let request = Request::new_with_init("https://api.github.com/user/emails", &init)?;
    let mut response = Fetch::Request(request).send().await?;

    if response.status_code() != 200 {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::RustError(format!(
            "GitHub emails fetch failed: {}",
            error_text
        )));
    }

    let emails: Vec<GitHubEmail> = response.json().await?;
    Ok(emails)
}

/// Find the primary verified email from a list of emails
fn find_primary_email(emails: &[GitHubEmail]) -> Option<String> {
    // First try to find a verified primary email
    emails
        .iter()
        .find(|email| email.primary && email.verified)
        .map(|email| email.email.clone())
        .or_else(|| {
            // If no primary email, fall back to any verified email
            emails
                .iter()
                .find(|email| email.verified)
                .map(|email| email.email.clone())
        })
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

    // If email is not provided in the main user endpoint, fetch from emails endpoint
    let email = if github_user.email.is_none() {
        match fetch_user_emails(access_token).await {
            Ok(emails) => find_primary_email(&emails),
            Err(_) => {
                // If fetching emails fails, log the error and fall back to noreply
                console_log!(
                    "Failed to fetch user emails from GitHub, falling back to noreply email"
                );
                None
            }
        }
    } else {
        github_user.email.clone()
    };

    Ok(NormalizedUser {
        provider: "github".to_string(),
        provider_id: github_user.id.to_string(),
        email: email.unwrap_or_else(|| format!("{}@users.noreply.github.com", github_user.login)),
        name: Some(
            github_user
                .name
                .unwrap_or_else(|| github_user.login.clone()),
        ),
        avatar_url: github_user.avatar_url,
    })
}
