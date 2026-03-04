use super::provider::BillingProvider;
use super::types::{CheckoutSession, CreateCheckoutSessionParams};
use worker::*;

const POLAR_API_BASE: &str = "https://api.polar.sh";
const POLAR_SANDBOX_BASE: &str = "https://sandbox-api.polar.sh";

pub struct PolarClient {
    access_token: String,
    sandbox: bool,
}

impl PolarClient {
    pub fn new(access_token: String, sandbox: bool) -> Self {
        Self {
            access_token,
            sandbox,
        }
    }

    fn api_base(&self) -> &str {
        if self.sandbox {
            POLAR_SANDBOX_BASE
        } else {
            POLAR_API_BASE
        }
    }

    async fn post_json(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.api_base(), path);
        let auth = format!("Bearer {}", self.access_token);
        let body_str =
            serde_json::to_string(body).map_err(|e| worker::Error::RustError(e.to_string()))?;

        let headers = Headers::new();
        headers.set("Authorization", &auth)?;
        headers.set("Content-Type", "application/json")?;
        headers.set("Accept", "application/json")?;

        let mut init = RequestInit::new();
        init.with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(wasm_bindgen::JsValue::from_str(&body_str)));

        let req = Request::new_with_init(&url, &init)?;
        let mut resp = Fetch::Request(req).send().await?;

        let status = resp.status_code();
        let json: serde_json::Value = resp.json().await?;

        if status >= 400 {
            let msg = json["detail"]
                .as_str()
                .or_else(|| json["error"].as_str())
                .unwrap_or("Polar API error")
                .to_string();
            return Err(worker::Error::RustError(format!(
                "Polar API error {}: {}",
                status, msg
            )));
        }

        Ok(json)
    }

    async fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.api_base(), path);
        let auth = format!("Bearer {}", self.access_token);

        let headers = Headers::new();
        headers.set("Authorization", &auth)?;
        headers.set("Accept", "application/json")?;

        let mut init = RequestInit::new();
        init.with_method(Method::Get).with_headers(headers);

        let req = Request::new_with_init(&url, &init)?;
        let mut resp = Fetch::Request(req).send().await?;

        let status = resp.status_code();
        let json: serde_json::Value = resp.json().await?;

        if status >= 400 {
            let msg = json["detail"]
                .as_str()
                .or_else(|| json["error"].as_str())
                .unwrap_or("Polar API error")
                .to_string();
            return Err(worker::Error::RustError(format!(
                "Polar API error {}: {}",
                status, msg
            )));
        }

        Ok(json)
    }

    /// Fetches the first non-archived price ID for a given product ID.
    /// Each of our products has exactly one price; this returns its UUID.
    pub async fn list_discounts(&self) -> Result<serde_json::Value> {
        self.get_json("/v1/discounts?limit=100").await
    }

    /// List all products from Polar
    pub async fn list_products(&self) -> Result<serde_json::Value> {
        self.get_json("/v1/products?limit=100").await
    }

    pub async fn fetch_price_id_for_product(&self, product_id: &str) -> Result<String> {
        let json = self
            .get_json(&format!("/v1/products/{}", product_id))
            .await?;

        let prices = json["prices"].as_array().ok_or_else(|| {
            worker::Error::RustError(format!("No prices array for product {}", product_id))
        })?;

        let price_id = prices
            .iter()
            .find(|p| !p["is_archived"].as_bool().unwrap_or(false))
            .and_then(|p| p["id"].as_str())
            .ok_or_else(|| {
                worker::Error::RustError(format!(
                    "No active price found for product {}",
                    product_id
                ))
            })?;

        Ok(price_id.to_string())
    }
}

/// Maps human-readable plan keys to Polar Product IDs (configured via env vars).
/// At runtime these are used to look up the actual Price IDs via the Polar API.
pub struct ProductCatalog {
    pub pro_monthly: String,
    pub pro_annual: String,
    pub business_monthly: String,
    pub business_annual: String,
}

impl ProductCatalog {
    /// Resolve a Polar Price ID (received in webhooks) to (plan, interval).
    /// Requires the price IDs to have been pre-fetched via `build_price_map`.
    pub fn plan_from_price_id(
        price_id: &str,
        price_map: &std::collections::HashMap<String, (&'static str, &'static str)>,
    ) -> (String, String) {
        if let Some((plan, interval)) = price_map.get(price_id) {
            (plan.to_string(), interval.to_string())
        } else {
            ("free".to_string(), "".to_string())
        }
    }
}

