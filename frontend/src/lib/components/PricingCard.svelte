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

	const dispatch = createEventDispatcher();

	function handleAction() {
		if (buttonHref) {
			window.location.href = buttonHref;
		} else {
			dispatch("checkout", { tier });
		}
	}
</script>

<div
	class="border {isPopular
		? 'border-2 border-orange-500'
		: 'border-gray-200'} rounded-2xl p-8 relative transition-all duration-700 h-full pricing-grid"
	class:shadow-lg={isPopular}
>
	{#if isPopular}
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
		{#if founderPricingActive && founderPrice}
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
			<div class="flex items-baseline justify-start gap-1">
				<span class="text-5xl font-bold text-gray-900">€{price}</span>
				<span class="text-gray-600 text-lg">/{interval}</span>
			</div>
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
			disabled={checkoutLoading === `${tier}_${billingInterval}`}
			class="w-full px-6 py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm hover:shadow-md disabled:opacity-60 disabled:cursor-not-allowed text-center"
		>
			{checkoutLoading === `${tier}_${billingInterval}`
				? "Redirecting…"
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
