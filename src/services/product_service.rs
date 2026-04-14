/// Product Service
///
/// Business logic for product catalog management:
/// - Syncing products from the Polar billing provider
/// - Fetching products for the public pricing page
use crate::billing::polar::PolarClient;
use crate::repositories::{ProductRepository, SettingsRepository};
use worker::d1::D1Database;
use worker::*;

pub struct ProductService;

impl ProductService {
    pub fn new() -> Self {
        Self
    }

    /// Fetch raw products from the Polar API and cache them in the local database.
    /// Returns the Polar API response payload on success.
    pub async fn sync_from_polar(
        &self,
        db: &D1Database,
        polar: &PolarClient,
    ) -> Result<serde_json::Value> {
        let products = polar.list_products().await?;
        ProductRepository::new().replace_all(db, &products).await?;
        Ok(products)
    }

    /// Build the public pricing page response from locally cached products.
    /// Reads the configured Polar product IDs from settings and returns
    /// one entry per product variant (pro_monthly, pro_annual, etc.).
    pub async fn list_for_pricing(&self, db: &D1Database) -> Result<Vec<serde_json::Value>> {
        let settings = SettingsRepository::new().get_all_settings(db).await?;
        let repo = ProductRepository::new();

        let product_keys = [
            ("product_pro_monthly_id", "product_pro_monthly_id"),
            ("product_pro_annual_id", "product_pro_annual_id"),
            ("product_business_monthly_id", "product_business_monthly_id"),
            ("product_business_annual_id", "product_business_annual_id"),
        ];

        let mut products = Vec::new();

        for (setting_key, label_id) in &product_keys {
            let product_id = match settings.get(*setting_key) {
                Some(id) if !id.is_empty() => id.clone(),
                _ => continue,
            };

            if let Ok(Some(mut entry)) = repo.get_by_product_id(db, &product_id).await {
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert(
                        "id".to_string(),
                        serde_json::Value::String(label_id.to_string()),
                    );
                }
                products.push(entry);
            }
        }

        Ok(products)
    }
}

impl Default for ProductService {
    fn default() -> Self {
        Self::new()
    }
}