/// Fetch price IDs for all configured products and build a price_id → (plan, interval) map.
/// This is called once per webhook request to resolve the price_id from Polar webhooks.
pub async fn build_price_map(
    polar: &PolarClient,
    catalog: &ProductCatalog,
) -> std::collections::HashMap<String, (&'static str, &'static str)> {
    let mut map = std::collections::HashMap::new();

    let entries: &[(&str, &'static str, &'static str)] = &[
        (&catalog.pro_monthly, "pro", "month"),
        (&catalog.pro_annual, "pro", "year"),
        (&catalog.business_monthly, "business", "month"),
        (&catalog.business_annual, "business", "year"),
    ];

    for (product_id, plan, interval) in entries {
        match polar.fetch_price_id_for_product(product_id).await {
            Ok(price_id) => {
                map.insert(price_id, (*plan, *interval));
            }
            Err(e) => {
                worker::console_error!("Failed to fetch price for product {}: {}", product_id, e);
            }
        }
    }

    map
}

pub fn plan_from_price_id(
    price_id: &str,
    price_map: &std::collections::HashMap<String, (&'static str, &'static str)>,
) -> (String, String) {
    ProductCatalog::plan_from_price_id(price_id, price_map)
}

impl BillingProvider for PolarClient {
    async fn create_checkout_session(
        &self,
        params: CreateCheckoutSessionParams,
    ) -> Result<CheckoutSession> {
        let mut body = serde_json::json!({
            "products": [params.price_id],
            "success_url": params.success_url,
            "metadata": {
                "billing_account_id": params.client_reference_id
            },
            "external_customer_id": params.client_reference_id,
        });

        // Add discount if provided
        if let Some(ref discount_id) = params.coupon_id
            && !discount_id.is_empty()
        {
            body["discount_id"] = serde_json::Value::String(discount_id.clone());
        }

        // If a customer already exists in Polar, pre-fill them
        if let Some(ref cid) = params.customer_id
            && !cid.is_empty()
        {
            body["customer_id"] = serde_json::Value::String(cid.clone());
        }

        let json = self.post_json("/v1/checkouts", &body).await?;

        let id = json["id"]
            .as_str()
            .ok_or_else(|| worker::Error::RustError("Missing checkout id".to_string()))?
            .to_string();

        let url = json["url"]
            .as_str()
            .ok_or_else(|| worker::Error::RustError("Missing checkout URL".to_string()))?
            .to_string();

        let customer_id = json["customer_id"].as_str().map(|s| s.to_string());

        Ok(CheckoutSession {
            id,
            url,
            customer_id,
        })
    }
}

pub fn product_catalog_from_env(env: &worker::Env) -> Result<ProductCatalog> {
    Ok(ProductCatalog {
        pro_monthly: env
            .var("POLAR_PRO_MONTHLY_PRODUCT_ID")
            .map(|v| v.to_string())
            .map_err(|_| {
                worker::Error::RustError("POLAR_PRO_MONTHLY_PRODUCT_ID not configured".to_string())
            })?,
        pro_annual: env
            .var("POLAR_PRO_ANNUAL_PRODUCT_ID")
            .map(|v| v.to_string())
            .map_err(|_| {
                worker::Error::RustError("POLAR_PRO_ANNUAL_PRODUCT_ID not configured".to_string())
            })?,
        business_monthly: env
            .var("POLAR_BUSINESS_MONTHLY_PRODUCT_ID")
            .map(|v| v.to_string())
            .map_err(|_| {
                worker::Error::RustError(
                    "POLAR_BUSINESS_MONTHLY_PRODUCT_ID not configured".to_string(),
                )
            })?,
        business_annual: env
            .var("POLAR_BUSINESS_ANNUAL_PRODUCT_ID")
            .map(|v| v.to_string())
            .map_err(|_| {
                worker::Error::RustError(
                    "POLAR_BUSINESS_ANNUAL_PRODUCT_ID not configured".to_string(),
                )
            })?,
    })
}

pub fn polar_client_from_env(env: &worker::Env) -> Result<PolarClient> {
    let access_token = env
        .secret("POLAR_ACCESS_TOKEN")
        .map(|v| v.to_string())
        .map_err(|_| worker::Error::RustError("POLAR_ACCESS_TOKEN not configured".to_string()))?;

    let sandbox = env
        .var("POLAR_SANDBOX")
        .map(|v| v.to_string() == "true")
        .unwrap_or(true);

    Ok(PolarClient::new(access_token, sandbox))
}
