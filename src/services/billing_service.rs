/// Billing Service
///
/// Business logic layer for billing operations including:
/// - Billing status and subscription management
/// - Checkout session creation
/// - Customer portal access
/// - Admin billing operations
use crate::billing::polar::PolarClient;
use crate::billing::provider::BillingProvider;
use crate::billing::types::CreateCheckoutSessionParams;
use crate::repositories::billing_repository::{
    BillingAccountDetails, BillingAccountWithStats, BillingRepository,
};
use crate::repositories::{OrgRepository, SettingsRepository};
use crate::utils::get_frontend_url;
use worker::d1::D1Database;
use worker::{Env, console_error};

/// Billing status information for a user
#[derive(Debug, serde::Serialize)]
pub struct BillingStatus {
    pub tier: String,
    pub is_billing_owner: bool,
    pub subscription_status: Option<String>,
    pub subscription_id: Option<String>,
    pub current_period_end: Option<i64>,
    pub cancel_at_period_end: bool,
    pub provider_customer_id: Option<String>,
    pub billing_account_id: Option<String>,
    pub amount_cents: Option<i64>,
    pub currency: Option<String>,
    pub discount_name: Option<String>,
    pub interval: Option<String>,
    pub subscription_plan: Option<String>,
}

/// Checkout session result
#[derive(Debug, serde::Serialize)]
pub struct CheckoutSession {
    pub url: String,
}

/// Portal session result
#[derive(Debug, serde::Serialize)]
pub struct PortalSession {
    pub url: String,
}

pub struct BillingService;

impl BillingService {
    pub fn new() -> Self {
        Self
    }

    /// Get billing status for a user.
    ///
    /// Returns the billing account tier, active subscription details,
    /// and whether the caller is the billing owner.
    /// Auto-creates a billing account for new users if one does not exist.
    pub async fn get_billing_status(
        &self,
        db: &D1Database,
        user_id: &str,
    ) -> Result<BillingStatus, worker::Error> {
        let repo = BillingRepository::new();
        let org_repo = OrgRepository::new();

        let billing_account = match repo.get_for_user(db, user_id).await? {
            Some(ba) => ba,
            None => {
                // Create default org and billing account for new user
                let org = org_repo.create_default(db, user_id, "Personal").await?;
                match repo
                    .get_by_id(db, org.billing_account_id.as_deref().unwrap_or(""))
                    .await?
                {
                    Some(ba) => ba,
                    None => {
                        // Return default free tier status if no billing account
                        return Ok(BillingStatus {
                            tier: "free".to_string(),
                            is_billing_owner: true,
                            subscription_status: None,
                            subscription_id: None,
                            current_period_end: None,
                            cancel_at_period_end: false,
                            provider_customer_id: None,
                            billing_account_id: None,
                            amount_cents: None,
                            currency: None,
                            discount_name: None,
                            interval: None,
                            subscription_plan: None,
                        });
                    }
                }
            }
        };

        let subscription = repo.get_subscription(db, &billing_account.id).await?;
        let is_billing_owner = billing_account.owner_user_id == user_id;

        match subscription {
            Some(sub) => Ok(BillingStatus {
                tier: billing_account.tier,
                is_billing_owner,
                subscription_status: sub["status"].as_str().map(|s| s.to_string()),
                subscription_id: sub["id"].as_str().map(|s| s.to_string()),
                current_period_end: sub["current_period_end"].as_i64(),
                cancel_at_period_end: sub["cancel_at_period_end"].as_i64().unwrap_or(0) != 0,
                provider_customer_id: billing_account.provider_customer_id,
                billing_account_id: Some(billing_account.id),
                amount_cents: sub["amount_cents"].as_i64(),
                currency: sub["currency"].as_str().map(|s| s.to_string()),
                discount_name: sub["discount_name"].as_str().map(|s| s.to_string()),
                interval: sub["interval"].as_str().map(|s| s.to_string()),
                subscription_plan: sub["plan"].as_str().map(|s| s.to_string()),
            }),
            None => Ok(BillingStatus {
                tier: billing_account.tier,
                is_billing_owner,
                subscription_status: None,
                subscription_id: None,
                current_period_end: None,
                cancel_at_period_end: false,
                provider_customer_id: billing_account.provider_customer_id,
                billing_account_id: Some(billing_account.id),
                amount_cents: None,
                currency: None,
                discount_name: None,
                interval: None,
                subscription_plan: None,
            }),
        }
    }

