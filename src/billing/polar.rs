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

    /// Creates a Polar Customer Portal session and returns the portal URL.
    pub async fn create_customer_portal_session(
        &self,
        customer_id: &str,
        return_url: &str,
    ) -> Result<String> {
        let body = serde_json::json!({
            "customer_id": customer_id,
            "return_url": return_url,
        });

        let json = self.post_json("/v1/customer-sessions", &body).await?;

        let portal_url = json["customer_portal_url"].as_str().ok_or_else(|| {
            worker::Error::RustError("Missing customer_portal_url in response".to_string())
        })?;

        Ok(portal_url.to_string())
    }

    /// Finds a Polar customer by external_id (our billing_account_id).
    /// Returns the customer ID if found, None if not found.
    /// Used to prevent duplicate customer creation during checkout.
    pub async fn find_customer_by_external_id(&self, external_id: &str) -> Result<Option<String>> {
        // URL encode the external_id to handle special characters
        let encoded_id = urlencoding::encode(external_id);
        let path = format!("/v1/customers?external_id={}", encoded_id);
        let json = self.get_json(&path).await?;

        let customers = json["items"].as_array().ok_or_else(|| {
            worker::Error::RustError("No items array in Polar customers response".to_string())
        })?;

        // Return first non-archived customer's ID
        for customer in customers {
            let is_archived = customer["is_archived"].as_bool().unwrap_or(false);
            if !is_archived && let Some(id) = customer["id"].as_str() {
                return Ok(Some(id.to_string()));
            }
        }

        Ok(None)
    }
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
