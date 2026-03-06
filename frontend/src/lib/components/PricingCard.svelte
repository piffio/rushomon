<script lang="ts">
	import { createEventDispatcher } from "svelte";

	export let tier: string;
	export let title: string;
	export let description: string;
	export let price: number;
	export let interval: string;
	export let features: string[];
	export let buttonText: string;
	export let buttonHref: string | undefined = undefined;
	export let isPopular = false;
	export let founderPrice: number | undefined = undefined;
	export let originalPrice: number | undefined = undefined;
	export let founderPricingActive = false;
	export let checkoutLoading: string | null = null;
	export let billingInterval = "monthly";
	export let disabled = false;
	export let isCurrentPlan = false;
	export let isUpgrade = false;
	export let isDowngrade = false;
	export let usePortalForUpgrade = false;

	const dispatch = createEventDispatcher();

	function handleAction() {
		// Don't do anything if button is disabled
		if (disabled) return;

		if (buttonHref) {
			window.location.href = buttonHref;
		} else if (isCurrentPlan) {
			dispatch("portal", { tier });
		} else if (isUpgrade && usePortalForUpgrade) {
			dispatch("portal", { tier });
		} else {
			dispatch("checkout", { tier });
		}
	}
</script>

<div
	class="border rounded-2xl p-8 relative transition-all duration-700 h-full pricing-grid {isCurrentPlan
		? 'border-2 border-green-500 bg-green-50'
		: isPopular
			? 'border-2 border-orange-500'
			: isDowngrade
				? 'border-gray-300 bg-gray-50 opacity-75'
				: 'border-gray-200'}"
	class:shadow-lg={isPopular || isCurrentPlan}
>
	{#if isCurrentPlan}
		<div class="absolute -top-4 left-1/2 transform -translate-x-1/2">
			<span
				class="bg-green-500 text-white px-4 py-1 rounded-full text-sm font-semibold flex items-center gap-1"
			>
				<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
					<path
						fill-rule="evenodd"
						d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
						clip-rule="evenodd"
					/>
				</svg>
				Your Plan
			</span>
		</div>
	{:else if isUpgrade}
		<div class="absolute -top-4 left-1/2 transform -translate-x-1/2">
			<span
				class="bg-blue-500 text-white px-4 py-1 rounded-full text-sm font-semibold flex items-center gap-1"
			>
				<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
					<path
						fill-rule="evenodd"
						d="M3.293 9.707a1 1 0 010-1.414l6-6a1 1 0 011.414 0l6 6a1 1 0 01-1.414 1.414L11 5.414V17a1 1 0 11-2 0V5.414L4.707 9.707a1 1 0 01-1.414 0z"
						clip-rule="evenodd"
					/>
				</svg>
				Upgrade
			</span>
		</div>
	{:else if isPopular}
		<div class="absolute -top-4 left-1/2 transform -translate-x-1/2">
			<span
				class="bg-orange-500 text-white px-4 py-1 rounded-full text-sm font-semibold"
			>
				Most Popular
			</span>
		</div>
	{/if}

	<!-- Header -->
	<div class="pricing-header">
		<h3 class="text-2xl font-bold text-gray-900">{title}</h3>
		<p class="text-gray-600">{description}</p>
	</div>

	<!-- Price -->
	<div class="pricing-price">
		{#if tier !== "free" && founderPricingActive && founderPrice}
			{#if billingInterval === "monthly"}
				<div class="flex items-baseline justify-start gap-1">
					<span class="text-5xl font-bold text-orange-600"
						>€{founderPrice}</span
					>
					<span class="text-gray-600 text-lg">/month</span>
					<span
						class="text-xs font-semibold text-orange-600 bg-orange-50 px-2 py-1 rounded-full ml-2"
						>🎉 Founder</span
					>
				</div>
				{#if originalPrice}
					<div class="text-sm text-gray-500 mt-1 line-through">
						Was €{originalPrice}/month
					</div>
				{/if}
			{:else}
				<div class="flex items-baseline justify-start gap-1">
					<span class="text-5xl font-bold text-orange-600"
						>€{founderPrice}</span
					>
					<span class="text-gray-600 text-lg">/year</span>
					<span
						class="text-xs font-semibold text-orange-600 bg-orange-50 px-2 py-1 rounded-full ml-2"
						>🎉 Founder</span
					>
				</div>
				{#if originalPrice}
					<div class="text-sm text-gray-500 mt-1 line-through">
						Was €{originalPrice}/year
					</div>
				{/if}
			{/if}
		{:else}
			<!-- Placeholder for alignment when no founder pricing -->
			<div class="mb-3 h-6" class:hidden={founderPricingActive}></div>
			{#if disabled}
				<!-- Paid tier not configured -->
				<div class="flex items-baseline justify-start gap-1">
					<span class="text-5xl font-bold text-gray-400">—</span>
					<span class="text-gray-400 text-lg">/{interval}</span>
				</div>
				<div class="text-sm text-gray-400 mt-1">Coming soon</div>
			{:else}
				<!-- Free tier or configured paid tier -->
				<div class="flex items-baseline justify-start gap-1">
					<span class="text-5xl font-bold text-gray-900"
						>{tier === "free" ? "0" : price}</span
					>
					<span class="text-gray-600 text-lg"
						>/{tier === "free" ? "month" : interval}</span
					>
				</div>
			{/if}
		{/if}
	</div>

	<!-- Features -->
	<div class="pricing-features">
		<ul class="space-y-3">
			{#each features as feature}
				<li class="flex items-start">
					<svg
						class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
						fill="currentColor"
						viewBox="0 0 20 20"
					>
						<path
							fill-rule="evenodd"
							d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
							clip-rule="evenodd"
						/>
					</svg>
					<span class="text-gray-700">{feature}</span>
				</li>
			{/each}
		</ul>
	</div>

	<!-- Action -->
	<div class="pricing-action">
		<button
			onclick={handleAction}
			disabled={checkoutLoading === `${tier}_${billingInterval}` ||
				checkoutLoading === "portal" ||
				disabled}
			class="w-full px-6 py-3 rounded-lg font-semibold transition-all shadow-sm text-center {disabled
				? 'bg-gray-200 text-gray-500 cursor-not-allowed'
				: isCurrentPlan
					? 'bg-gradient-to-r from-green-500 to-green-600 text-white hover:from-green-600 hover:to-green-700 hover:shadow-md'
					: isUpgrade
						? 'bg-gradient-to-r from-blue-500 to-blue-600 text-white hover:from-blue-600 hover:to-blue-700 hover:shadow-md'
						: 'bg-gradient-to-r from-orange-500 to-orange-600 text-white hover:from-orange-600 hover:to-orange-700 hover:shadow-md'}"
		>
			{checkoutLoading === `${tier}_${billingInterval}` ||
			checkoutLoading === "portal"
				? isCurrentPlan
					? "Opening Portal…"
					: "Redirecting…"
				: buttonText}
		</button>
	</div>
</div>

<style>
	.pricing-grid {
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	.pricing-header {
		height: 6.5rem;
		overflow: visible;
		margin-bottom: 1.5rem;
	}
	.pricing-price {
		height: 8rem;
		margin-bottom: 1.5rem;
	}
	.pricing-features {
		flex-grow: 1;
		margin-bottom: 1.5rem;
	}
	.pricing-action {
		margin-top: auto;
	}
</style>
