<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import Footer from "$lib/components/Footer.svelte";
	import PricingCard from "$lib/components/PricingCard.svelte";
	import { authApi } from "$lib/api/auth";
	import { billingApi } from "$lib/api/billing";
	import { apiClient } from "$lib/api/client";
	import { onMount } from "svelte";
	import type { PageData } from "./$types";
	import type { User } from "$lib/types/api";
	import type { ProductPrice } from "$lib/api/billing";
	import { createPricingTiers, type PricingTier } from "../../config/pricing";

	let { data }: { data: PageData } = $props();

	const loginUrl = "/login";

	let mounted = $state(false);
	let currentUser = $state<User | undefined>(undefined);
	let billingInterval = $state<"monthly" | "annual">("monthly");
	let checkoutLoading = $state<string | null>(null);
	let checkoutError = $state<string | null>(null);

	// Dynamic pricing from API
	let products = $state<ProductPrice[]>([]);
	let pricingLoading = $state(true);
	let pricingError = $state<string | null>(null);

	// Helper function to get price by plan and interval
	function getPrice(
		plan: "pro" | "business",
		interval: "monthly" | "annual",
	): number {
		const settingKey = `product_${plan}_${interval}_id`;
		const product = products.find((p) => p.id === settingKey);
		return product?.price_amount || 0;
	}

	// Helper function to get display price in euros
	function getDisplayPrice(
		plan: "pro" | "business",
		interval: "monthly" | "annual",
	): number {
		return Math.round(getPrice(plan, interval) / 100);
	}

	// Fallback prices for when database is empty or API fails
	const FALLBACK_PRICES = {
		pro_monthly: 9,
		pro_annual: 90,
		business_monthly: 29,
		business_annual: 290,
	};

	// Helper function to get price with fallback
	function getPriceWithFallback(
		plan: "pro" | "business",
		interval: "monthly" | "annual",
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
		interval: "monthly" | "annual",
	): number {
		return Math.round(getPriceWithFallback(plan, interval) / 100);
	}

	// Founder pricing state
	let founderPricingActive = $state(false);
	let discountIds = $state({
		pro_monthly: "",
		pro_annual: "",
		business_monthly: "",
		business_annual: "",
	});
	let discountAmounts = $state({
		pro_monthly: 0,
		pro_annual: 0,
		business_monthly: 0,
		business_annual: 0,
	});

	// Derived: effective founder price for a slot (base - discount), in whole euros
	function founderPrice(
		plan: "pro" | "business",
		interval: "monthly" | "annual",
	): number {
		const base = getPriceWithFallback(plan, interval);
		const disc =
			discountAmounts[
				`${plan}_${interval}` as keyof typeof discountAmounts
			] || 0;
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
			pricingError = "Failed to load pricing information";
		} finally {
			pricingLoading = false;
		}

		// Fetch settings on client side using apiClient
		try {
			const settings = await apiClient.get<{
				founder_pricing_active: boolean;
				active_discount_pro_monthly: string;
				active_discount_pro_annual: string;
				active_discount_business_monthly: string;
				active_discount_business_annual: string;
				active_discount_amount_pro_monthly: number;
				active_discount_amount_pro_annual: number;
				active_discount_amount_business_monthly: number;
				active_discount_amount_business_annual: number;
			}>("/api/settings");
			founderPricingActive = settings.founder_pricing_active || false;
			discountIds.pro_monthly =
				settings.active_discount_pro_monthly || "";
			discountIds.pro_annual = settings.active_discount_pro_annual || "";
			discountIds.business_monthly =
				settings.active_discount_business_monthly || "";
			discountIds.business_annual =
				settings.active_discount_business_annual || "";
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
			.then((user) => {
				currentUser = user;
			})
			.catch(() => {
				currentUser = undefined;
			});
	});

	async function startCheckout(priceEnvKey: "pro" | "business") {
		if (!currentUser) {
			window.location.href = loginUrl;
			return;
		}
		const interval = billingInterval;
		const settingKey = `product_${priceEnvKey}_${interval}_id`;

		// Find the product from pricing data
		const product = products.find((p) => p.id === settingKey);
		if (!product) {
			checkoutError = "Product not found. Please contact support.";
			return;
		}

		// Determine coupon ID based on the active discount for this slot
		const key = `${priceEnvKey}_${interval}`;
		const couponId = founderPricingActive
			? discountIds[key as keyof typeof discountIds] || undefined
			: undefined;

		checkoutLoading = key;
		checkoutError = null;
		try {
			const { url } = await billingApi.createCheckout(
				product.polar_product_id,
				interval,
				couponId,
			);
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

	// Compute pricing tiers reactively
	let pricingTiers = $derived(
		createPricingTiers(
			(tier: string, interval: string) =>
				getDisplayPriceWithFallback(
					tier as "pro" | "business",
					interval as "monthly" | "annual",
				),
			(tier: string, interval: string) =>
				founderPrice(
					tier as "pro" | "business",
					interval as "monthly" | "annual",
				),
			billingInterval,
			currentUser,
			loginUrl,
		),
	);

	// Handle checkout events from PricingCard components
	function handleCheckout(event: CustomEvent<{ tier: string }>) {
		startCheckout(event.detail.tier as "pro" | "business");
	}
</script>

<svelte:head>
	<title>Pricing - Rushomon</title>
	<meta
		name="description"
		content="Simple, transparent pricing for Rushomon URL shortener. Free forever for personal use, with paid plans for creators and teams."
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
							class="text-sm font-medium {billingInterval ===
							'monthly'
								? 'text-gray-900'
								: 'text-gray-400'}">Monthly</span
						>
						<button
							onclick={() =>
								(billingInterval =
									billingInterval === "monthly"
										? "annual"
										: "monthly")}
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
							class="text-sm font-medium {billingInterval ===
							'annual'
								? 'text-gray-900'
								: 'text-gray-400'}"
						>
							Annual <span class="text-green-600 font-semibold"
								>(2 months free)</span
							>
						</span>
					</div>
					<h2
						class="text-4xl md:text-5xl font-bold text-gray-900 mb-6"
					>
						Simple, Transparent Pricing
					</h2>
					<p class="text-xl text-gray-600 max-w-2xl mx-auto">
						Start free, upgrade when you need more. Rushomon is 100%
						open source — self-host it yourself or let us handle the
						infrastructure.
					</p>
				</div>

				<!-- Pricing Cards -->
				<div class="grid md:grid-cols-3 gap-8 mb-16 items-stretch">
					{#each pricingTiers as tierConfig (tierConfig.tier)}
						<PricingCard
							tier={tierConfig.tier}
							title={tierConfig.title}
							description={tierConfig.description}
							price={typeof tierConfig.price === "function"
								? tierConfig.price()
								: tierConfig.price}
							interval={tierConfig.interval}
							features={tierConfig.features}
							buttonText={tierConfig.buttonText}
							buttonHref={tierConfig.buttonHref}
							isPopular={tierConfig.isPopular}
							founderPrice={tierConfig.founderPrice?.()}
							originalPrice={tierConfig.originalPrice?.()}
							{founderPricingActive}
							{checkoutLoading}
							{billingInterval}
							on:checkout={handleCheckout}
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
			<!-- max-w-5tl container ends -->

			<!-- FAQ / Notes -->
			<div class="text-center max-w-3xl mx-auto px-4">
				<div
					class="bg-orange-50 border border-orange-200 rounded-lg p-6 text-center"
				>
					<h3 class="text-lg font-semibold text-gray-900 mb-3">
						🎉 Founder Pricing — Limited Spots!
					</h3>
					<p class="text-gray-700 mb-4">
						First 100 paying users get a lifetime discount
						automatically applied at checkout. No code needed.
					</p>

					<!-- Founder Pricing Info -->
					<div
						class="bg-white rounded-lg p-4 mb-4 border border-orange-100"
					>
						<h4 class="font-semibold text-orange-600 mb-2">
							🎉 Founder Pricing
						</h4>
						<p class="text-sm text-gray-600 mb-2">
							First 100 paying users get lifetime discounts:
						</p>
						<div
							class="flex flex-col sm:flex-row gap-2 justify-center text-sm"
						>
							<span class="font-medium text-gray-900"
								>Pro: <span class="line-through text-gray-500"
									>$9</span
								> $5/mo</span
							>
							<span class="text-gray-400">·</span>
							<span class="font-medium text-gray-900"
								>Business: <span
									class="line-through text-gray-500">$29</span
								> $19/mo</span
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
