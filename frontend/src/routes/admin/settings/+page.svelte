<script lang="ts">
  import { onMount } from "svelte";
  import { adminApi, type Discount, type Product } from "$lib/api/admin";
  import { billingApi } from "$lib/api/billing";

  let settings = $state<Record<string, string>>({});
  let loading = $state(false);
  let saving = $state(false);
  let showToast = $state(false);
  let toastMessage = $state("");
  let signupsEnabled = $state(true);
  let defaultUserTier = $state("free");
  let founderPricingEnabled = $state(false);
  let confirmingSignupToggle = $state(false);
  let confirmingFounderPricingToggle = $state(false);

  // System Operations state
  let cronLoading = $state(false);
  let cronResult = $state<{
    processed: number;
    success: number;
    errors: number;
  } | null>(null);
  let showCronResult = $state(false);

  let discounts = $state<Discount[]>([]);
  let discountsLoading = $state(false);
  let discountsError = $state("");
  let discountSlots = $state({
    pro_monthly: "",
    pro_annual: "",
    business_monthly: "",
    business_annual: ""
  });

  // Product management state
  let products = $state<Product[]>([]);
  let productsLoading = $state(false);
  let productsError = $state("");
  let syncingProducts = $state(false);
  let productSlots = $state({
    pro_monthly: "",
    pro_annual: "",
    business_monthly: "",
    business_annual: ""
  });

  let currentPricing = $state<any[]>([]);
  let pricingLoading = $state(false);
  let pricingError = $state("");
  let initialProductSlots = $state({
    pro_monthly: "",
    pro_annual: "",
    business_monthly: "",
    business_annual: ""
  });

  // Product IDs - will be populated from loaded products
  let PRODUCT_IDS = $state({
    pro_monthly: "",
    pro_annual: "",
    business_monthly: "",
    business_annual: ""
  });

  // Filter discounts for a specific product
  function getDiscountsForSlot(slot: keyof typeof PRODUCT_IDS): Discount[] {
    const productId = PRODUCT_IDS[slot];
    return discounts.filter(
      (d) =>
        !d.products || // If no products array, discount applies to all
        d.products.some((p) => p.id === productId)
    );
  }

  // Get pricing info for a specific slot
  function getPricingInfoForSlot(
    slot: keyof typeof PRODUCT_IDS,
    discount: Discount
  ): { original: string; discounted: string } | null {
    // Use our known base prices since Polar API doesn't include price in products array
    const basePrices = {
      pro_monthly: 900, // €9.00
      pro_annual: 9000, // €90.00
      business_monthly: 2900, // €29.00
      business_annual: 29000 // €290.00
    };

    const basePriceCents = basePrices[slot];
    const basePrice = basePriceCents / 100; // Convert cents to euros
    let discountedPrice = basePrice;

    if (discount.type === "fixed") {
      discountedPrice = Math.max(0, basePrice - (discount.amount || 0) / 100);
    } else {
      discountedPrice = basePrice * (1 - (discount.basis_points || 0) / 10000);
    }

    return {
      original: `€${basePrice.toFixed(2)}`,
      discounted: `€${discountedPrice.toFixed(2)}`
    };
  }

  onMount(() => {
    loadSettings();
    loadDiscounts();
    loadProducts();
  });

  async function loadSettings() {
    try {
      loading = true;
      settings = await adminApi.getSettings();
      signupsEnabled = settings.signups_enabled !== "false";
      defaultUserTier = settings.default_user_tier || "free";
      founderPricingEnabled = settings.founder_pricing_active === "true";
      discountSlots.pro_monthly = settings.active_discount_pro_monthly || "";
      discountSlots.pro_annual = settings.active_discount_pro_annual || "";
      discountSlots.business_monthly =
        settings.active_discount_business_monthly || "";
      discountSlots.business_annual =
        settings.active_discount_business_annual || "";

      // Load product slots and store initial values for change detection
      productSlots.pro_monthly = settings.product_pro_monthly_id || "";
      productSlots.pro_annual = settings.product_pro_annual_id || "";
      productSlots.business_monthly =
        settings.product_business_monthly_id || "";
      productSlots.business_annual = settings.product_business_annual_id || "";

      // Store initial values to track changes
      initialProductSlots = { ...productSlots };

      // Populate PRODUCT_IDS from database settings
      PRODUCT_IDS = {
        pro_monthly: productSlots.pro_monthly,
        pro_annual: productSlots.pro_annual,
        business_monthly: productSlots.business_monthly,
        business_annual: productSlots.business_annual
      };

      // Load current pricing data
      await loadCurrentPricing();
    } catch (err) {
      console.error("Failed to load settings:", err);
    } finally {
      loading = false;
    }
  }

  async function loadCurrentPricing() {
    try {
      pricingLoading = true;
      // Use the billing API to get pricing data from backend
      const data = await billingApi.getPricing();
      currentPricing = data.products || [];
    } catch (err) {
      console.error("Failed to load current pricing:", err);
      pricingError = "Failed to load current pricing data";
      currentPricing = [];
    } finally {
      pricingLoading = false;
    }
  }

  async function loadDiscounts() {
    try {
      discountsLoading = true;
      discountsError = "";
      const res = await adminApi.listDiscounts();
      discounts = res.items || [];
    } catch (err) {
      console.error("Failed to load discounts:", err);
      discountsError =
        "Could not load discounts from Polar. Check your API configuration.";
    } finally {
      discountsLoading = false;
    }
  }

  async function loadProducts() {
    try {
      productsLoading = true;
      productsError = "";
      const res = await adminApi.listProducts();
      products = res.items || [];
    } catch (err) {
      console.error("Failed to load products:", err);
      productsError =
        "Could not load products from Polar. Check your API configuration.";
    } finally {
      productsLoading = false;
    }
  }

  async function handleDiscountSlotChange(key: string, value: string) {
    try {
      saving = true;
      settings = await adminApi.updateSetting(key, value);

      // Also persist the discount amount so the public pricing page can show it
      const amountKey = key.replace(
        "active_discount_",
        "active_discount_amount_"
      );
      const selectedDiscount = discounts.find((d) => d.id === value);
      let amount = "0";

      if (selectedDiscount) {
        if (selectedDiscount.type === "fixed") {
          amount = String(selectedDiscount.amount || 0);
        } else {
          // For percentage discounts, calculate the amount based on the base price
          const slot = key
            .replace("active_discount_", "")
            .replace("_monthly", "_monthly")
            .replace("_annual", "_annual");
          const basePrices = {
            pro_monthly: 900,
            pro_annual: 9000,
            business_monthly: 2900,
            business_annual: 29000
          };
          const basePrice = basePrices[slot as keyof typeof basePrices] || 0;
          const discountAmount = Math.round(
            (basePrice * (selectedDiscount.basis_points || 0)) / 10000
          );
          amount = String(discountAmount);
        }
      }

      settings = await adminApi.updateSetting(amountKey, amount);

      showToastMessage("Discount updated");
    } catch (err) {
      console.error("Failed to update discount:", err);
      showToastMessage("Failed to update discount");
    } finally {
      saving = false;
    }
  }

  async function syncProducts() {
    try {
      syncingProducts = true;
      const result = await adminApi.syncProducts();
      console.log("Products sync result:", result);

      if (result.success) {
        showToastMessage(
          `Fetched ${result.products_count} products from Polar`
        );
        // Reload products to show updated data
        await loadProducts();
      } else {
        showToastMessage("Failed to fetch products");
      }
    } catch (err) {
      console.error("Failed to fetch products:", err);
      showToastMessage("Failed to fetch products");
    } finally {
      syncingProducts = false;
    }
  }

  // Check if there are unsaved changes
  function hasUnsavedChanges(): boolean {
    return (
      productSlots.pro_monthly !== initialProductSlots.pro_monthly ||
      productSlots.pro_annual !== initialProductSlots.pro_annual ||
      productSlots.business_monthly !== initialProductSlots.business_monthly ||
      productSlots.business_annual !== initialProductSlots.business_annual
    );
  }

  // Get current price for a product slot
  function getCurrentPrice(slot: keyof typeof productSlots): string {
    if (!currentPricing || currentPricing.length === 0) return "Not configured";

    const settingKey = `product_${slot}_id`;
    const product = currentPricing.find((p: any) => p.id === settingKey);
    if (!product) return "Not configured";

    const amount = product.price_amount || 0;
    const currency = product.price_currency || "EUR";
    const interval = product.recurring_interval;

    const price = (amount / 100).toFixed(2);
    return interval ? `€${price}/${interval}` : `€${price}`;
  }

  // Get selected product price
  function getSelectedProductPrice(slot: keyof typeof productSlots): string {
    const productId = productSlots[slot];

    // If no selection or same as current, show "NA"
    if (!productId || productId === initialProductSlots[slot] || !products) {
      return "NA";
    }

    const product = products.find((p: Product) => p.id === productId);
    if (!product || !product.prices || product.prices.length === 0) return "NA";

    const price = product.prices[0];
    const amount = (price.price_amount / 100).toFixed(2);
    const interval = price.recurring_interval;

    return interval ? `€${amount}/${interval}` : `€${amount}`;
  }

  async function saveProductConfiguration() {
    try {
      saving = true;

      // Save product slot assignments one by one
      await adminApi.updateSetting(
        "product_pro_monthly_id",
        productSlots.pro_monthly
      );
      await adminApi.updateSetting(
        "product_pro_annual_id",
        productSlots.pro_annual
      );
      await adminApi.updateSetting(
        "product_business_monthly_id",
        productSlots.business_monthly
      );
      await adminApi.updateSetting(
        "product_business_annual_id",
        productSlots.business_annual
      );

      // Cache products from Polar
      const result = await adminApi.saveProducts();
      console.log("Products cached:", result);

      if (result.success) {
        // Update initial values to reflect saved state
        initialProductSlots = { ...productSlots };

        // Reload current pricing data
        await loadCurrentPricing();

        showToastMessage("Product configuration saved and cached successfully");
      } else {
        showToastMessage(
          "Product configuration saved, but failed to cache products"
        );
      }
    } catch (err) {
      console.error("Failed to save product configuration:", err);
      showToastMessage("Failed to save product configuration");
    } finally {
      saving = false;
    }
  }

  function formatDiscountLabel(
    d: Discount,
    slot?: keyof typeof PRODUCT_IDS
  ): string {
    let amount: string;
    if (d.type === "fixed") {
      amount = `€${((d.amount || 0) / 100).toFixed(0)} off`;
    } else {
      // basis_points is in hundredths of a percent (e.g., 3400 = 34%)
      const percentage = ((d.basis_points || 0) / 100).toFixed(0);
      amount = `${percentage}% off`;
    }

    let pricingInfo = "";
    if (slot) {
      const pricing = getPricingInfoForSlot(slot, d);
      if (pricing) {
        pricingInfo = ` — ${pricing.original} → ${pricing.discounted}`;
      }
    }

    const used = d.max_redemptions
      ? `${d.redemptions_count}/${d.max_redemptions} used`
      : `${d.redemptions_count} used`;
    return `${d.name} — ${amount}${pricingInfo} (${used})`;
  }

  async function handleSignupToggle() {
    confirmingSignupToggle = true;
  }

  async function confirmSignupToggle() {
    saving = true;
    try {
      const newValue = signupsEnabled ? "false" : "true";
      const updatedSettings = await adminApi.updateSetting(
        "signups_enabled",
        newValue
      );
      settings = updatedSettings;
      signupsEnabled = updatedSettings.signups_enabled !== "false";
      showToastMessage(`Signups ${signupsEnabled ? "enabled" : "disabled"}`);
    } catch (err) {
      console.error("Failed to update setting:", err);
      showToastMessage("Failed to update setting");
    } finally {
      saving = false;
      confirmingSignupToggle = false;
    }
  }

  async function handleDefaultTierChange() {
    saving = true;
    try {
      const updatedSettings = await adminApi.updateSetting(
        "default_user_tier",
        defaultUserTier
      );
      settings = updatedSettings;
      defaultUserTier = updatedSettings.default_user_tier || "free";
      showToastMessage(`Default tier updated to ${defaultUserTier}`);
    } catch (err) {
      console.error("Failed to update setting:", err);
      showToastMessage("Failed to update setting");
    } finally {
      saving = false;
    }
  }

  async function handleFounderPricingToggle() {
    confirmingFounderPricingToggle = true;
  }

  async function confirmFounderPricingToggle() {
    saving = true;
    try {
      const newValue = founderPricingEnabled ? "false" : "true";
      const updatedSettings = await adminApi.updateSetting(
        "founder_pricing_active",
        newValue
      );
      settings = updatedSettings;
      founderPricingEnabled = updatedSettings.founder_pricing_active === "true";
      showToastMessage(
        `Founder pricing ${founderPricingEnabled ? "enabled" : "disabled"}`
      );
    } catch (err) {
      console.error("Failed to update setting:", err);
      showToastMessage("Failed to update setting");
    } finally {
      saving = false;
      confirmingFounderPricingToggle = false;
    }
  }

  async function handleUpdateSetting(key: string, value: string) {
    try {
      saving = true;
      settings = await adminApi.updateSetting(key, value);
      showToastMessage("Setting updated");
    } catch (err) {
      console.error("Failed to update setting:", err);
      showToastMessage("Failed to update setting");
    } finally {
      saving = false;
    }
  }

  function showToastMessage(message: string) {
    toastMessage = message;
    showToast = true;
    setTimeout(() => {
      showToast = false;
    }, 3000);
  }

  async function triggerCronJob() {
    try {
      cronLoading = true;
      const result = await adminApi.triggerCronDowngrade();
      cronResult = result;
      showCronResult = true;
    } catch (err) {
      console.error("Failed to trigger cron job:", err);
      showToastMessage("Failed to trigger cron job");
    } finally {
      cronLoading = false;
    }
  }
