<script lang="ts">
	import { authApi } from '$lib/api/auth';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const loginUrl = authApi.getLoginUrl();

	let mounted = $state(false);
	onMount(() => {
		mounted = true;
	});
</script>

<svelte:head>
	<title>Rushomon - Self-Hosted URL Shortener</title>
</svelte:head>

<div class="min-h-screen bg-white flex flex-col">
	<!-- Header -->
	<header class="border-b border-gray-200">
		<div class="container mx-auto px-4 py-6">
			<div class="flex justify-between items-center">
				<div class="flex items-center gap-2">
					<div class="w-8 h-8 bg-gradient-to-br from-orange-500 to-orange-600 rounded-lg flex items-center justify-center">
						<span class="text-white font-bold text-sm">R</span>
					</div>
					<h1 class="text-2xl font-bold text-gray-900">Rushomon</h1>
				</div>
				<div class="flex items-center gap-4">
					{#if data.user}
						<!-- Dashboard Button (only visible if authenticated) -->
						<a
							href="/dashboard"
							class="px-4 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm hover:shadow-md"
						>
							Go to Dashboard →
						</a>
					{/if}
					<a
						href="https://github.com/piffio/rushomon"
						class="text-gray-600 hover:text-orange-600 transition-colors"
						target="_blank"
						rel="noopener noreferrer"
						aria-label="View on GitHub"
					>
						<svg class="w-6 h-6" fill="currentColor" viewBox="0 0 24 24">
							<path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
						</svg>
					</a>
				</div>
			</div>
		</div>
	</header>

	<!-- Hero Section -->
	<main class="flex-1">
		<section class="container mx-auto px-4 py-20 md:py-32">
			<div class="max-w-4xl mx-auto text-center">
				<div class="transition-all duration-700 {mounted ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-4'}">
					<h2 class="text-5xl md:text-7xl font-bold text-gray-900 mb-8 leading-tight">
						Your Own<br />
						<span class="text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-orange-600">
							URL Shortener
						</span>
					</h2>
					<p class="text-xl md:text-2xl text-gray-600 mb-12 leading-relaxed max-w-2xl mx-auto">
						Self-hosted, blazing fast, and powerful. Create custom short links with analytics on your own domain.
					</p>

					<!-- CTA Button -->
					<a
						href={loginUrl}
						class="inline-flex items-center gap-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white px-10 py-5 rounded-xl text-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-lg hover:shadow-xl hover:scale-105 transform duration-200"
					>
						<svg class="w-6 h-6" fill="currentColor" viewBox="0 0 24 24">
							<path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
						</svg>
						Sign in with GitHub
					</a>
				</div>
			</div>
		</section>

		<!-- Dashboard Preview Section -->
		<section class="bg-gradient-to-b from-gray-50 to-white py-20">
			<div class="container mx-auto px-4">
				<div class="max-w-5xl mx-auto">
					<h3 class="text-3xl md:text-4xl font-bold text-gray-900 text-center mb-6">
						Simple, Powerful Interface
					</h3>
					<p class="text-lg text-gray-600 text-center mb-12 max-w-2xl mx-auto">
						A clean dashboard to create, manage, and track your short links. No complexity, just what you need.
					</p>
					<div class="bg-gradient-to-br from-gray-100 to-gray-50 rounded-2xl p-8 md:p-12 shadow-xl border border-gray-200">
						<div class="bg-white rounded-xl shadow-lg overflow-hidden border border-gray-200">
							<!-- Mock Dashboard Preview -->
							<div class="bg-gradient-to-r from-orange-500 to-orange-600 px-6 py-4 flex items-center justify-between">
								<div class="flex items-center gap-2">
									<div class="w-6 h-6 bg-white/20 rounded"></div>
									<div class="text-white font-semibold">Dashboard</div>
								</div>
								<div class="w-8 h-8 bg-white/20 rounded-full"></div>
							</div>
							<div class="p-6 space-y-4">
								<div class="flex gap-4">
									<div class="flex-1 h-12 bg-gray-100 rounded-lg"></div>
									<div class="w-32 h-12 bg-gradient-to-r from-orange-500 to-orange-600 rounded-lg"></div>
								</div>
								<div class="space-y-3">
									<div class="flex items-center gap-4 p-4 bg-gray-50 rounded-lg border border-gray-200">
										<div class="w-10 h-10 bg-orange-100 rounded-lg"></div>
										<div class="flex-1 space-y-2">
											<div class="h-4 bg-gray-200 rounded w-2/3"></div>
											<div class="h-3 bg-gray-100 rounded w-1/2"></div>
										</div>
										<div class="h-8 w-20 bg-gray-200 rounded"></div>
									</div>
									<div class="flex items-center gap-4 p-4 bg-gray-50 rounded-lg border border-gray-200">
										<div class="w-10 h-10 bg-orange-100 rounded-lg"></div>
										<div class="flex-1 space-y-2">
											<div class="h-4 bg-gray-200 rounded w-2/3"></div>
											<div class="h-3 bg-gray-100 rounded w-1/2"></div>
										</div>
										<div class="h-8 w-20 bg-gray-200 rounded"></div>
									</div>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</section>

		<!-- Features Section -->
		<section class="py-20 bg-white">
			<div class="container mx-auto px-4">
				<div class="max-w-6xl mx-auto">
					<h3 class="text-3xl md:text-4xl font-bold text-gray-900 text-center mb-16">
						Everything You Need
					</h3>

					<div class="grid md:grid-cols-3 gap-8">
						<!-- Lightning Fast -->
						<div class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white">
							<div class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5">
								<svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
								</svg>
							</div>
							<h4 class="text-xl font-semibold text-gray-900 mb-3">Lightning Fast</h4>
							<p class="text-gray-600 leading-relaxed">
								Powered by Cloudflare's edge network for sub-millisecond redirects worldwide.
							</p>
						</div>

						<!-- Custom Codes -->
						<div class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white">
							<div class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5">
								<svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
								</svg>
							</div>
							<h4 class="text-xl font-semibold text-gray-900 mb-3">Custom Codes</h4>
							<p class="text-gray-600 leading-relaxed">
								Choose your own memorable short codes or let the system generate them automatically.
							</p>
						</div>

						<!-- Analytics -->
						<div class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white">
							<div class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5">
								<svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
								</svg>
							</div>
							<h4 class="text-xl font-semibold text-gray-900 mb-3">Analytics</h4>
							<p class="text-gray-600 leading-relaxed">
								Track clicks, referrers, and geographic data to understand your link performance.
							</p>
						</div>

						<!-- Self-Hosted -->
						<div class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white">
							<div class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5">
								<svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
								</svg>
							</div>
							<h4 class="text-xl font-semibold text-gray-900 mb-3">Self-Hosted</h4>
							<p class="text-gray-600 leading-relaxed">
								Full control over your data. Deploy on your own domain with Cloudflare Workers.
							</p>
						</div>

						<!-- Open Source -->
						<div class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white">
							<div class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5">
								<svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
								</svg>
							</div>
							<h4 class="text-xl font-semibold text-gray-900 mb-3">Open Source</h4>
							<p class="text-gray-600 leading-relaxed">
								Built with Rust and SvelteKit. Free and open source under AGPL-3.0 license.
							</p>
						</div>

						<!-- Multi-Tenant -->
						<div class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white">
							<div class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5">
								<svg class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
								</svg>
							</div>
							<h4 class="text-xl font-semibold text-gray-900 mb-3">Multi-Tenant</h4>
							<p class="text-gray-600 leading-relaxed">
								Organization support built-in. Perfect for teams and personal use alike.
							</p>
						</div>
					</div>
				</div>
			</div>
		</section>
	</main>

	<!-- Footer -->
	<footer class="border-t border-gray-200 py-12 bg-gray-50">
		<div class="container mx-auto px-4">
			<div class="max-w-6xl mx-auto text-center">
				<p class="text-gray-600 mb-4">
					Built with
					<a href="https://kit.svelte.dev" class="text-orange-600 hover:text-orange-700 font-medium transition-colors" target="_blank" rel="noopener noreferrer">SvelteKit</a>
					and
					<a href="https://workers.cloudflare.com" class="text-orange-600 hover:text-orange-700 font-medium transition-colors" target="_blank" rel="noopener noreferrer">Cloudflare Workers</a>
				</p>
				<p class="text-sm text-gray-500">
					<a href="https://github.com/piffio/rushomon" class="hover:text-orange-600 transition-colors" target="_blank" rel="noopener noreferrer">View on GitHub</a>
					<span class="mx-2">·</span>
					Licensed under AGPL-3.0
				</p>
			</div>
		</div>
	</footer>
</div>
