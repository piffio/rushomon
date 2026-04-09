<script lang="ts">
  import { authApi } from "$lib/api/auth";
  import type { BillingStatus, ProductPrice } from "$lib/api/billing";
  import { billingApi } from "$lib/api/billing";
  import { apiClient } from "$lib/api/client";
  import Footer from "$lib/components/Footer.svelte";
  import Header from "$lib/components/Header.svelte";
  import PricingCard from "$lib/components/PricingCard.svelte";
  import SEO from "$lib/components/SEO.svelte";
  import type { User } from "$lib/types/api";
  import { onMount } from "svelte";
  import { createPricingTiers } from "../../config/pricing";
  import type { PageData } from "./$types";

  const { data: _data }: { data: PageData } = $props();

  const loginUrl = "/login";

  let mounted = $state(false);
  let currentUser = $state<User | undefined>(undefined);
  let billingInterval = $state<"monthly" | "annual">("monthly");
  let checkoutLoading = $state<string | null>(null);
  let checkoutError = $state<string | null>(null);

  // Dynamic pricing from API
  let products = $state<ProductPrice[]>([]);

  // Billing status for logged-in users
  let billingStatus = $state<BillingStatus | null>(null);

  // Helper function to get price by plan and interval
  function getPrice(
    plan: "pro" | "business",
    interval: "monthly" | "annual"
  ): number {
    const product = products.find(
      (p) =>
        p.polar_product_id ===
          (plan === "pro" ? "product_pro" : "product_business") &&
        p.recurring_interval === interval
    );
    return product?.price_amount || 0;
  }

  // Fallback prices for when database is empty or API fails
  const FALLBACK_PRICES = {
    pro_monthly: 9,
    pro_annual: 90,
    business_monthly: 29,
    business_annual: 290
  };

  // Helper function to get price with fallback
  function getPriceWithFallback(
    plan: "pro" | "business",
    interval: "monthly" | "annual"
  ): number {
    const dbPrice = getPrice(plan, interval);
    if (dbPrice > 0) return dbPrice;

    // Return fallback price in cents
    const key = `${plan}_${interval}` as keyof typeof FALLBACK_PRICES;
    return FALLBACK_PRICES[key] * 100;
  }

  // Helper function to get display price with fallback
  function getDisplayPriceWithFallback(
    plan: "pro" | "business",
    interval: "monthly" | "annual"
  ): number {
    return Math.round(getPriceWithFallback(plan, interval) / 100);
  }

  // Founder pricing state
  let founderPricingActive = $state(false);
  const discountAmounts = $state({
    pro_monthly: 0,
    pro_annual: 0,
    business_monthly: 0,
    business_annual: 0
  });

  // Derived: effective founder price for a slot (base - discount), in whole euros
  function founderPrice(
    plan: "pro" | "business",
    interval: "monthly" | "annual"
  ): number {
    const base = getPriceWithFallback(plan, interval);
    const disc =
      discountAmounts[`${plan}_${interval}` as keyof typeof discountAmounts] ||
      0;
    return Math.round((base - disc) / 100);
  }

  // Initialize settings
  onMount(async () => {
    // Fetch pricing from API
    try {
      const pricing = await billingApi.getPricing();
      products = pricing.products;
    } catch (error) {
      console.error("Failed to fetch pricing:", error);
    }

    // Fetch settings on client side using apiClient
    try {
      const settings = await apiClient.get<{
        founder_pricing_active: boolean;
        active_discount_amount_pro_monthly: number;
        active_discount_amount_pro_annual: number;
        active_discount_amount_business_monthly: number;
        active_discount_amount_business_annual: number;
      }>("/api/settings");
      founderPricingActive = settings.founder_pricing_active || false;
      discountAmounts.pro_monthly =
        settings.active_discount_amount_pro_monthly || 0;
      discountAmounts.pro_annual =
        settings.active_discount_amount_pro_annual || 0;
      discountAmounts.business_monthly =
        settings.active_discount_amount_business_monthly || 0;
      discountAmounts.business_annual =
        settings.active_discount_amount_business_annual || 0;
    } catch (error) {
      console.warn("Failed to fetch settings:", error);
    }

    mounted = true;
    authApi
      .me()
      .then(async (user) => {
        currentUser = user;

        // Fetch billing status for logged-in users
        if (user) {
          try {
            billingStatus = await billingApi.getStatus();
          } catch (error) {
            console.warn("Failed to fetch billing status:", error);
          }
        }
      })
      .catch(() => {
        currentUser = undefined;
      });
  });

  async function startCheckout(plan: "pro" | "business") {
    if (!currentUser) {
      window.location.href = loginUrl;
      return;
    }
    const interval = billingInterval;
    const planKey = `${plan}_${interval}`; // e.g., "pro_monthly"

    checkoutLoading = planKey;
    checkoutError = null;
    try {
      const { url } = await billingApi.createCheckout(planKey);
      window.location.href = url;
    } catch (e: unknown) {
      const msg =
        e && typeof e === "object" && "message" in e
          ? String((e as { message: unknown }).message)
          : "Failed to start checkout. Please try again.";
      checkoutError = msg;
      checkoutLoading = null;
    }
  }

  async function openPortal() {
    if (!currentUser) return;

    checkoutLoading = "portal";
    checkoutError = null;
    try {
      const { url } = await billingApi.createPortal();
      window.location.href = url;
    } catch (e: unknown) {
      const msg =
        e && typeof e === "object" && "message" in e
          ? String((e as { message: unknown }).message)
          : "Failed to open billing portal. Please try again.";
      checkoutError = msg;
      checkoutLoading = null;
    }
  }

  // Compute pricing tiers reactively
  const pricingTiers = $derived(
    createPricingTiers(
      (tier: string, interval: string) =>
        getDisplayPriceWithFallback(
          tier as "pro" | "business",
          interval as "monthly" | "annual"
        ),
      (tier: string, interval: string) =>
        founderPrice(
          tier as "pro" | "business",
          interval as "monthly" | "annual"
        ),
      billingInterval,
      currentUser,
      loginUrl,
      products,
      billingStatus
    )
  );

  // Helper function to determine tier hierarchy
  function getTierHierarchy(tier: string): number {
    switch (tier) {
      case "free":
        return 0;
      case "pro":
        return 1;
      case "business":
        return 2;
      default:
        return 0;
    }
  }

  // Handle checkout events from PricingCard components
  function handleCheckout(event: CustomEvent<{ tier: string }>) {
    startCheckout(event.detail.tier as "pro" | "business");
  }