</script>

<div class="settings-page">
  <div class="page-header">
    <h1>Instance Settings</h1>
    <p class="subtitle">Configure system-wide settings and policies</p>
  </div>

  {#if loading}
    <div class="loading">Loading settings...</div>
  {:else}
    <div class="settings-container">
      <!-- Registration Setting -->
      <div class="setting-card">
        <div class="setting-content">
          <div class="setting-info">
            <h3>Allow new signups</h3>
            <p class="setting-description">
              Control whether new users can create accounts on this instance
            </p>
          </div>
          <div class="setting-control">
            <button
              onclick={handleSignupToggle}
              disabled={saving}
              class="toggle-switch {signupsEnabled ? 'enabled' : 'disabled'}"
              role="switch"
              aria-checked={signupsEnabled}
              aria-label="Toggle new signups"
            >
              <span class="toggle-slider"></span>
            </button>
          </div>
        </div>
      </div>

      <!-- Default User Tier Setting -->
      <div class="setting-card">
        <div class="setting-content">
          <div class="setting-info">
            <h3>Default tier for new users</h3>
            <p class="setting-description">
              New signups will be assigned this tier by default
            </p>
          </div>
          <div class="setting-control">
            <select
              bind:value={defaultUserTier}
              onchange={handleDefaultTierChange}
              disabled={saving}
              class="tier-select"
            >
              <option value="free">Free</option>
              <option value="unlimited">Unlimited</option>
            </select>
          </div>
        </div>
      </div>

      <!-- Pricing Section -->
      <div class="pricing-section">
        <h2 class="pricing-section-header">💰 Pricing Configuration</h2>
        <p class="pricing-section-description">
          Configure products and discounts for pricing calculations
        </p>
      </div>

      <!-- Product Configuration Setting -->
      <div class="setting-card">
        <div class="setting-info" style="margin-bottom: 1rem;">
          <h3>📦 Product Configuration</h3>
          <p class="setting-description">
            Assign Polar products to each plan. Used for checkout.
          </p>
          <div class="sync-button-container">
            <button
              onclick={syncProducts}
              disabled={syncingProducts || productsLoading}
              class="sync-button"
            >
              {syncingProducts
                ? "Fetching..."
                : "🔄 Fetch all products from Polar"}
            </button>
            <button
              onclick={saveProductConfiguration}
              disabled={saving || productsLoading}
              class="save-button"
            >
              {saving ? "Saving..." : "💾 Save & Cache Products"}
            </button>
          </div>
        </div>
        {#if productsLoading}
          <p class="setting-description">Loading products from Polar…</p>
        {:else if productsError}
          <p class="setting-description" style="color: #dc2626;">
            {productsError}
          </p>
        {:else}
          <div class="product-grid">
            {#each [{ key: "product_pro_monthly_id", label: "Pro Monthly", slot: "pro_monthly" as const }, { key: "product_pro_annual_id", label: "Pro Annual", slot: "pro_annual" as const }, { key: "product_business_monthly_id", label: "Business Monthly", slot: "business_monthly" as const }, { key: "product_business_annual_id", label: "Business Annual", slot: "business_annual" as const }] as row}
              <div
                class="product-row"
                class:unsaved={productSlots[row.slot] !==
                  initialProductSlots[row.slot]}
              >
                <div class="product-header">
                  <label class="product-label" for="product-{row.slot}">
                    {row.label}
                  </label>
                  <!-- Status icon with tooltip -->
                  {#if productSlots[row.slot] !== initialProductSlots[row.slot]}
                    <div class="status-icon unsaved" title="Unsaved changes">
                      ⚠️
                    </div>
                  {:else}
                    <div class="status-icon saved" title="Saved">✓</div>
                  {/if}
                </div>

                <div class="product-pricing-info">
                  <!-- Price comparison (side by side) -->
                  <div class="price-comparison">
                    <div class="price-item current">
                      <span class="price-label">Current:</span>
                      <span class="price-value"
                        >{getCurrentPrice(row.slot)}</span
                      >
                    </div>
                    <div class="price-item next">
                      <span class="price-label">Next:</span>
                      <span
                        class="price-value"
                        class:na={getSelectedProductPrice(row.slot) === "NA"}
                        >{getSelectedProductPrice(row.slot)}</span
                      >
                    </div>
                  </div>

                  <!-- Product selector -->
                  <select
                    id="product-{row.slot}"
                    value={productSlots[row.slot as keyof typeof productSlots]}
                    onchange={(e) => {
                      const val = (e.target as HTMLSelectElement).value;
                      productSlots[row.slot as keyof typeof productSlots] = val;
                      // Don't save immediately - only update UI state
                    }}
                    disabled={saving}
                    class="product-select"
                  >
                    <option value="">Select product</option>
                    {#each products.filter((p) => !p.is_archived) as p}
                      <option value={p.id}
                        >{p.name} ({p.prices.length > 0
                          ? `€${(p.prices[0].price_amount / 100).toFixed(2)}`
                          : "No price"})</option
                      >
                    {/each}
                  </select>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Promotions & Discounts Setting -->
      <div class="setting-card">
        <div class="setting-info" style="margin-bottom: 1rem;">
          <h3>🏷️ Active Discount Codes</h3>
          <p class="setting-description">
            Assign which Polar discount applies to each plan/billing
            combination. Applied automatically at checkout when Founder Pricing
            is enabled.
          </p>
        </div>
        {#if discountsLoading}
          <p class="setting-description">Loading discounts from Polar…</p>
        {:else if discountsError}
          <p class="setting-description" style="color: #dc2626;">
            {discountsError}
          </p>
        {:else}
          <div class="discount-grid">
            {#each [{ key: "active_discount_pro_monthly", label: "Pro", interval: "Monthly", slot: "pro_monthly" as const, basePrice: 9 }, { key: "active_discount_pro_annual", label: "Pro", interval: "Annual", slot: "pro_annual" as const, basePrice: 90 }, { key: "active_discount_business_monthly", label: "Business", interval: "Monthly", slot: "business_monthly" as const, basePrice: 29 }, { key: "active_discount_business_annual", label: "Business", interval: "Annual", slot: "business_annual" as const, basePrice: 290 }] as row}
              <div class="discount-row">
                <div class="discount-label-section">
                  <label class="discount-label" for="discount-{row.slot}">
                    {row.label} — {row.interval}
                  </label>
                  <div class="discount-pricing">
                    <span class="base-price">€{row.basePrice}</span>
                    {#if discountSlots[row.slot]}
                      {@const selectedDiscount = discounts.find(
                        (d) => d.id === discountSlots[row.slot]
                      )}
                      {#if selectedDiscount}
                        {@const pricing = getPricingInfoForSlot(
                          row.slot,
                          selectedDiscount
                        )}
                        {#if pricing}
                          <span class="discounted-price"
                            >→ {pricing.discounted}</span
                          >
                        {/if}
                      {/if}
                    {/if}
                  </div>
                </div>
                <select
                  id="discount-{row.slot}"
                  value={discountSlots[row.slot as keyof typeof discountSlots]}
                  onchange={(e) => {
                    const val = (e.target as HTMLSelectElement).value;
                    discountSlots[row.slot as keyof typeof discountSlots] = val;
                    handleDiscountSlotChange(row.key, val);
                  }}
                  disabled={saving}
                  class="tier-select"
                >
                  <option value="">— None —</option>
                  {#each getDiscountsForSlot(row.slot) as d}
                    <option value={d.id}
                      >{formatDiscountLabel(d, row.slot)}</option
                    >
                  {/each}
                </select>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Founder Pricing Setting -->
      <div class="setting-card">
        <div class="setting-content">
          <div class="setting-info">
            <h3>🚀 Founder Pricing</h3>
            <p class="setting-description">
              Show founder pricing badges and apply active discount codes at
              checkout
            </p>
          </div>
          <div class="setting-control">
            <button
              onclick={handleFounderPricingToggle}
              disabled={saving}
              class="toggle-switch {founderPricingEnabled
                ? 'enabled'
                : 'disabled'}"
              role="switch"
              aria-checked={founderPricingEnabled}
              aria-label="Toggle founder pricing"
            >
              <span class="toggle-slider"></span>
            </button>
          </div>
        </div>
      </div>

      <!-- System Operations Section -->
      <div class="pricing-section">
        <h2 class="pricing-section-header">⚙️ System Operations</h2>
        <p class="pricing-section-description">
          Manual system maintenance operations
        </p>
      </div>

      <!-- System Operations Setting -->
      <div class="setting-card">
        <div class="setting-content">
          <div class="setting-info">
            <h3>Process Expired Subscriptions</h3>
            <p class="setting-description">
              Manually trigger the cron job that downgrades subscriptions with
              expired pending cancellations. This normally runs automatically on
              a schedule.
            </p>
          </div>
          <div class="setting-control">
            <button
              class="btn btn-primary"
              onclick={triggerCronJob}
              disabled={cronLoading}
            >
              {#if cronLoading}
                Processing...
              {:else}
                Process Now
              {/if}
            </button>
          </div>
        </div>

        {#if showCronResult && cronResult}
          <div
            class="cron-result {cronResult.errors > 0
              ? 'has-errors'
              : 'success'}"
          >
            <h4>Operation Results</h4>
            <div class="result-stats">
              <div class="stat">
                <span class="stat-value">{cronResult.processed}</span>
                <span class="stat-label">Processed</span>
              </div>
              <div class="stat">
                <span class="stat-value">{cronResult.success}</span>
                <span class="stat-label">Success</span>
              </div>
              <div class="stat">
                <span class="stat-value">{cronResult.errors}</span>
                <span class="stat-label">Errors</span>
              </div>
            </div>
            {#if cronResult.errors > 0}
              <p class="error-message">
                Some subscriptions failed to process. Check the logs for
                details.
              </p>
            {:else}
              <p class="success-message">
                All expired subscriptions were processed successfully.
              </p>
            {/if}
          </div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Signup Toggle Confirmation Modal -->
  {#if confirmingSignupToggle}
    <div
      class="modal-backdrop"
      role="button"
      tabindex="0"
      onclick={() => (confirmingSignupToggle = false)}
      onkeydown={(e) => e.key === "Enter" && (confirmingSignupToggle = false)}
    >
      <div
        class="modal"
        onclick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        tabindex="0"
        onkeydown={(e) =>
          e.key === "Escape" && (confirmingSignupToggle = false)}
      >
        <div class="modal-header">
          <h3>
            {signupsEnabled ? "Disable New Signups?" : "Enable New Signups?"}
          </h3>
          <button
            class="modal-close"
            onclick={() => (confirmingSignupToggle = false)}>&times;</button
          >
        </div>
        <div class="modal-body">
          <p>
            {#if signupsEnabled}
              Are you sure you want to <strong>disable new signups</strong>? New
              users will no longer be able to create accounts.
            {:else}
              Are you sure you want to <strong>enable new signups</strong>? New
              users will be able to create accounts.
            {/if}
          </p>
        </div>
        <div class="modal-footer">
          <button
            class="btn btn-secondary"
            onclick={() => (confirmingSignupToggle = false)}
            disabled={saving}
          >
            Cancel
          </button>
          <button
            class="btn btn-primary"
            onclick={confirmSignupToggle}
            disabled={saving}
          >
            {#if saving}
              Updating...
            {:else if signupsEnabled}
              Disable Signups
            {:else}
              Enable Signups
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Founder Pricing Toggle Confirmation Modal -->
  {#if confirmingFounderPricingToggle}
    <div
      class="modal-backdrop"
      role="button"
      tabindex="0"
      onclick={() => (confirmingFounderPricingToggle = false)}
      onkeydown={(e) =>
        e.key === "Enter" && (confirmingFounderPricingToggle = false)}
    >
      <div
        class="modal"
        onclick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        tabindex="0"
        onkeydown={(e) =>
          e.key === "Escape" && (confirmingFounderPricingToggle = false)}
      >
        <div class="modal-header">
          <h3>
            {founderPricingEnabled
              ? "Disable Founder Pricing?"
              : "Enable Founder Pricing?"}
          </h3>
          <button
            class="modal-close"
            onclick={() => (confirmingFounderPricingToggle = false)}
            >&times;</button
          >
        </div>
        <div class="modal-body">
          <p>
            {#if founderPricingEnabled}
              Are you sure you want to <strong>disable founder pricing</strong>?
              New users will no longer see the discounted founder prices.
            {:else}
              Are you sure you want to <strong>enable founder pricing</strong>?
              New users will see lifetime discounted prices (Pro: €5/mo,
              Business: €19/mo).
            {/if}
          </p>
        </div>
        <div class="modal-footer">
          <button
            class="btn btn-secondary"
            onclick={() => (confirmingFounderPricingToggle = false)}
            disabled={saving}
          >
            Cancel
          </button>
          <button
            class="btn btn-primary"
            onclick={confirmFounderPricingToggle}
            disabled={saving}
          >
            {#if saving}
              Updating...
            {:else if founderPricingEnabled}
              Disable Founder Pricing
            {:else}
              Enable Founder Pricing
            {/if}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Toast -->
  {#if showToast}
    <div class="toast">{toastMessage}</div>
  {/if}
</div>

<style>
  .settings-page {
    max-width: 800px;
    margin: 0 auto;
  }

  .page-header {
    margin-bottom: 2rem;
  }

  .page-header h1 {
    margin: 0 0 0.5rem 0;
    font-size: 1.75rem;
    font-weight: 600;
    color: #1e293b;
  }

  .subtitle {
    margin: 0;
    color: #64748b;
    font-size: 1rem;
  }

  .loading {
    text-align: center;
    padding: 3rem;
    color: #64748b;
  }

  .settings-container {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .setting-card {
    background: white;
    border-radius: 8px;
    padding: 1.5rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    border: 1px solid #e2e8f0;
  }

  .setting-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .setting-info {
    flex: 1;
  }

  .setting-info h3 {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
    font-weight: 600;
    color: #1e293b;
  }

  .setting-description {
    margin: 0;
    font-size: 0.875rem;
    color: #64748b;
    line-height: 1.4;
  }

  .setting-control {
    display: flex;
    align-items: center;
  }

  /* Toggle Switch */
  .toggle-switch {
    position: relative;
    display: inline-flex;
    width: 44px;
    height: 24px;
    flex-shrink: 0;
    cursor: pointer;
    border: none;
    border-radius: 12px;
    transition: background-color 0.2s;
    outline: none;
  }

  .toggle-switch:focus {
    box-shadow: 0 0 0 2px #3b82f6;
  }

  .toggle-switch.enabled {
    background-color: #f97316;
  }

  .toggle-switch.disabled {
    background-color: #e2e8f0;
  }

  .toggle-switch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .toggle-slider {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    background-color: white;
    border-radius: 50%;
    transition: transform 0.2s;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .toggle-switch.enabled .toggle-slider {
    transform: translateX(20px);
  }

  .toggle-switch.disabled .toggle-slider {
    transform: translateX(0);
  }

  /* Select Input */
  .tier-select {
    padding: 0.625rem 0.875rem;
    border: 2px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    font-family: inherit;
    background: white;
    width: 100%;
    max-width: 100%;
    transition:
      border-color 0.2s,
      box-shadow 0.2s;
  }

  .tier-select:hover {
    border-color: #9ca3af;
  }

  .tier-select:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px #3b82f6;
  }

  .tier-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Discount Grid */
  .discount-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(500px, 1fr));
    gap: 1.5rem;
    padding: 1rem;
    background: #f9fafb;
    border-radius: 8px;
    border: 1px solid #e5e7eb;
  }

  .discount-row {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1rem;
    background: white;
    border-radius: 6px;
    border: 1px solid #e5e7eb;
    box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  }

  .discount-label-section {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .discount-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: #111827;
    margin: 0;
  }

  .discount-pricing {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: #6b7280;
    background: #f3f4f6;
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    border: 1px solid #e5e7eb;
  }

  .base-price {
    font-weight: 500;
    color: #374151;
  }

  .discounted-price {
    font-weight: 600;
    color: #059669;
    background: #ecfdf5;
    padding: 0.125rem 0.5rem;
    border-radius: 3px;
    border: 1px solid #a7f3d0;
  }

  /* Product Configuration Styles */
  .product-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1rem;
    padding: 1rem;
    background: #f9fafb;
    border-radius: 8px;
    border: 1px solid #e5e7eb;
  }

  .product-row {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 1rem;
    background: white;
    border-radius: 6px;
    border: 1px solid #e5e7eb;
    box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  }

  .product-label {
    font-size: 0.875rem;
    font-weight: 600;
    color: #111827;
    margin: 0;
  }

  .product-select {
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: 2px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    font-family: inherit;
    background: white;
    transition:
      border-color 0.2s,
      box-shadow 0.2s;
  }

  .product-select:hover {
    border-color: #9ca3af;
  }

  .product-select:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px #3b82f6;
  }

  /* New visual indicator styles */
  .product-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .status-icon {
    font-size: 1rem;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    cursor: help;
  }

  .status-icon.saved {
    background: #ecfdf5;
    color: #059669;
  }

  .status-icon.unsaved {
    background: #fef3c7;
    color: #d97706;
  }

  .product-pricing-info {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .price-comparison {
    display: flex;
    gap: 1rem;
    align-items: center;
    padding: 0.5rem;
    background: #f9fafb;
    border-radius: 6px;
    border: 1px solid #e5e7eb;
  }

  .price-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    flex: 1;
  }

  .price-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .price-item.current .price-label {
    color: #059669;
  }

  .price-item.next .price-label {
    color: #3b82f6;
  }

  .price-value {
    font-size: 0.875rem;
    font-weight: 700;
  }

  .price-item.current .price-value {
    color: #047857;
  }

  .price-item.next .price-value {
    color: #1d4ed8;
  }

  .price-item.next .price-value.na {
    color: #9ca3af;
    font-style: italic;
  }

  .product-row.unsaved {
    border-color: #fcd34d;
    background: #fffbeb;
    box-shadow: 0 1px 3px 0 rgba(251, 191, 36, 0.1);
  }

  /* Pricing Section */
  .pricing-section {
    text-align: center;
    margin: 2rem 0 1rem 0;
  }

  .pricing-section-header {
    font-size: 1.5rem;
    font-weight: 700;
    color: #111827;
    margin: 0 0 0.5rem 0;
  }

  .pricing-section-description {
    font-size: 0.875rem;
    color: #6b7280;
    margin: 0;
    max-width: 600px;
    margin: 0 auto;
  }

  /* Sync Button */
  .sync-button-container {
    margin-top: 0.75rem;
  }

  .sync-button {
    padding: 0.5rem 1rem;
    background: #f3f4f6;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
    color: #374151;
    cursor: pointer;
    transition: all 0.2s;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }

  .sync-button:hover:not(:disabled) {
    background: #e5e7eb;
    border-color: #9ca3af;
  }

  .save-button {
    padding: 0.5rem 1rem;
    background: #10b981;
    color: white;
    border: 1px solid #059669;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }

  .save-button:hover:not(:disabled) {
    background: #059669;
    border-color: #047857;
  }

  .save-button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: white;
    border-radius: 8px;
    width: 90%;
    max-width: 400px;
    max-height: 90vh;
    overflow-y: auto;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    border-bottom: 1px solid #e2e8f0;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: #1e293b;
  }

  .modal-close {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: #64748b;
    padding: 0;
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal-body {
    padding: 1.5rem;
  }

  .modal-body p {
    margin: 0;
    color: #475569;
    line-height: 1.5;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1.5rem;
    border-top: 1px solid #e2e8f0;
  }

  .btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary {
    background: #3b82f6;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .btn-secondary {
    background: #64748b;
    color: white;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #475569;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* System Operations */

  .cron-result {
    margin-top: 1rem;
    padding: 1rem;
    border-radius: 6px;
    border-left: 4px solid;
  }

  .cron-result.success {
    background: #f0fdf4;
    border-left-color: #22c55e;
    color: #166534;
  }

  .cron-result.has-errors {
    background: #fef2f2;
    border-left-color: #ef4444;
    color: #991b1b;
  }

  .result-stats {
    display: flex;
    gap: 2rem;
    margin: 1rem 0;
  }

  .stat {
    text-align: center;
  }

  .stat-value {
    display: block;
    font-size: 1.5rem;
    font-weight: 600;
    color: #1e293b;
  }

  .stat-label {
    font-size: 0.8rem;
    color: #64748b;
    text-transform: uppercase;
  }

  .error-message {
    margin: 0.5rem 0 0 0;
    color: #991b1b;
  }

  .success-message {
    margin: 0.5rem 0 0 0;
    color: #166534;
  }

  /* Toast */
  .toast {
    position: fixed;
    bottom: 2rem;
    right: 2rem;
    background: #1e293b;
    color: white;
    padding: 1rem 1.5rem;
    border-radius: 6px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    z-index: 1001;
    animation: slideIn 0.3s ease;
  }

  @keyframes slideIn {
    from {
      transform: translateY(100%);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  /* Responsive */
  @media (max-width: 640px) {
    .setting-content {
      flex-direction: column;
      align-items: flex-start;
      gap: 1rem;
    }

    .setting-control {
      align-self: flex-end;
    }
  }
</style>
