use crate::utils::device::{DeviceType, detect_device};
use minijinja::{Environment, UndefinedBehavior};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Write};
use worker::Request;

// Keep the template helper in frontend/src/lib/components/LinkModal.svelte in sync
// whenever template context fields, functions, or filters change.

pub const MAX_TEMPLATE_SOURCE_BYTES: usize = 4 * 1024;
pub const MAX_RENDERED_URL_BYTES: usize = 8 * 1024;
pub const TEMPLATE_FUEL: u64 = 20_000;
const TEMPLATE_RECURSION_LIMIT: usize = 32;
const ALLOWED_HEADERS: &[&str] = &[
    "accept",
    "accept-language",
    "dnt",
    "referer",
    "sec-ch-ua",
    "sec-ch-ua-mobile",
    "sec-ch-ua-platform",
    "sec-fetch-dest",
    "sec-fetch-mode",
    "user-agent",
];

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateError {
    SourceTooLong,
    InvalidSyntax,
    RenderFailed,
    OutputTooLong,
    InvalidRequest,
}

impl fmt::Display for TemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::SourceTooLong => "Template must be 4096 bytes or less",
            Self::InvalidSyntax => "Invalid template syntax",
            Self::RenderFailed => "Template could not be rendered",
            Self::OutputTooLong => "Rendered URL exceeds 8192 bytes",
            Self::InvalidRequest => "Request context is unavailable",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TemplateError {}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateContext {
    pub request: TemplateRequestContext,
    pub link: TemplateLinkContext,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateRequestContext {
    pub url: String,
    pub method: String,
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub query_string: String,
    pub query: BTreeMap<String, String>,
    pub query_all: BTreeMap<String, Vec<String>>,
    pub headers: BTreeMap<String, String>,
    pub user_agent: String,
    pub referrer: String,
    pub accept_language: String,
    pub country: String,
    pub city: String,
    pub device: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateLinkContext {
    pub short_code: String,
    pub hostname: String,
    pub is_custom_domain: bool,
}

impl TemplateContext {
    pub fn from_request(
        request: &Request,
        short_code: &str,
        is_custom_domain: bool,
    ) -> Result<Self, TemplateError> {
        let url = request.url().map_err(|_| TemplateError::InvalidRequest)?;
        let mut query = BTreeMap::new();
        let mut query_all: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (key, value) in url.query_pairs() {
            query.insert(key.to_string(), value.to_string());
            query_all
                .entry(key.to_string())
                .or_default()
                .push(value.to_string());
        }

        let headers = collect_safe_headers(request.headers().entries());
        let header = |name: &str| headers.get(name).cloned().unwrap_or_default();
        let user_agent = header("user-agent");
        let device = if user_agent.is_empty() {
            DeviceType::Other
        } else {
            detect_device(&user_agent)
        };
        let host = url.host_str().unwrap_or_default().to_ascii_lowercase();

        Ok(Self {
            request: TemplateRequestContext {
                url: url.to_string(),
                method: request.method().to_string(),
                scheme: url.scheme().to_string(),
                host: host.clone(),
                path: url.path().to_string(),
                query_string: url.query().unwrap_or_default().to_string(),
                query,
                query_all,
                user_agent,
                referrer: header("referer"),
                accept_language: header("accept-language"),
                country: request
                    .headers()
                    .get("CF-IPCountry")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                city: request
                    .headers()
                    .get("CF-IPCity")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                headers,
                device: device.as_str().to_string(),
            },
            link: TemplateLinkContext {
                short_code: short_code.to_string(),
                hostname: host,
                is_custom_domain,
            },
        })
    }
}

pub fn validate_redirect_template(source: &str) -> Result<(), TemplateError> {
    validate_source_length(source)?;
    template_environment()
        .template_from_str(source)
        .map(|_| ())
        .map_err(|_| TemplateError::InvalidSyntax)
}

pub fn render_redirect_template(
    source: &str,
    context: &TemplateContext,
) -> Result<String, TemplateError> {
    validate_source_length(source)?;
    let environment = template_environment();
    let template = environment
        .template_from_str(source)
        .map_err(|_| TemplateError::InvalidSyntax)?;
    let mut output = CappedWriter::new(MAX_RENDERED_URL_BYTES);
    template
        .render_captured_to(context, &mut output)
        .map_err(|_| {
            if output.exceeded {
                TemplateError::OutputTooLong
            } else {
                TemplateError::RenderFailed
            }
        })?;
    Ok(output.value)
}

fn validate_source_length(source: &str) -> Result<(), TemplateError> {
    if source.len() > MAX_TEMPLATE_SOURCE_BYTES {
        Err(TemplateError::SourceTooLong)
    } else {
        Ok(())
    }
}

fn template_environment() -> Environment<'static> {
    let mut environment = Environment::new();
    environment.add_filter("dateformat", minijinja_contrib::filters::dateformat);
    environment.add_filter("datetimeformat", minijinja_contrib::filters::datetimeformat);
    environment.add_filter("random", minijinja_contrib::filters::random);
    environment.add_filter("timeformat", minijinja_contrib::filters::timeformat);
    environment.add_function("now", crate::utils::time::now_timestamp);
    environment.add_function("randrange", minijinja_contrib::globals::randrange);
    environment.add_global("RAND_SEED", rand::random::<u64>());
    environment.set_undefined_behavior(UndefinedBehavior::Chainable);
    environment.set_recursion_limit(TEMPLATE_RECURSION_LIMIT);
    environment.set_fuel(Some(TEMPLATE_FUEL));
    environment
}

fn collect_safe_headers(
    entries: impl IntoIterator<Item = (String, String)>,
) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .filter_map(|(name, value)| {
            let name = name.to_ascii_lowercase();
            ALLOWED_HEADERS
                .contains(&name.as_str())
                .then_some((name, value))
        })
        .collect()
}

