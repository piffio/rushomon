use worker::{Env, Fetch, Request, Response, Result, RouteContext, console_log};

/// Fetch the title of a web page from a given URL
pub async fn fetch_title(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // Only allow POST requests
    if req.method() != worker::Method::Post {
        return Response::error("Method not allowed", 405);
    }

    // Parse the request body
    let body = req.json::<serde_json::Value>().await?;
    let url = match body.get("url").and_then(|v| v.as_str()) {
        Some(url) => url,
        None => return Response::error("URL is required", 400),
    };

    // Basic URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Response::error("Invalid URL format", 400);
    }

    // Fetch the URL
    let fetch_result = fetch_url(url, &ctx.env).await;

    match fetch_result {
        Ok(html) => {
            // Extract title from HTML
            let title = extract_title(&html);

            let response = serde_json::json!({
                "title": title
            });

            Response::from_json(&response)
        }
        Err(e) => {
            console_log!(
                "{}",
                serde_json::json!({
                    "event": "title_fetch_failed",
                    "url": url,
                    "error": e.to_string(),
                    "level": "warn"
                })
            );
            // Return success with null title instead of error, so frontend can handle gracefully
            let response = serde_json::json!({
                "title": null
            });
            Response::from_json(&response)
        }
    }
}

/// Fetch a URL using worker's fetch
async fn fetch_url(url: &str, _env: &Env) -> Result<String> {
    // Use the global fetch available in Cloudflare Workers
    let headers = worker::Headers::new();
    headers.set(
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    )?;
    headers.set(
        "User-Agent",
        "Mozilla/5.0 (compatible; Rushomon-TitleFetcher/1.0)",
    )?;

    let request = worker::Request::new_with_init(
        url,
        worker::RequestInit::new()
            .with_method(worker::Method::Get)
            .with_headers(headers),
    )?;

    // Use the fetch function from the worker crate
    let mut resp = Fetch::Request(request).send().await?;

    if resp.status_code() != 200 {
        return Err(worker::Error::from(format!("HTTP {}", resp.status_code())));
    }

    resp.text().await
}

/// Extract title from HTML content
fn extract_title(html: &str) -> Option<String> {
    // Use a simple approach to find title tag
    let html_lower = html.to_lowercase();

    if let Some(start) = html_lower.find("<title") {
        // Find the end of the opening title tag
        let tag_end = match html[start..].find('>') {
            Some(pos) => start + pos + 1,
            None => return None,
        };

        // Find the closing title tag
        if let Some(end) = html_lower[tag_end..].find("</title>") {
            let title_content = &html[tag_end..tag_end + end];

            let cleaned = title_content
                .replace("&amp;", "&")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#39;", "'")
                .replace("&nbsp;", " ")
                .trim()
                .to_string();

            if cleaned.is_empty() {
                None
            } else {
                Some(cleaned)
            }
        } else {
            None
        }
    } else {
        None
    }
}
