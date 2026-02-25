<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import Footer from "$lib/components/Footer.svelte";
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { page } from "$app/stores";
	import type { PageData } from "./$types";

	let { data }: { data: PageData } = $props();

	let mounted = $state(false);
	let signupsDisabled = $derived(
		$page.url.searchParams.get("error") === "signups_disabled",
	);

	onMount(() => {
		mounted = true;
	});
</script>

<svelte:head>
	<title>Rushomon - Self-Hosted URL Shortener</title>
</svelte:head>

<div class="min-h-screen bg-white flex flex-col">
	<Header user={data.user} currentPage="landing" />

	<!-- Hero Section -->
	<main class="flex-1">
		<section class="container mx-auto px-4 py-20 md:py-32">
			<div class="max-w-4xl mx-auto text-center">
				<div
					class="transition-all duration-700 {mounted
						? 'opacity-100 translate-y-0'
						: 'opacity-0 translate-y-4'}"
				>
					<h2
						class="text-5xl md:text-7xl font-bold text-gray-900 mb-8 leading-tight"
					>
						Your Own<br />
						<span
							class="text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-orange-600"
						>
							URL Shortener
						</span>
					</h2>
					<p
						class="text-xl md:text-2xl text-gray-600 mb-12 leading-relaxed max-w-2xl mx-auto"
					>
						Self-hosted, blazing fast, and powerful. Create custom
						short links with analytics on your own domain.
					</p>

					<!-- Signups Disabled Error -->
					{#if signupsDisabled}
						<div
							class="bg-red-50 border border-red-200 text-red-700 px-6 py-4 rounded-xl mb-8 max-w-xl mx-auto"
						>
							<p class="font-medium">
								New signups are currently disabled
							</p>
							<p class="text-sm mt-1 text-red-600">
								Contact the administrator if you need access to
								this instance.
							</p>
						</div>
					{/if}

					<!-- CTA Button -->
					<a
						href={data.user ? "/dashboard" : "/login"}
						class="inline-flex items-center gap-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white px-10 py-5 rounded-xl text-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-lg hover:shadow-xl hover:scale-105 transform duration-200"
					>
						{data.user ? "Go to Dashboard" : "Get Started"}
						<svg
							class="w-6 h-6"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M13 7l5 5m0 0l-5 5m5-5H6"
							/>
						</svg>
					</a>
				</div>
			</div>
		</section>

		<!-- Dashboard Preview Section -->
		<section class="bg-gradient-to-b from-gray-50 to-white py-20">
			<div class="container mx-auto px-4">
				<div class="max-w-5xl mx-auto">
					<h3
						class="text-3xl md:text-4xl font-bold text-gray-900 text-center mb-6"
					>
						Simple, Powerful Interface
					</h3>
					<p
						class="text-lg text-gray-600 text-center mb-12 max-w-2xl mx-auto"
					>
						A clean dashboard to create, manage, and track your
						short links. No complexity, just what you need.
					</p>
					<div
						class="bg-gradient-to-br from-gray-100 to-gray-50 rounded-2xl p-8 md:p-12 shadow-xl border border-gray-200"
					>
						<div
							class="bg-white rounded-xl shadow-lg overflow-hidden border border-gray-200"
						>
							<!-- Mock Dashboard Preview -->
							<div
								class="bg-gradient-to-r from-orange-500 to-orange-600 px-6 py-4 flex items-center justify-between"
							>
								<div class="flex items-center gap-2">
									<div
										class="w-6 h-6 bg-white/20 rounded"
									></div>
									<div class="text-white font-semibold">
										Dashboard
									</div>
								</div>
								<div
									class="w-8 h-8 bg-white/20 rounded-full"
								></div>
							</div>
							<div class="p-6 space-y-4">
								<div class="flex gap-4">
									<div
										class="flex-1 h-12 bg-gray-100 rounded-lg"
									></div>
									<div
										class="w-32 h-12 bg-gradient-to-r from-orange-500 to-orange-600 rounded-lg"
									></div>
								</div>
								<div class="space-y-3">
									<div
										class="flex items-center gap-4 p-4 bg-gray-50 rounded-lg border border-gray-200"
									>
										<div
											class="w-10 h-10 bg-orange-100 rounded-lg"
										></div>
										<div class="flex-1 space-y-2">
											<div
												class="h-4 bg-gray-200 rounded w-2/3"
											></div>
											<div
												class="h-3 bg-gray-100 rounded w-1/2"
											></div>
										</div>
										<div
											class="h-8 w-20 bg-gray-200 rounded"
										></div>
									</div>
									<div
										class="flex items-center gap-4 p-4 bg-gray-50 rounded-lg border border-gray-200"
									>
										<div
											class="w-10 h-10 bg-orange-100 rounded-lg"
										></div>
										<div class="flex-1 space-y-2">
											<div
												class="h-4 bg-gray-200 rounded w-2/3"
											></div>
											<div
												class="h-3 bg-gray-100 rounded w-1/2"
											></div>
										</div>
										<div
											class="h-8 w-20 bg-gray-200 rounded"
										></div>
									</div>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</section>

		<!-- Features Section -->
		<section id="features" class="py-20 bg-white">
			<div class="container mx-auto px-4">
				<div class="max-w-6xl mx-auto">
					<h3
						class="text-3xl md:text-4xl font-bold text-gray-900 text-center mb-16"
					>
						Everything You Need
					</h3>

					<div class="grid md:grid-cols-3 gap-8">
						<!-- Lightning Fast -->
						<div
							class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
						>
							<div
								class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
							>
								<svg
									class="w-7 h-7 text-white"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M13 10V3L4 14h7v7l9-11h-7z"
									/>
								</svg>
							</div>
							<h4
								class="text-xl font-semibold text-gray-900 mb-3"
							>
								Lightning Fast
							</h4>
							<p class="text-gray-600 leading-relaxed">
								Powered by Cloudflare's edge network for
								sub-millisecond redirects worldwide.
							</p>
						</div>

						<!-- Custom Codes -->
						<div
							class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
						>
							<div
								class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
							>
								<svg
									class="w-7 h-7 text-white"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
									/>
								</svg>
							</div>
							<h4
								class="text-xl font-semibold text-gray-900 mb-3"
							>
								Custom Codes
							</h4>
							<p class="text-gray-600 leading-relaxed">
								Choose your own memorable short codes or let the
								system generate them automatically.
							</p>
						</div>

						<!-- Analytics -->
						<div
							class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
						>
							<div
								class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
							>
								<svg
									class="w-7 h-7 text-white"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
									/>
								</svg>
							</div>
							<h4
								class="text-xl font-semibold text-gray-900 mb-3"
							>
								Analytics
							</h4>
							<p class="text-gray-600 leading-relaxed">
								Track clicks, referrers, and geographic data to
								understand your link performance.
							</p>
						</div>

						<!-- Self-Hosted -->
						<div
							class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
						>
							<div
								class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
							>
								<svg
									class="w-7 h-7 text-white"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
									/>
								</svg>
							</div>
							<h4
								class="text-xl font-semibold text-gray-900 mb-3"
							>
								Self-Hosted
							</h4>
							<p class="text-gray-600 leading-relaxed">
								Full control over your data. Deploy on your own
								domain with Cloudflare Workers.
							</p>
						</div>

						<!-- Open Source -->
						<div
							class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
						>
							<div
								class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
							>
								<svg
									class="w-7 h-7 text-white"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
									/>
								</svg>
							</div>
							<h4
								class="text-xl font-semibold text-gray-900 mb-3"
							>
								Open Source
							</h4>
							<p class="text-gray-600 leading-relaxed">
								Built with Rust and SvelteKit. Free and open
								source under AGPL-3.0 license.
							</p>
						</div>

						<!-- Multi-Tenant -->
						<div
							class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
						>
							<div
								class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
							>
								<svg
									class="w-7 h-7 text-white"
									fill="none"
									stroke="currentColor"
									viewBox="0 0 24 24"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
									/>
								</svg>
							</div>
							<h4
								class="text-xl font-semibold text-gray-900 mb-3"
							>
								Multi-Tenant
							</h4>
							<p class="text-gray-600 leading-relaxed">
								Organization support built-in. Perfect for teams
								and personal use alike.
							</p>
						</div>
					</div>
				</div>
			</div>
		</section>
	</main>

	<Footer />
</div>
