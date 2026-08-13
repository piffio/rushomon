---
sidebar_position: 3
title: Template Redirects
---

# Template redirects

Pro, Business, and Unlimited organizations can render a destination from each
visitor request. Choose **Template** when creating or editing a link, then enter
an inline MiniJinja URL template:

```text
https://example.com/search?q={{ request.query.q | default('') | urlencode }}
```

Template links always use a `307` temporary redirect and return `Cache-Control:
private, no-store`. This prevents a browser or intermediary from caching one
visitor's rendered destination for another visitor.

## Request context

The following values are available:

| Value | Description |
| --- | --- |
| `request.url` | Full incoming URL |
| `request.method` | HTTP method, such as `GET` or `HEAD` |
| `request.scheme` | Incoming URL scheme |
| `request.host` | Lowercase request hostname |
| `request.path` | Incoming URL path |
| `request.query_string` | Raw query string |
| `request.query` | Last value for each decoded query key |
| `request.query_all` | All decoded values for each query key |
| `request.user_agent` | Visitor User-Agent, if provided |
| `request.referrer` | Visitor referrer, if provided |
| `request.accept_language` | Visitor language preferences, if provided |
| `request.country` | Cloudflare country code, if available |
| `request.city` | Cloudflare city, if available |
| `request.device` | `ios`, `android`, `desktop`, or `other` |
| `link.short_code` | Current link's short code |
| `link.hostname` | Hostname used for this request |
| `link.is_custom_domain` | Whether the request used a custom domain |

`request.headers` contains only these lowercase, non-credential headers:
`accept`, `accept-language`, `dnt`, `referer`, `sec-ch-ua`,
`sec-ch-ua-mobile`, `sec-ch-ua-platform`, `sec-fetch-dest`,
`sec-fetch-mode`, and `user-agent`. Use bracket access for names containing a
hyphen, for example `request.headers['accept-language']`.

Cookies, authorization values, IP and forwarding headers, Cloudflare Access
credentials, environment bindings, and other sensitive headers are
intentionally unavailable. Missing values render as empty strings. Use the
`default` filter when an explicit fallback makes the template clearer.

Always apply `urlencode` to request values used in a URL path or query value.
Rushomon validates the rendered result as an absolute HTTP or HTTPS URL before
redirecting and applies blacklist rules to the rendered destination.

## Supported subset

Template redirects support inline variable expressions (`{{ }}`), `if`, `for`,
and `set` blocks, whitespace control, and MiniJinja's built-in filters including
`default` and `urlencode`. They also provide `random` for selecting an item from
a sequence and `randrange` for generating an integer in a range:

```text
{{ ["https://a.example", "https://b.example"] | random }}
```

```text
{% if randrange(100) < 20 %}https://a.example{% else %}https://b.example{% endif %}
```

These utilities are suitable for traffic distribution, not security-sensitive
decisions. Each request is selected independently, so retries can use a
different destination.

The `now()` function returns the current UTC Unix timestamp in seconds. Use
`dateformat`, `timeformat`, or `datetimeformat` to create time-based rules or URL
values:

```text
https://example.com/archive/{{ now() | dateformat(format="short") }}
```

The format filters accept `short`, `medium`, `long`, `full`, and documented
MiniJinja-contrib format strings. `datetimeformat` and `timeformat` also accept
`iso` and `unix`. Named timezone conversion is not enabled; do not pass a `tz`
argument. `now()` is always UTC.

Template source is limited to 4 KiB, rendered output to 8 KiB, and execution to
20,000 MiniJinja fuel units to protect redirect availability.

Templates use the MiniJinja runtime. Includes, imports, template inheritance,
asynchronous filters, custom global functions, and unsupported filters such as
`dump` and `nl2br` are not available. Syntax errors are rejected when the link
is saved. A runtime error, invalid rendered URL, or unavailable moderation
check sends the visitor to the standard not-found page and does not record a
click.

Device-specific destinations use the same mode as the default destination. If
a link is a template link, every configured device destination must also be a
valid template.

## CSV and deployment

Exports include an `is_template` column so a template cannot be mistaken for a
static destination. CSV import deliberately rejects template rows; create them
through the link form or API so their required `307` behavior is explicit.

When deploying this schema, publish the template-aware Worker before enabling
template writes in the UI or API. An older Worker ignores the KV mode flag and
can interpret template source as a static destination. If the Worker must be
rolled back, disable template links and resynchronize their KV mappings first.