struct CappedWriter {
    value: String,
    limit: usize,
    exceeded: bool,
}

impl CappedWriter {
    fn new(limit: usize) -> Self {
        Self {
            value: String::with_capacity(limit.min(256)),
            limit,
            exceeded: false,
        }
    }
}

impl Write for CappedWriter {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        if self.value.len().saturating_add(value.len()) > self.limit {
            self.exceeded = true;
            return Err(io::Error::other("rendered URL exceeds output limit"));
        }
        let value = std::str::from_utf8(value).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "template output is not UTF-8")
        })?;
        self.value.push_str(value);
        Ok(value.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> TemplateContext {
        TemplateContext {
            request: TemplateRequestContext {
                url: "https://sho.rt/code?q=hello%20world".to_string(),
                method: "GET".to_string(),
                scheme: "https".to_string(),
                host: "sho.rt".to_string(),
                path: "/code".to_string(),
                query_string: "q=hello%20world".to_string(),
                query: BTreeMap::from([("q".to_string(), "hello world".to_string())]),
                query_all: BTreeMap::from([(
                    "q".to_string(),
                    vec!["first".to_string(), "hello world".to_string()],
                )]),
                headers: BTreeMap::from([("accept-language".to_string(), "en-CA".to_string())]),
                user_agent: "test-agent".to_string(),
                referrer: String::new(),
                accept_language: "en-CA".to_string(),
                country: "CA".to_string(),
                city: "Montreal".to_string(),
                device: "desktop".to_string(),
            },
            link: TemplateLinkContext {
                short_code: "code".to_string(),
                hostname: "sho.rt".to_string(),
                is_custom_domain: false,
            },
        }
    }

    #[test]
    fn renders_request_context_and_control_flow() {
        let rendered = render_redirect_template(
            "https://example.com/{{ request.country | lower }}{% if request.device == 'desktop' %}/desktop{% endif %}",
            &context(),
        )
        .unwrap();
        assert_eq!(rendered, "https://example.com/ca/desktop");
    }

    #[test]
    fn missing_values_are_chainable_and_render_empty() {
        let rendered = render_redirect_template(
            "https://example.com/?q={{ request.query.missing.value }}&header={{ request.headers.missing }}",
            &context(),
        )
        .unwrap();
        assert_eq!(rendered, "https://example.com/?q=&header=");
    }

    #[test]
    fn urlencode_filter_encodes_request_values() {
        let rendered = render_redirect_template(
            "https://example.com/?q={{ request.query.q | urlencode }}",
            &context(),
        )
        .unwrap();
        assert_eq!(rendered, "https://example.com/?q=hello%20world");
    }

    #[test]
    fn random_filter_selects_a_sequence_item() {
        let rendered = render_redirect_template(
            "{{ ['https://a.example', 'https://b.example'] | random }}",
            &context(),
        )
        .unwrap();
        assert!(matches!(
            rendered.as_str(),
            "https://a.example" | "https://b.example"
        ));
    }

    #[test]
    fn randrange_returns_a_value_within_bounds() {
        let rendered = render_redirect_template("{{ randrange(10, 20) }}", &context()).unwrap();
        let value = rendered.parse::<u8>().unwrap();
        assert!((10..20).contains(&value));
    }

    #[test]
    fn date_and_time_filters_format_unix_timestamps() {
        let rendered = render_redirect_template(
            "{{ 0 | dateformat(format='short') }}|{{ 0 | timeformat(format='long') }}|{{ 0 | datetimeformat(format='iso') }}",
            &context(),
        )
        .unwrap();
        assert_eq!(rendered, "1970-01-01|00:00:00|1970-01-01T00:00:00+00:00");
    }

    #[test]
    fn now_returns_a_unix_timestamp() {
        let rendered =
            render_redirect_template("{{ now() | datetimeformat(format='unix') }}", &context())
                .unwrap();
        let value = rendered.parse::<i64>().unwrap();
        assert!(value > 0);
    }

    #[test]
    fn exposes_last_and_all_query_values() {
        let rendered = render_redirect_template(
            "{{ request.query.q }}|{{ request.query_all.q | join(',') }}",
            &context(),
        )
        .unwrap();
        assert_eq!(rendered, "hello world|first,hello world");
    }

    #[test]
    fn rejects_invalid_or_oversized_source() {
        assert_eq!(
            validate_redirect_template("{{ invalid"),
            Err(TemplateError::InvalidSyntax)
        );
        assert_eq!(
            validate_redirect_template(&"x".repeat(MAX_TEMPLATE_SOURCE_BYTES + 1)),
            Err(TemplateError::SourceTooLong)
        );
    }

    #[test]
    fn aborts_during_render_when_output_limit_is_exceeded() {
        let mut context = context();
        context.request.url = "x".repeat(MAX_RENDERED_URL_BYTES + 1);
        assert_eq!(
            render_redirect_template("{{ request.url }}", &context),
            Err(TemplateError::OutputTooLong)
        );
    }

    #[test]
    fn fuel_limits_expensive_templates() {
        assert_eq!(
            render_redirect_template(
                "{% for item in range(1000000) %}{% set value = item %}{% endfor %}https://example.com",
                &context(),
            ),
            Err(TemplateError::RenderFailed)
        );
    }

    #[test]
    fn header_context_uses_a_fixed_allowlist() {
        let headers = collect_safe_headers([
            ("Accept-Language".to_string(), "en-CA".to_string()),
            ("Authorization".to_string(), "secret".to_string()),
            ("Cookie".to_string(), "secret".to_string()),
            ("CF-Connecting-IP".to_string(), "192.0.2.1".to_string()),
            ("True-Client-IP".to_string(), "192.0.2.1".to_string()),
            ("CF-Access-Jwt-Assertion".to_string(), "secret".to_string()),
        ]);
        assert_eq!(
            headers,
            BTreeMap::from([("accept-language".to_string(), "en-CA".to_string())])
        );
    }
}