    /// Create a checkout session for a user.
    ///
    /// Looks up the product ID for the given plan, creates or retrieves
    /// the Polar customer, and creates a checkout session.
    pub async fn create_checkout(
        &self,
        db: &D1Database,
        env: &Env,
        user_id: &str,
        plan: &str,
    ) -> Result<CheckoutSession, worker::Error> {
        let settings_repo = SettingsRepository::new();
        let billing_repo = BillingRepository::new();
        let org_repo = OrgRepository::new();

        let settings = settings_repo.get_all_settings(db).await?;

        // Map plan to product ID setting key
        let product_id_key = match plan {
            "pro_monthly" => "product_pro_monthly_id",
            "pro_annual" => "product_pro_annual_id",
            "business_monthly" => "product_business_monthly_id",
            "business_annual" => "product_business_annual_id",
            _ => {
                console_error!("[checkout] Invalid plan: {}", plan);
                return Err(worker::Error::RustError("Invalid plan".to_string()));
            }
        };

        let polar_product_id = match settings.get(product_id_key) {
            Some(id) => id.clone(),
            None => {
                console_error!("[checkout] Product ID not found for plan: {}", plan);
                return Err(worker::Error::RustError("Plan not configured".to_string()));
            }
        };

        // Check for founder pricing / discount
        let coupon_id = if settings
            .get("founder_pricing_active")
            .map(|v| v == "true")
            .unwrap_or(false)
        {
            let discount_key = match plan {
                "pro_monthly" => "active_discount_pro_monthly",
                "pro_annual" => "active_discount_pro_annual",
                "business_monthly" => "active_discount_business_monthly",
                "business_annual" => "active_discount_business_annual",
                _ => "",
            };
            settings
                .get(discount_key)
                .cloned()
                .filter(|id| !id.is_empty())
        } else {
            None
        };

        // Initialize Polar client
        let polar = self.create_polar_client(env)?;

        // Get or create billing account
        let billing_account = match billing_repo.get_for_user(db, user_id).await? {
            Some(ba) => ba,
            None => {
                let org = org_repo.create_default(db, user_id, "Personal").await?;
                match billing_repo
                    .get_by_id(db, org.billing_account_id.as_deref().unwrap_or(""))
                    .await?
                {
                    Some(ba) => ba,
                    None => {
                        console_error!("[checkout] Failed to create billing account");
                        return Err(worker::Error::RustError(
                            "Failed to create billing account".to_string(),
                        ));
                    }
                }
            }
        };

        // Get or create Polar customer
        let polar_customer_id = if let Some(existing_id) = &billing_account.provider_customer_id {
            Some(existing_id.clone())
        } else {
            match polar
                .find_customer_by_external_id(&billing_account.id)
                .await
            {
                Ok(Some(cid)) => {
                    if let Err(e) = billing_repo
                        .update_provider_customer_id(db, &billing_account.id, &cid)
                        .await
                    {
                        console_error!("[checkout] Failed to store found customer_id: {}", e);
                    }
                    Some(cid)
                }
                Ok(None) => None,
                Err(e) => {
                    console_error!(
                        "[checkout] Failed to query Polar for existing customer: {}",
                        e
                    );
                    None
                }
            }
        };

        // Build checkout params
        let frontend_url = get_frontend_url(env);
        let success_url = format!(
            "{}/billing/success?session_id={{CHECKOUT_SESSION_ID}}",
            frontend_url
        );
        let cancel_url = format!("{}/billing/cancelled", frontend_url);

        let params = CreateCheckoutSessionParams {
            billing_account_id: billing_account.id.clone(),
            customer_id: polar_customer_id,
            price_id: polar_product_id,
            success_url,
            cancel_url,
            coupon_id,
            client_reference_id: billing_account.id.clone(),
        };

        // Create checkout session
        match polar.create_checkout_session(params).await {
            Ok(session) => Ok(CheckoutSession { url: session.url }),
            Err(e) => {
                console_error!("[checkout] Polar API error: {}", e);
                Err(worker::Error::RustError(
                    "Failed to create checkout session".to_string(),
                ))
            }
        }
    }

    /// Create a customer portal session.
    ///
    /// Generates a Polar Customer Portal URL for the authenticated user
    /// to manage their subscription, payment methods, and invoices.
    pub async fn create_portal_session(
        &self,
        db: &D1Database,
        env: &Env,
        user_id: &str,
    ) -> Result<PortalSession, worker::Error> {
        let polar = self.create_polar_client(env)?;

        let billing_account = match BillingRepository::new().get_for_user(db, user_id).await? {
            Some(ba) => ba,
            None => {
                return Err(worker::Error::RustError(
                    "No billing account found".to_string(),
                ));
            }
        };

        let customer_id = match billing_account.provider_customer_id {
            Some(ref id) if !id.is_empty() => id.clone(),
            _ => {
                console_error!(
                    "[portal] No Polar customer_id for billing_account: {}",
                    billing_account.id
                );
                return Err(worker::Error::RustError(
                    "No billing account found. Please create a subscription first.".to_string(),
                ));
            }
        };

        let frontend_url = get_frontend_url(env);
        let return_url = format!("{}/billing", frontend_url);

        match polar
            .create_customer_portal_session(&customer_id, &return_url)
            .await
        {
            Ok(portal_url) => Ok(PortalSession { url: portal_url }),
            Err(e) => {
                console_error!("[portal] Polar API error: {}", e);
                Err(worker::Error::RustError(
                    "Failed to create portal session".to_string(),
                ))
            }
        }
    }

    /// Helper to create Polar client from environment
    fn create_polar_client(&self, env: &Env) -> Result<PolarClient, worker::Error> {
        let api_key = match env.secret("POLAR_API_KEY") {
            Ok(key) => key.to_string(),
            Err(_) => {
                console_error!("[billing] POLAR_API_KEY not configured");
                return Err(worker::Error::RustError(
                    "Billing not configured".to_string(),
                ));
            }
        };

        // Check if using sandbox mode
        let sandbox = env
            .var("POLAR_SANDBOX")
            .map(|v| v.to_string() == "true")
            .unwrap_or(false);

        Ok(PolarClient::new(api_key, sandbox))
    }

    // ─── Admin Methods ────────────────────────────────────────────────────────

    /// List all billing accounts with stats (admin only).
    pub async fn admin_list_billing_accounts(
        &self,
        db: &D1Database,
        page: i64,
        limit: i64,
        search: Option<&str>,
        tier_filter: Option<&str>,
    ) -> Result<(Vec<BillingAccountWithStats>, i64), worker::Error> {
        let repo = BillingRepository::new();
        repo.list_for_admin(db, page, limit, search, tier_filter)
            .await
    }

    /// Get detailed billing account information (admin only).
    pub async fn admin_get_billing_account(
        &self,
        db: &D1Database,
        billing_account_id: &str,
    ) -> Result<Option<BillingAccountDetails>, worker::Error> {
        let repo = BillingRepository::new();
        repo.get_details(db, billing_account_id).await
    }
}
