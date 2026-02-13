<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import type { LinkAnalyticsResponse, UserAgentCount } from "$lib/types/api";
	import {
		PUBLIC_VITE_API_BASE_URL,
		PUBLIC_VITE_SHORT_LINK_BASE_URL,
	} from "$env/static/public";
	import { goto } from "$app/navigation";
	import { page } from "$app/stores";
	import { onMount, onDestroy } from "svelte";
	import {
		Chart,
		LineController,
		LineElement,
		PointElement,
		LinearScale,
		CategoryScale,
		Filler,
		Tooltip,
		Legend,
		DoughnutController,
		ArcElement,
		BarController,
		BarElement,
	} from "chart.js";
	import UAParser from "ua-parser-js";

	Chart.register(
		LineController,
		LineElement,
		PointElement,
		LinearScale,
		CategoryScale,
		Filler,
		Tooltip,
		Legend,
		DoughnutController,
		ArcElement,
		BarController,
		BarElement,
	);

	let { data } = $props();

	const SHORT_LINK_BASE =
		PUBLIC_VITE_SHORT_LINK_BASE_URL ||
		PUBLIC_VITE_API_BASE_URL ||
		"http://localhost:8787";

	const link = $derived(data.analytics?.link ?? data.link);
	const analytics = $derived(data.analytics);
	const days = $derived(data.days ?? 30);
	const shortUrl = $derived(
		link ? `${SHORT_LINK_BASE}/${link.short_code}` : "",
	);

	const timeRanges = [
		{ label: "Last 7 days", value: 7 },
		{ label: "Last 30 days", value: 30 },
		{ label: "Last 90 days", value: 90 },
		{ label: "All time", value: 0 },
	];

	function selectTimeRange(value: number) {
		const params = new URLSearchParams($page.url.searchParams);
		if (value === 30) {
			params.delete("days");
		} else {
			params.set("days", value.toString());
		}
		const query = params.toString();
		goto(`${$page.url.pathname}${query ? `?${query}` : ""}`, {
			invalidateAll: true,
		});
	}

	// Country code to emoji flag
	function countryFlag(code: string): string {
		if (!code || code === "Unknown" || code.length !== 2) return "🌍";
		const codePoints = code
			.toUpperCase()
			.split("")
			.map((char) => 127397 + char.charCodeAt(0));
		return String.fromCodePoint(...codePoints);
	}

	// Parse user agents client-side
	interface ParsedUA {
		browser: string;
		os: string;
		count: number;
	}

	function parseUserAgents(agents: UserAgentCount[]): ParsedUA[] {
		return agents.map((a) => {
			const parser = new UAParser(a.user_agent);
			return {
				browser: parser.getBrowser().name || "Unknown",
				os: parser.getOS().name || "Unknown",
				count: a.count,
			};
		});
	}

	// Aggregate parsed UAs by browser
	function aggregateByBrowser(
		parsed: ParsedUA[],
	): { name: string; count: number }[] {
		const map = new Map<string, number>();
		for (const p of parsed) {
			map.set(p.browser, (map.get(p.browser) || 0) + p.count);
		}
		return Array.from(map.entries())
			.map(([name, count]) => ({ name, count }))
			.sort((a, b) => b.count - a.count);
	}

	// Aggregate parsed UAs by OS
	function aggregateByOS(
		parsed: ParsedUA[],
	): { name: string; count: number }[] {
		const map = new Map<string, number>();
		for (const p of parsed) {
			map.set(p.os, (map.get(p.os) || 0) + p.count);
		}
		return Array.from(map.entries())
			.map(([name, count]) => ({ name, count }))
			.sort((a, b) => b.count - a.count);
	}

	const parsedUAs = $derived(
		analytics ? parseUserAgents(analytics.top_user_agents) : [],
	);
	const browserData = $derived(aggregateByBrowser(parsedUAs));
	const osData = $derived(aggregateByOS(parsedUAs));

	// Chart instances
	let clicksChartCanvas: HTMLCanvasElement = $state(null!);
	let browserChartCanvas: HTMLCanvasElement = $state(null!);
	let osChartCanvas: HTMLCanvasElement = $state(null!);
	let clicksChart: Chart | null = null;
	let browserChart: Chart | null = null;
	let osChart: Chart | null = null;

	const chartColors = [
		"#f97316",
		"#3b82f6",
		"#10b981",
		"#8b5cf6",
		"#ef4444",
		"#f59e0b",
		"#06b6d4",
		"#ec4899",
		"#84cc16",
		"#6366f1",
	];

	function createClicksChart() {
		if (!clicksChartCanvas || !analytics) return;
		if (clicksChart) clicksChart.destroy();

		const labels = analytics.clicks_over_time.map((d) => {
			const date = new Date(d.date + "T00:00:00");
			return date.toLocaleDateString("en-US", {
				month: "short",
				day: "numeric",
			});
		});
		const values = analytics.clicks_over_time.map((d) => d.count);

		clicksChart = new Chart(clicksChartCanvas, {
			type: "line",
			data: {
				labels,
				datasets: [
					{
						label: "Clicks",
						data: values,
						borderColor: "#f97316",
						backgroundColor: "rgba(249, 115, 22, 0.1)",
						fill: true,
						tension: 0.3,
						pointRadius: values.length > 60 ? 0 : 3,
						pointHoverRadius: 5,
						borderWidth: 2,
					},
				],
			},
			options: {
				responsive: true,
				maintainAspectRatio: false,
				plugins: {
					legend: { display: false },
					tooltip: {
						mode: "index",
						intersect: false,
					},
				},
				scales: {
					x: {
						grid: { display: false },
						ticks: {
							maxTicksLimit: 10,
							font: { size: 11 },
						},
					},
					y: {
						beginAtZero: true,
						ticks: {
							precision: 0,
							font: { size: 11 },
						},
						grid: { color: "rgba(0,0,0,0.05)" },
					},
				},
				interaction: {
					mode: "nearest",
					axis: "x",
					intersect: false,
				},
			},
		});
	}

	function createBrowserChart() {
		if (!browserChartCanvas || browserData.length === 0) return;
		if (browserChart) browserChart.destroy();

		browserChart = new Chart(browserChartCanvas, {
			type: "doughnut",
			data: {
				labels: browserData.map((b) => b.name),
				datasets: [
					{
						data: browserData.map((b) => b.count),
						backgroundColor: chartColors.slice(
							0,
							browserData.length,
						),
						borderWidth: 2,
						borderColor: "#fff",
					},
				],
			},
			options: {
				responsive: true,
				maintainAspectRatio: false,
				plugins: {
					legend: {
						position: "bottom",
						labels: { padding: 12, font: { size: 11 } },
					},
				},
				cutout: "60%",
			},
		});
	}

	function createOSChart() {
		if (!osChartCanvas || osData.length === 0) return;
		if (osChart) osChart.destroy();

		osChart = new Chart(osChartCanvas, {
			type: "doughnut",
			data: {
				labels: osData.map((o) => o.name),
				datasets: [
					{
						data: osData.map((o) => o.count),
						backgroundColor: chartColors.slice(0, osData.length),
						borderWidth: 2,
						borderColor: "#fff",
					},
				],
			},
			options: {
				responsive: true,
				maintainAspectRatio: false,
				plugins: {
					legend: {
						position: "bottom",
						labels: { padding: 12, font: { size: 11 } },
					},
				},
				cutout: "60%",
			},
		});
	}

	onMount(() => {
		createClicksChart();
		createBrowserChart();
		createOSChart();
	});

	onDestroy(() => {
		clicksChart?.destroy();
		browserChart?.destroy();
		osChart?.destroy();
	});

	// Recreate charts when analytics data changes
	$effect(() => {
		if (analytics) {
			// Need to tick so canvas refs are ready
			setTimeout(() => {
				createClicksChart();
				createBrowserChart();
				createOSChart();
			}, 0);
		}
	});

	function formatDate(timestamp: number): string {
		return new Date(timestamp * 1000).toLocaleDateString();
	}

	function getDomain(url: string): string {
		try {
			return new URL(url).hostname.replace("www.", "");
		} catch {
			return url;
		}
	}

	let copySuccess = $state(false);
	async function copyToClipboard() {
		try {
			await navigator.clipboard.writeText(shortUrl);
			copySuccess = true;
			setTimeout(() => (copySuccess = false), 2000);
		} catch (err) {
			console.error("Failed to copy:", err);
		}
	}
