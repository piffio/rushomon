/// Cloudflare for SaaS API client
///
/// Thin async wrapper around the Cloudflare Custom Hostnames API.
/// Used to register customer domains so CF can issue SSL certificates
/// and route traffic through our Worker.
///
/// Required env vars:
///   CF_ZONE_ID   - Cloudflare Zone ID for the rush.mn zone
///   CF_API_TOKEN - CF API token with ssl_and_certificates:edit permission
use serde::{Deserialize, Serialize};
use worker::{Env, Fetch, Headers, Method, Request as WorkerRequest, RequestInit};

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// Response from CF API when creating or fetching a custom hostname
#[derive(Debug, Deserialize)]
pub struct CfCustomHostnameResult {
    pub id: String,
    pub hostname: String,
    pub status: String,
    pub ssl: CfSslResult,
    pub ownership_verification: Option<CfOwnershipVerification>,
    pub ownership_verification_http: Option<CfOwnershipVerificationHttp>,
    #[serde(default)]
    pub custom_origin_server: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CfSslResult {
    pub status: String,
    #[serde(default)]
    pub validation_records: Option<Vec<CfValidationRecord>>,
}

/// ACME validation record for SSL certificates
#[derive(Debug, Deserialize)]
pub struct CfValidationRecord {
    pub txt_name: Option<String>,
    pub txt_value: Option<String>,
    pub txt_status: Option<String>,
}

/// TXT-based domain ownership verification (DNS challenge)
#[derive(Debug, Deserialize)]
pub struct CfOwnershipVerification {
    #[serde(rename = "type")]
    pub verification_type: String,
    pub name: String,
    pub value: String,
}

/// HTTP-based domain ownership verification
#[derive(Debug, Deserialize)]
pub struct CfOwnershipVerificationHttp {
    pub http_url: String,
    pub http_body: String,
}

#[derive(Debug, Deserialize)]
struct CfApiResponse<T> {
    pub success: bool,
    pub result: Option<T>,
    pub errors: Vec<CfApiError>,
}

#[derive(Debug, Deserialize)]
struct CfApiError {
    pub code: u32,
    pub message: String,
}

#[derive(Debug, Serialize)]
struct CreateCustomHostnameBody {
    hostname: String,
    ssl: CreateSslConfig,
}

#[derive(Debug, Serialize)]
struct CreateSslConfig {
    method: String,
    #[serde(rename = "type")]
    ssl_type: String,
    settings: SslSettings,
    bundle_method: String,
    wildcard: bool,
}

#[derive(Debug, Serialize)]
struct SslSettings {
    min_tls_version: String,
}

/// Get CF credentials from environment
fn get_cf_credentials(env: &Env) -> Option<(String, String)> {
    let zone_id = env.var("CF_ZONE_ID").ok()?.to_string();
    let api_token = env.secret("CF_API_TOKEN").ok()?.to_string();
    if zone_id.is_empty() || api_token.is_empty() {
        return None;
    }
    Some((zone_id, api_token))
}

/// Build CF API authorization headers
fn build_headers(api_token: &str) -> worker::Result<Headers> {
    let headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", api_token))?;
    headers.set("Content-Type", "application/json")?;
    Ok(headers)
}

/// Create a custom hostname in CF for SaaS.
/// Returns the CF hostname record on success, or None if CF credentials are not configured.
pub async fn create_custom_hostname(
    env: &Env,
    hostname: &str,
) -> worker::Result<Option<CfCustomHostnameResult>> {
    let Some((zone_id, api_token)) = get_cf_credentials(env) else {
        return Ok(None);
    };

    let url = format!("{}/zones/{}/custom_hostnames", CF_API_BASE, zone_id);

    let body = CreateCustomHostnameBody {
        hostname: hostname.to_string(),
        ssl: CreateSslConfig {
            method: "txt".to_string(),
            ssl_type: "dv".to_string(),
            settings: SslSettings {
                min_tls_version: "1.2".to_string(),
            },
            bundle_method: "ubiquitous".to_string(),
            wildcard: false,
        },
    };

    let body_str = serde_json::to_string(&body)
        .map_err(|e| worker::Error::RustError(format!("Failed to serialize CF request: {}", e)))?;

    let headers = build_headers(&api_token)?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body_str.into()));

    let request = WorkerRequest::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(request).send().await?;

    let text = resp.text().await?;
    let parsed: CfApiResponse<CfCustomHostnameResult> =
        serde_json::from_str(&text).map_err(|e| {
            worker::Error::RustError(format!(
                "Failed to parse CF response: {} — body: {}",
                e, text
            ))
        })?;

    if !parsed.success {
        let errors: Vec<String> = parsed
            .errors
            .iter()
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect();
        return Err(worker::Error::RustError(format!(
            "CF API error: {}",
            errors.join(", ")
        )));
    }

    Ok(parsed.result)
}

/// Fetch the current status of a custom hostname from CF.
/// Returns None if CF credentials are not configured.
pub async fn get_custom_hostname(
    env: &Env,
    cf_hostname_id: &str,
) -> worker::Result<Option<CfCustomHostnameResult>> {
    let Some((zone_id, api_token)) = get_cf_credentials(env) else {
        return Ok(None);
    };

    let url = format!(
        "{}/zones/{}/custom_hostnames/{}",
        CF_API_BASE, zone_id, cf_hostname_id
    );

    let headers = build_headers(&api_token)?;
    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);

    let request = WorkerRequest::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(request).send().await?;

    let text = resp.text().await?;
    let parsed: CfApiResponse<CfCustomHostnameResult> =
        serde_json::from_str(&text).map_err(|e| {
            worker::Error::RustError(format!(
                "Failed to parse CF response: {} — body: {}",
                e, text
            ))
        })?;

    if !parsed.success {
        let errors: Vec<String> = parsed
            .errors
            .iter()
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect();
        return Err(worker::Error::RustError(format!(
            "CF API error: {}",
            errors.join(", ")
        )));
    }

    Ok(parsed.result)
}

/// Delete a custom hostname from CF for SaaS.
/// Returns Ok(()) whether or not CF credentials are configured (no-op if unconfigured).
pub async fn delete_custom_hostname(env: &Env, cf_hostname_id: &str) -> worker::Result<()> {
    let Some((zone_id, api_token)) = get_cf_credentials(env) else {
        return Ok(());
    };

    let url = format!(
        "{}/zones/{}/custom_hostnames/{}",
        CF_API_BASE, zone_id, cf_hostname_id
    );

    let headers = build_headers(&api_token)?;
    let mut init = RequestInit::new();
    init.with_method(Method::Delete).with_headers(headers);

    let request = WorkerRequest::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(request).send().await?;

    let status = resp.status_code();
    if status != 200 && status != 204 {
        let text = resp.text().await.unwrap_or_default();
        return Err(worker::Error::RustError(format!(
            "CF delete hostname failed ({}): {}",
            status, text
        )));
    }

    Ok(())
}

/// Check if CF for SaaS is configured (both env vars present)
pub fn is_cf_saas_configured(env: &Env) -> bool {
    get_cf_credentials(env).is_some()
}
