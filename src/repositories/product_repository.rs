use worker::Result;
/// Product Repository
///
/// Data access layer for the `cached_products` table — Polar product data
/// that has been synced locally for fast lookup during webhook processing
/// and pricing display.
use worker::d1::D1Database;

pub struct ProductRepository;

impl ProductRepository {
    pub fn new() -> Self {
        Self
    }

    /// Look up a cached product row by its Polar price ID.
    /// Used by webhook handlers to resolve plan name and interval.
    /// Note: webhook handler still uses db::get_cached_product_by_price_id until Step 14 (Billing).
    #[allow(dead_code)]
    pub async fn get_by_price_id(
        &self,
        db: &D1Database,
        price_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        let stmt = db.prepare(
            "SELECT id, name, description, price_amount, price_currency,
                    recurring_interval, recurring_interval_count, is_archived,
                    polar_product_id, polar_price_id, created_at, updated_at
             FROM cached_products
             WHERE polar_price_id = ?1 AND is_archived = FALSE",
        );
        stmt.bind(&[price_id.into()])?
            .first::<serde_json::Value>(None)
            .await
    }

    /// Look up a cached product row by its Polar product ID.
    /// Used by the pricing endpoint to display plan info.
    pub async fn get_by_product_id(
        &self,
        db: &D1Database,
        product_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        let stmt = db.prepare(
            "SELECT id, name, description, price_amount, price_currency,
                    recurring_interval, recurring_interval_count, is_archived,
                    polar_product_id, polar_price_id
             FROM cached_products
             WHERE polar_product_id = ?1 AND is_archived = FALSE",
        );
        stmt.bind(&[product_id.into()])?
            .first::<serde_json::Value>(None)
            .await
    }

    /// Replace the entire `cached_products` table with fresh data from Polar.
    /// Deletes all existing rows then inserts one row per price variant.
    pub async fn replace_all(&self, db: &D1Database, products: &serde_json::Value) -> Result<()> {
        let delete_stmt = db.prepare("DELETE FROM cached_products");
        delete_stmt.run().await?;

        let Some(items) = products.get("items").and_then(|i| i.as_array()) else {
            return Ok(());
        };

        for product in items {
            let product_id = product.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let product_name = product.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let product_description = product
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");

            let Some(prices) = product.get("prices").and_then(|p| p.as_array()) else {
                continue;
            };

            for price in prices {
                let price_amount = price
                    .get("price_amount")
                    .and_then(|p| p.as_i64())
                    .unwrap_or(0) as i32;
                let price_currency = price
                    .get("price_currency")
                    .and_then(|c| c.as_str())
                    .unwrap_or("EUR")
                    .to_string();
                let price_id = price
                    .get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();
                let recurring_interval = price.get("recurring_interval").and_then(|i| i.as_str());
                let recurring_interval_count = price
                    .get("recurring_interval_count")
                    .and_then(|c| c.as_i64());

                let insert_stmt = db.prepare(
                    "INSERT INTO cached_products (
                        id, name, description, price_amount, price_currency,
                        recurring_interval, recurring_interval_count, is_archived,
                        polar_product_id, polar_price_id, created_at, updated_at
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                );

                let now = worker::Date::now().to_string();
                insert_stmt
                    .bind(&[
                        price_id.clone().into(),
                        product_name.into(),
                        product_description.into(),
                        (price_amount as f64).into(),
                        price_currency.into(),
                        recurring_interval.unwrap_or("").into(),
                        (recurring_interval_count.unwrap_or(1) as f64).into(),
                        false.into(),
                        product_id.into(),
                        price_id.into(),
                        now.clone().into(),
                        now.into(),
                    ])?
                    .run()
                    .await?;
            }
        }

        Ok(())
    }
}

impl Default for ProductRepository {
    fn default() -> Self {
        Self::new()
    }
}