</script>

<svelte:head>
  <SEO
    title="Pricing – Rushomon"
    description="Simple, transparent pricing for Rushomon URL shortener. Free forever for personal use, with paid plans for creators and teams."
  />
</svelte:head>

<div class="min-h-screen bg-white flex flex-col">
  <Header user={currentUser} currentPage="landing" />

  <main class="flex-1">
    <section class="container mx-auto px-4 py-20 md:py-32">
      <div class="max-w-5xl mx-auto">
        <!-- Header -->
        <div
          class="text-center mb-16 transition-all duration-700 {mounted
            ? 'opacity-100 translate-y-0'
            : 'opacity-0 translate-y-4'}"
        >
          <!-- Billing interval toggle -->
          <div class="flex items-center justify-center gap-3 mb-8">
            <span
              class="text-sm font-medium {billingInterval === 'monthly'
                ? 'text-gray-900'
                : 'text-gray-400'}">Monthly</span
            >
            <button
              onclick={() =>
                (billingInterval =
                  billingInterval === "monthly" ? "annual" : "monthly")}
              class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {billingInterval ===
              'annual'
                ? 'bg-orange-500'
                : 'bg-gray-200'}"
              aria-label="Toggle billing interval"
            >
              <span
                class="inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform {billingInterval ===
                'annual'
                  ? 'translate-x-6'
                  : 'translate-x-1'}"
              ></span>
            </button>
            <span
              class="text-sm font-medium {billingInterval === 'annual'
                ? 'text-gray-900'
                : 'text-gray-400'}"
            >
              Annual <span class="text-green-600 font-semibold"
                >(2 months free)</span
              >
            </span>
          </div>
          {#if currentUser && billingStatus}
            <div class="mb-8 p-6 bg-blue-50 border border-blue-200 rounded-xl">
              <p class="text-lg text-blue-900 font-medium mb-2">
                Hi {currentUser.name || currentUser.email.split("@")[0]}, you're
                on the
                <span class="font-bold text-blue-700">{billingStatus.tier}</span
                > plan
              </p>
              {#if billingStatus.subscription_status === "active" && billingStatus.current_period_end}
                <p class="text-blue-700">
                  {billingStatus.cancel_at_period_end
                    ? "Expires on"
                    : "Renews on"}
                  {new Date(
                    billingStatus.current_period_end * 1000
                  ).toLocaleDateString()}
                </p>
              {/if}
            </div>
          {/if}

          {#if currentUser && billingStatus && billingStatus.amount_cents}
            <div class="mb-8 p-4 bg-gray-50 border border-gray-200 rounded-lg">
              <h3 class="font-semibold text-gray-900 mb-2">
                Current Plan Summary
              </h3>
              <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
                <div>
                  <span class="text-gray-600">Plan:</span>
                  <span class="font-medium text-gray-900 ml-2"
                    >{billingStatus.tier}</span
                  >
                </div>
                <div>
                  <span class="text-gray-600">Billing:</span>
                  <span class="font-medium text-gray-900 ml-2">
                    €{billingStatus.amount_cents /
                      100}/{billingStatus.interval || "month"}
                  </span>
                </div>
                <div>
                  <span class="text-gray-600">Status:</span>
                  <span class="font-medium text-gray-900 ml-2 capitalize"
                    >{billingStatus.subscription_status || "Active"}</span
                  >
                </div>
              </div>
            </div>
          {/if}

          <h2 class="text-4xl md:text-5xl font-bold text-gray-900 mb-6">
            Simple, Transparent Pricing
          </h2>
          <p class="text-xl text-gray-600 max-w-2xl mx-auto">
            Start free, upgrade when you need more. Rushomon is 100% open source
            — self-host it yourself or let us handle the infrastructure.
          </p>
        </div>

        <!-- Pricing Cards -->
        <div class="grid md:grid-cols-3 gap-8 mb-16 items-stretch">
          {#each pricingTiers as tierConfig (tierConfig.tier)}
            {@const resolvedButtonText =
              typeof tierConfig.buttonText === "function"
                ? tierConfig.buttonText()
                : tierConfig.buttonText}
            {@const resolvedButtonHref =
              typeof tierConfig.buttonHref === "function"
                ? tierConfig.buttonHref()
                : tierConfig.buttonHref}
            {@const resolvedDisabled =
              typeof tierConfig.disabled === "function"
                ? tierConfig.disabled()
                : (tierConfig.disabled ?? false)}
            {@const isCurrentPlan =
              (billingStatus &&
                billingStatus.tier?.toLowerCase() ===
                  tierConfig.tier.toLowerCase()) ||
              undefined}
            {@const isUpgrade =
              (billingStatus &&
                getTierHierarchy(billingStatus.tier.toLowerCase()) <
                  getTierHierarchy(tierConfig.tier.toLowerCase())) ||
              undefined}
            {@const isDowngrade =
              (billingStatus &&
                getTierHierarchy(billingStatus.tier.toLowerCase()) >
                  getTierHierarchy(tierConfig.tier.toLowerCase())) ||
              undefined}
            {@const usePortalForUpgrade =
              (billingStatus &&
                billingStatus.tier?.toLowerCase() !== "free" &&
                getTierHierarchy(billingStatus.tier.toLowerCase()) <
                  getTierHierarchy(tierConfig.tier.toLowerCase())) ||
              undefined}
            <PricingCard
              tier={tierConfig.tier}
              title={tierConfig.title}
              description={tierConfig.description}
              price={typeof tierConfig.price === "function"
                ? tierConfig.price()
                : tierConfig.price}
              interval={tierConfig.interval}
              features={tierConfig.features}
              buttonText={resolvedButtonText}
              buttonHref={resolvedButtonHref}
              disabled={resolvedDisabled}
              isPopular={tierConfig.isPopular}
              founderPrice={tierConfig.founderPrice?.()}
              originalPrice={tierConfig.originalPrice?.()}
              {founderPricingActive}
              {checkoutLoading}
              {billingInterval}
              {isCurrentPlan}
              {isUpgrade}
              {isDowngrade}
              {usePortalForUpgrade}
              on:checkout={handleCheckout}
              on:portal={() => openPortal()}
            />
          {/each}
        </div>
        <!-- Pricing Cards grid ends -->

        {#if checkoutError}
          <div
            class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm text-center"
          >
            {checkoutError}
          </div>
        {/if}
      </div>
      <!-- max-w-5xl container ends -->

      <!-- FAQ / Notes -->
      <div class="text-center max-w-3xl mx-auto px-4">
        <div
          class="bg-orange-50 border border-orange-200 rounded-lg p-6 text-center"
        >
          <h3 class="text-lg font-semibold text-gray-900 mb-3">
            🎉 Founder Pricing — Limited Spots!
          </h3>
          <p class="text-gray-700 mb-4">
            First 100 paying users get a lifetime discount automatically applied
            at checkout. No code needed.
          </p>

          <!-- Founder Pricing Info -->
          <div class="bg-white rounded-lg p-4 mb-4 border border-orange-100">
            <h4 class="font-semibold text-orange-600 mb-2">
              🎉 Founder Pricing
            </h4>
            <p class="text-sm text-gray-600 mb-2">
              First 100 paying users get lifetime discounts:
            </p>
            <div class="flex flex-col sm:flex-row gap-2 justify-center text-sm">
              <span class="font-medium text-gray-900"
                >Pro: <span class="line-through text-gray-500">€9</span> €5/mo</span
              >
              <span class="text-gray-400">·</span>
              <span class="font-medium text-gray-900"
                >Business: <span class="line-through text-gray-500">€29</span> €19/mo</span
              >
            </div>
          </div>

          <div class="flex flex-col sm:flex-row gap-4 justify-center">
            <a
              href={loginUrl}
              class="px-6 py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm hover:shadow-md text-center"
            >
              Start Free Now
            </a>
            <a
              href="/"
              class="px-6 py-3 border border-gray-300 text-gray-700 rounded-lg font-semibold hover:bg-gray-50 transition-colors text-center"
            >
              Back to Home
            </a>
          </div>
        </div>
      </div>
    </section>
  </main>

  <Footer />
</div>
