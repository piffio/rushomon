<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import Footer from "$lib/components/Footer.svelte";
	import { authApi } from "$lib/api/auth";
	import { onMount } from "svelte";
	import type { PageData } from "./$types";

	let { data }: { data: PageData } = $props();

	const loginUrl = authApi.getLoginUrl();

	let mounted = $state(false);

	onMount(() => {
		mounted = true;
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
	<Header user={data.user} currentPage="landing" />

	<main class="flex-1">
		<section class="container mx-auto px-4 py-20 md:py-32">
			<div class="max-w-5xl mx-auto">
				<!-- Header -->
				<div
					class="text-center mb-16 transition-all duration-700 {mounted
						? 'opacity-100 translate-y-0'
						: 'opacity-0 translate-y-4'}"
				>
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
									>25 links per month</span
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
								For creators who need extended analytics history
							</p>
						</div>

						<div class="mb-8">
							<span class="text-5xl font-bold text-gray-900"
								>$9</span
							>
							<span class="text-gray-600">/month</span>
							<div class="text-sm text-gray-500 mt-1">
								or $90/year (2 months free)
							</div>
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
									>Everything in Free</span
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
									>1,000 links per month</span
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
									>1-year analytics retention</span
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
									>Device-based routing</span
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
									>Password protection</span
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
									>Email support (48h)</span
								>
							</li>
						</ul>

						<button
							disabled
							class="w-full px-6 py-3 bg-gray-100 text-gray-400 rounded-lg font-semibold cursor-not-allowed text-center"
						>
							Coming Soon
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
							<span class="text-5xl font-bold text-gray-900"
								>$29</span
							>
							<span class="text-gray-600">/month</span>
							<div class="text-sm text-gray-500 mt-1">
								or $290/year (2 months free)
							</div>
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
									>Everything in Pro</span
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
									>10,000 links per month</span
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
									>3-year analytics retention</span
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
									>3 organizations</span
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
									>10 team members</span
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
									>API access (future)</span
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
									>Priority support (24h)</span
								>
							</li>
						</ul>

						<button
							disabled
							class="w-full px-6 py-3 bg-gray-100 text-gray-400 rounded-lg font-semibold cursor-not-allowed text-center"
						>
							Coming Soon
						</button>
					</div>
				</div>

				<!-- FAQ / Notes -->
				<div class="text-center max-w-3xl mx-auto">
					<div
						class="bg-orange-50 border border-orange-200 rounded-lg p-6"
					>
						<h3 class="text-lg font-semibold text-gray-900 mb-3">
							🚀 Paid tiers coming soon!
						</h3>
						<p class="text-gray-700 mb-4">
							We're currently in beta with the Free tier. Sign up
							now to get early access to paid features and special
							founder pricing when we launch.
						</p>
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