</script>

<svelte:head>
	<title>{link?.title || link?.short_code || "Link"} - Analytics</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
	<Header user={data.user} currentPage="dashboard" />

	<div class="max-w-6xl mx-auto px-4 py-8">
		<!-- Back Navigation -->
		<a
			href="/dashboard"
			class="inline-flex items-center gap-2 text-sm text-gray-600 hover:text-orange-600 mb-6 transition-colors"
		>
			<svg
				class="w-4 h-4"
				fill="none"
				stroke="currentColor"
				viewBox="0 0 24 24"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M15 19l-7-7 7-7"
				/>
			</svg>
			Back to Dashboard
		</a>

		{#if data.error}
			<div
				class="bg-red-50 border border-red-200 rounded-xl p-6 text-center"
			>
				<p class="text-red-600 font-medium">{data.error}</p>
				<a
					href="/dashboard"
					class="text-orange-600 hover:underline mt-2 inline-block"
				>
					Return to Dashboard
				</a>
			</div>
		{:else if link && analytics}
			<!-- Link Summary Header -->
			<div class="bg-white border-2 border-gray-200 rounded-xl p-6 mb-6">
				<div class="flex items-start justify-between gap-4 flex-wrap">
					<div class="flex-1 min-w-0">
						<h1
							class="text-2xl font-bold text-gray-900 truncate mb-2"
						>
							{link.title || link.short_code}
						</h1>
						<div class="flex items-center gap-3 text-sm flex-wrap">
							<a
								href={shortUrl}
								target="_blank"
								rel="noopener noreferrer"
								class="font-medium text-orange-600 hover:text-orange-700 hover:underline"
							>
								{shortUrl}
							</a>
							<span class="text-gray-400">→</span>
							<a
								href={link.destination_url}
								target="_blank"
								rel="noopener noreferrer"
								class="text-gray-600 hover:text-gray-900 hover:underline truncate"
								title={link.destination_url}
							>
								{getDomain(link.destination_url)}
							</a>
							<button
								onclick={copyToClipboard}
								class="ml-1 p-1.5 rounded-md transition-colors {copySuccess
									? 'bg-green-100 text-green-700'
									: 'bg-gray-100 hover:bg-gray-200 text-gray-600'}"
								title="Copy short link"
							>
								{#if copySuccess}
									<svg
										class="w-4 h-4"
										fill="none"
										stroke="currentColor"
										viewBox="0 0 24 24"
									>
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M5 13l4 4L19 7"
										/>
									</svg>
								{:else}
									<svg
										class="w-4 h-4"
										fill="none"
										stroke="currentColor"
										viewBox="0 0 24 24"
									>
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
										/>
									</svg>
								{/if}
							</button>
						</div>
					</div>

					<!-- Summary Stats -->
					<div class="flex items-center gap-6 text-center">
						<div>
							<p class="text-3xl font-bold text-gray-900">
								{link.click_count}
							</p>
							<p
								class="text-xs text-gray-500 uppercase tracking-wide"
							>
								Total Clicks
							</p>
						</div>
						<div class="w-px h-10 bg-gray-200"></div>
						<div>
							<p class="text-3xl font-bold text-orange-600">
								{analytics.total_clicks_in_range}
							</p>
							<p
								class="text-xs text-gray-500 uppercase tracking-wide"
							>
								{#if days === 0}All Time{:else}Last {days}d{/if}
							</p>
						</div>
						<div class="w-px h-10 bg-gray-200"></div>
						<div>
							<p class="text-sm text-gray-500">Created</p>
							<p class="text-sm font-medium text-gray-900">
								{formatDate(link.created_at)}
							</p>
						</div>
					</div>
				</div>
			</div>

			<!-- Time Range Selector -->
			<div class="flex items-center gap-2 mb-6">
				{#each timeRanges as range}
					<button
						onclick={() => selectTimeRange(range.value)}
						class="px-4 py-2 text-sm font-medium rounded-lg transition-colors {days ===
						range.value
							? 'bg-orange-600 text-white'
							: 'bg-white border border-gray-200 text-gray-700 hover:bg-gray-50'}"
					>
						{range.label}
					</button>
				{/each}
			</div>

			<!-- Clicks Over Time Chart -->
			<div class="bg-white border-2 border-gray-200 rounded-xl p-6 mb-6">
				<h2 class="text-lg font-semibold text-gray-900 mb-4">
					Clicks Over Time
				</h2>
				{#if analytics.clicks_over_time.length === 0}
					<div
						class="h-64 flex items-center justify-center text-gray-400"
					>
						<p>No click data for this period</p>
					</div>
				{:else}
					<div class="h-64">
						<canvas bind:this={clicksChartCanvas}></canvas>
					</div>
				{/if}
			</div>

			<!-- Stats Grid: 2x2 -->
			<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
				<!-- Top Referrers -->
				<div class="bg-white border-2 border-gray-200 rounded-xl p-6">
					<h2 class="text-lg font-semibold text-gray-900 mb-4">
						Top Referrers
					</h2>
					{#if analytics.top_referrers.length === 0}
						<p class="text-gray-400 text-sm">No referrer data</p>
					{:else}
						<div class="space-y-3">
							{#each analytics.top_referrers as ref, i}
								{@const maxCount =
									analytics.top_referrers[0].count}
								<div class="flex items-center gap-3">
									<span
										class="text-xs text-gray-400 w-5 text-right"
										>{i + 1}</span
									>
									<div class="flex-1 min-w-0">
										<div
											class="flex items-center justify-between mb-1"
										>
											<span
												class="text-sm text-gray-900 truncate"
												>{ref.referrer}</span
											>
											<span
												class="text-sm font-medium text-gray-600 ml-2"
												>{ref.count}</span
											>
										</div>
										<div
											class="w-full bg-gray-100 rounded-full h-1.5"
										>
											<div
												class="bg-orange-500 h-1.5 rounded-full"
												style="width: {(ref.count /
													maxCount) *
													100}%"
											></div>
										</div>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>

				<!-- Top Countries -->
				<div class="bg-white border-2 border-gray-200 rounded-xl p-6">
					<h2 class="text-lg font-semibold text-gray-900 mb-4">
						Top Countries
					</h2>
					{#if analytics.top_countries.length === 0}
						<p class="text-gray-400 text-sm">No country data</p>
					{:else}
						<div class="space-y-3">
							{#each analytics.top_countries as country, i}
								{@const maxCount =
									analytics.top_countries[0].count}
								<div class="flex items-center gap-3">
									<span class="text-lg"
										>{countryFlag(country.country)}</span
									>
									<div class="flex-1 min-w-0">
										<div
											class="flex items-center justify-between mb-1"
										>
											<span class="text-sm text-gray-900"
												>{country.country}</span
											>
											<span
												class="text-sm font-medium text-gray-600 ml-2"
												>{country.count}</span
											>
										</div>
										<div
											class="w-full bg-gray-100 rounded-full h-1.5"
										>
											<div
												class="bg-blue-500 h-1.5 rounded-full"
												style="width: {(country.count /
													maxCount) *
													100}%"
											></div>
										</div>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>

				<!-- Browser Breakdown -->
				<div class="bg-white border-2 border-gray-200 rounded-xl p-6">
					<h2 class="text-lg font-semibold text-gray-900 mb-4">
						Browsers
					</h2>
					{#if browserData.length === 0}
						<p class="text-gray-400 text-sm">No browser data</p>
					{:else}
						<div class="h-56">
							<canvas bind:this={browserChartCanvas}></canvas>
						</div>
					{/if}
				</div>

				<!-- OS Breakdown -->
				<div class="bg-white border-2 border-gray-200 rounded-xl p-6">
					<h2 class="text-lg font-semibold text-gray-900 mb-4">
						Operating Systems
					</h2>
					{#if osData.length === 0}
						<p class="text-gray-400 text-sm">No OS data</p>
					{:else}
						<div class="h-56">
							<canvas bind:this={osChartCanvas}></canvas>
						</div>
					{/if}
				</div>
			</div>
		{:else}
			<!-- Loading state -->
			<div class="flex items-center justify-center py-20">
				<div
					class="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600"
				></div>
			</div>
		{/if}
	</div>
</div>
