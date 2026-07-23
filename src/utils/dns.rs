//! DNS-over-HTTPS helpers used for organization domain verification.
//!
//! Runs server-side (via Cloudflare's DoH resolver) so the browser never has
//! to make cross-origin DNS requests — keeping the frontend within its CSP.
use serde::Deserialize;
use worker::*;

#[derive(Deserialize, Debug)]
struct DnsResponse {
    #[serde(rename = "Answer")]
    answer: Option<Vec<DnsAnswer>>,
}

#[derive(Deserialize, Debug)]
struct DnsAnswer {
    data: String,
}

/// Query a DNS record type for a domain via Cloudflare DoH, returning the
/// list of answer `data` strings (empty if none / on error).
async fn query_dns(domain: &str, record_type: &str) -> Result<Vec<String>> {
    let encoded_domain = urlencoding::encode(domain);
    let url = format!(
        "https://cloudflare-dns.com/dns-query?name={}&type={}",
        encoded_domain, record_type
    );

    let headers = Headers::new();
    headers.set("Accept", "application/dns-json")?;

    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(headers);

    let req = Request::new_with_init(&url, &init)?;
    let mut response = Fetch::Request(req).send().await?;

    if response.status_code() != 200 {
        console_log!(
            "DNS {} query failed with status: {}",
            record_type,
            response.status_code()
        );
        return Ok(Vec::new());
    }

    let resp: DnsResponse = response.json().await?;
    Ok(resp
        .answer
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.data)
        .collect())
}

/// Verify that a domain has a TXT record containing the expected verification token.
pub async fn verify_dns_txt(domain: &str, expected_token: &str) -> Result<bool> {
    let answers = query_dns(domain, "TXT").await?;
    let expected_content = format!("rushomon-verification={}", expected_token);
    // DNS TXT records are usually wrapped in quotes, but check for both.
    let expected_quoted = format!("\"{}\"", expected_content);

    Ok(answers
        .iter()
        .any(|a| a.contains(&expected_content) || a.contains(&expected_quoted)))
}

/// Best-effort check for whether a domain is served by Cloudflare nameservers.
/// Powers the "Open Cloudflare" convenience shortcut in the UI. Never errors —
/// returns false on any lookup failure.
pub async fn is_cloudflare_domain(domain: &str) -> bool {
    match query_dns(domain, "NS").await {
        Ok(answers) => answers.iter().any(|a| a.contains("cloudflare.com")),
        Err(_) => false,
    }
}
