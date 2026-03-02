<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import Footer from "$lib/components/Footer.svelte";
	import { authApi } from "$lib/api/auth";
	import { billingApi } from "$lib/api/billing";
	import { onMount } from "svelte";
	import type { PageData } from "./$types";
	import type { User } from "$lib/types/api";

	let { data }: { data: PageData } = $props();

	const loginUrl = authApi.getLoginUrl();

	let mounted = $state(false);
	let currentUser = $state<User | undefined>(undefined);
	let billingInterval = $state<"monthly" | "annual">("monthly");
	let checkoutLoading = $state<string | null>(null);
	let checkoutError = $state<string | null>(null);

	async function startCheckout(priceEnvKey: "pro" | "business") {
		if (!currentUser) {
			window.location.href = loginUrl;
			return;
		}
		const interval = billingInterval;
		const key = `${priceEnvKey}_${interval}`; // Send human-readable key

		checkoutLoading = key;
		checkoutError = null;
		try {
			const { url } = await billingApi.createCheckout(key, interval);
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

	onMount(() => {
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
				<div class="grid md:grid-cols-3 gap-8 mb-16">
					<!-- Free Tier -->
					<div
						class="border border-gray-200 rounded-2xl p-8 transition-all duration-700 {mounted
							? 'opacity-100 translate-y-0'
							: 'opacity-0 translate-y-4'}"
						style="transition-delay: 100ms"
					>
						<div class="mb-8">
							<h3 class="text-2xl font-bold text-gray-900 mb-2">
								Free
							</h3>
							<p class="text-gray-600">
								For personal projects and trying out Rushomon
							</p>
						</div>

						<div class="mb-8">
							<span class="text-5xl font-bold text-gray-900"
								>$0</span
							>
							<span class="text-gray-600">/month</span>
						</div>

						<ul class="space-y-3 mb-8">
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
								<span class="text-gray-700"
									>15 links per month</span
								>
							</li>
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
								<span class="text-gray-700"
									>7-day analytics retention</span
								>
							</li>
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
								<span class="text-gray-700"
									>rush.mn short links</span
								>
							</li>
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
								<span class="text-gray-700"
									>QR code generation</span
								>
							</li>
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
								<span class="text-gray-700"
									>Community support (GitHub)</span
								>
							</li>
						</ul>

						<a
							href={loginUrl}
							class="w-full block px-6 py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm hover:shadow-md text-center"
						>
							Get Started Free
						</a>
					</div>

					<!-- Pro Tier -->
					<div
						class="border-2 border-orange-500 rounded-2xl p-8 relative transition-all duration-700 {mounted
							? 'opacity-100 translate-y-0'
							: 'opacity-0 translate-y-4'}"
						style="transition-delay: 200ms"
					>
						<div
							class="absolute -top-4 left-1/2 transform -translate-x-1/2"
						>
							<span
								class="bg-orange-500 text-white px-4 py-1 rounded-full text-sm font-semibold"
							>
								Most Popular
							</span>
						</div>

						<div class="mb-8">
							<h3 class="text-2xl font-bold text-gray-900 mb-2">
								Pro
							</h3>
							<p class="text-gray-600">
								For creators who need custom codes and extended
								analytics history
							</p>
						</div>

						<div class="mb-8">
							{#if billingInterval === "monthly"}
								<span class="text-5xl font-bold text-gray-900"
									>$9</span
								>
								<span class="text-gray-600">/month</span>
							{:else}
								<span class="text-5xl font-bold text-gray-900"
									>$90</span
								>
								<span class="text-gray-600">/year</span>
								<div
									class="text-sm text-green-600 font-medium mt-1"
								>
									$7.50/mo — 2 months free
								</div>
							{/if}
						</div>

						<ul class="space-y-3 mb-8">
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>Everything in Free</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>1,000 links per month</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>1-year analytics retention</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>Custom short codes</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>Email support (48h)</span
								>
							</li>
						</ul>

						<button
							onclick={() => startCheckout("pro")}
							disabled={checkoutLoading ===
								`pro_${billingInterval}`}
							class="w-full px-6 py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm hover:shadow-md disabled:opacity-60 disabled:cursor-not-allowed text-center"
						>
							{checkoutLoading === `pro_${billingInterval}`
								? "Redirecting…"
								: currentUser
									? "Upgrade to Pro"
									: "Get Started"}
						</button>
					</div>

					<!-- Business Tier -->
					<div
						class="border border-gray-200 rounded-2xl p-8 transition-all duration-700 {mounted
							? 'opacity-100 translate-y-0'
							: 'opacity-0 translate-y-4'}"
						style="transition-delay: 300ms"
					>
						<div class="mb-8">
							<h3 class="text-2xl font-bold text-gray-900 mb-2">
								Business
							</h3>
							<p class="text-gray-600">
								For teams needing long-term analytics insights
							</p>
						</div>

						<div class="mb-8">
							{#if billingInterval === "monthly"}
								<span class="text-5xl font-bold text-gray-900"
									>$29</span
								>
								<span class="text-gray-600">/month</span>
							{:else}
								<span class="text-5xl font-bold text-gray-900"
									>$290</span
								>
								<span class="text-gray-600">/year</span>
								<div
									class="text-sm text-green-600 font-medium mt-1"
								>
									$24.17/mo — 2 months free
								</div>
							{/if}
						</div>

						<ul class="space-y-3 mb-8">
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>Everything in Pro</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>10,000 links per month</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>3-year analytics retention</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>3 organizations</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>20 team members</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>Device-based routing</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>Password protection</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>API access (future)</span
								>
							</li>
							<li class="flex items-start">
								<svg
									class="w-5 h-5 text-green-500 mr-3 mt-0.5 flex-shrink-0"
									fill="currentColor"
									viewBox="0 0 20 20"
									><path
										fill-rule="evenodd"
										d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
										clip-rule="evenodd"
									/></svg
								>
								<span class="text-gray-700"
									>Priority support (24h)</span
								>
							</li>
						</ul>

						<button
							onclick={() => startCheckout("business")}
							disabled={checkoutLoading ===
								`business_${billingInterval}`}
							class="w-full px-6 py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm hover:shadow-md disabled:opacity-60 disabled:cursor-not-allowed text-center"
						>
							{checkoutLoading === `business_${billingInterval}`
								? "Redirecting…"
								: currentUser
									? "Upgrade to Business"
									: "Get Started"}
						</button>
					</div>
				</div>

				{#if checkoutError}
					<div
						class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm text-center"
					>
						{checkoutError}
					</div>
				{/if}

				<!-- FAQ / Notes -->
				<div class="text-center max-w-3xl mx-auto">
					<div
						class="bg-orange-50 border border-orange-200 rounded-lg p-6"
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
									>Pro: <span
										class="line-through text-gray-500"
										>$9</span
									> $5/mo</span
								>
								<span class="text-gray-400">·</span>
								<span class="font-medium text-gray-900"
									>Business: <span
										class="line-through text-gray-500"
										>$29</span
									> $19/mo</span
								>
							</div>
						</div>

						<div
							class="flex flex-col sm:flex-row gap-4 justify-center"
						>
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
			</div>
		</section>
	</main>

	<Footer />
</div>
