<script lang="ts">
  import type {
    TopLinkCount,
    LinkAnalyticsResponse,
    UserAgentCount
  } from "$lib/types/api";
  import { linksApi } from "$lib/api/links";
  import {
    PUBLIC_VITE_SHORT_LINK_BASE_URL,
    PUBLIC_VITE_API_BASE_URL
  } from "$env/static/public";
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
    ArcElement
  } from "chart.js";
  import { UAParser } from "ua-parser-js";
  import countries from "i18n-iso-countries";
  import enLocale from "i18n-iso-countries/langs/en.json";
  import { browser } from "$app/environment";

  countries.registerLocale(enLocale);

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
    ArcElement
  );

  interface Props {
    link: TopLinkCount;
    days: number;
    onclose: () => void;
  }

  let { link, days, onclose }: Props = $props();

  const SHORT_LINK_BASE =
    PUBLIC_VITE_SHORT_LINK_BASE_URL ||
    PUBLIC_VITE_API_BASE_URL ||
    "http://localhost:8787";

  const shortUrl = $derived(`${SHORT_LINK_BASE}/${link.short_code}`);

  let analytics = $state<LinkAnalyticsResponse | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let copySuccess = $state(false);

  // Charts
  let clicksChartCanvas: HTMLCanvasElement = $state(null!);
  let clicksChart: Chart | null = null;

  const chartColors = [
    "#f97316",
    "#3b82f6",
    "#10b981",
    "#8b5cf6",
    "#ef4444",
    "#f59e0b",
    "#06b6d4",
    "#ec4899"
  ];

  function countryFlag(code: string): string {
    if (!code || code === "Unknown" || code.length !== 2) return "🌍";
    const pts = code
      .toUpperCase()
      .split("")
      .map((c) => 127397 + c.charCodeAt(0));
    return String.fromCodePoint(...pts);
  }

  function countryCodeToName(code: string): string {
    if (!code || code === "Unknown") return code;
    return countries.getName(code.toUpperCase(), "en") || code;
  }

  function barWidth(count: number, items: { count: number }[]): number {
    const max = items[0]?.count ?? 1;
    return max > 0 ? Math.round((count / max) * 100) : 0;
  }

  function createClicksChart() {
    if (!clicksChartCanvas || !analytics) return;
    if (clicksChart) clicksChart.destroy();
    const labels = analytics.clicks_over_time.map((d) => {
      const date = new Date(d.date + "T00:00:00");
      return date.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric"
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
            pointRadius: values.length > 30 ? 0 : 3,
            pointHoverRadius: 5,
            borderWidth: 2
          }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
          tooltip: { mode: "index", intersect: false }
        },
        scales: {
          x: {
            grid: { display: false },
            ticks: { maxTicksLimit: 8, font: { size: 10 } }
          },
          y: {
            beginAtZero: true,
            ticks: { precision: 0, font: { size: 10 } },
            grid: { color: "rgba(0,0,0,0.05)" }
          }
        },
        interaction: { mode: "nearest", axis: "x", intersect: false }
      }
    });
  }

  async function loadAnalytics() {
    loading = true;
    error = null;
    try {
      analytics = await linksApi.getAnalytics(link.link_id, days);
      setTimeout(() => createClicksChart(), 0);
    } catch (e: any) {
      error = e?.message ?? "Failed to load analytics";
    } finally {
      loading = false;
    }
  }

  async function copyToClipboard() {
    try {
      await navigator.clipboard.writeText(shortUrl);
      copySuccess = true;
      setTimeout(() => (copySuccess = false), 2000);
    } catch {
      // ignore
    }
  }

  // Keyboard close
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onclose();
  }

  onMount(() => {
    loadAnalytics();
    document.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    clicksChart?.destroy();
    if (browser) {
      document.removeEventListener("keydown", handleKeydown);
    }
  });
</script>

<!-- Backdrop -->
<div
  class="fixed inset-0 bg-black/30 z-40 transition-opacity"
  role="button"
  tabindex="-1"
  aria-label="Close panel"
  onclick={onclose}
  onkeydown={(e) => e.key === "Enter" && onclose()}
></div>

<!-- Slide-over panel -->
<div
  class="fixed right-0 top-0 h-full w-full max-w-md bg-white shadow-2xl z-50 flex flex-col overflow-hidden"
  role="dialog"
  aria-modal="true"
  aria-label="Link analytics"
>
  <!-- Header -->
  <div
    class="flex items-start justify-between gap-3 p-5 border-b border-gray-200"
  >
    <div class="min-w-0">
      <h2 class="text-base font-semibold text-gray-900 truncate">
        {link.title || "/" + link.short_code}
      </h2>
      <div class="flex items-center gap-2 mt-1">
        <a
          href={shortUrl}
          target="_blank"
          rel="noopener noreferrer"
          class="text-sm text-orange-600 hover:underline truncate"
        >
          {shortUrl}
        </a>
        <button
          onclick={copyToClipboard}
          class="flex-shrink-0 text-gray-400 hover:text-gray-600 transition-colors"
          title="Copy URL"
          aria-label="Copy short URL"
        >
          {#if copySuccess}
            <svg
              class="w-4 h-4 text-green-500"
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
    <div class="flex items-center gap-2 flex-shrink-0">
      <a
        href="/dashboard/links/{link.short_code}"
        class="text-xs text-gray-500 hover:text-orange-600 transition-colors whitespace-nowrap"
      >
        Open full page →
      </a>
      <button
        onclick={onclose}
        class="p-1 text-gray-400 hover:text-gray-600 transition-colors rounded-lg hover:bg-gray-100"
        aria-label="Close"
      >
        <svg
          class="w-5 h-5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M6 18L18 6M6 6l12 12"
          />
        </svg>
      </button>
    </div>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-y-auto p-5 space-y-5">
    {#if loading}
      <div class="space-y-4">
        {#each [1, 2, 3] as _}
          <div class="bg-gray-100 rounded-xl h-24 animate-pulse"></div>
        {/each}
      </div>
    {:else if error}
      <div class="text-center py-8 text-sm text-red-500">{error}</div>
    {:else if analytics}
      <!-- Total clicks stat -->
      <div class="bg-orange-50 rounded-xl p-4 text-center">
        <p class="text-xs text-orange-600 font-medium uppercase tracking-wide">
          Clicks (last {days === 0 ? "all time" : `${days}d`})
        </p>
        <p class="text-3xl font-bold text-orange-700 mt-1">
          {analytics.total_clicks_in_range.toLocaleString()}
        </p>
      </div>

      <!-- Clicks over time chart -->
      {#if analytics.clicks_over_time.length > 0}
        <div class="bg-white border border-gray-200 rounded-xl p-4">
          <h3 class="text-xs font-semibold text-gray-600 mb-3">
            Clicks over time
          </h3>
          <div class="h-36">
            <canvas bind:this={clicksChartCanvas}></canvas>
          </div>
        </div>
      {/if}

      <!-- Top referrers -->
      {#if analytics.top_referrers.length > 0}
        <div class="bg-white border border-gray-200 rounded-xl p-4">
          <h3 class="text-xs font-semibold text-gray-600 mb-3">
            Top referrers
          </h3>
          <div class="space-y-1.5">
            {#each analytics.top_referrers.slice(0, 5) as ref}
              <div class="flex items-center justify-between gap-3 text-sm">
                <span class="text-gray-700 truncate">{ref.referrer}</span>
                <div class="flex items-center gap-2 flex-shrink-0">
                  <div
                    class="w-12 h-1.5 bg-gray-100 rounded-full overflow-hidden"
                  >
                    <div
                      class="h-full bg-blue-400 rounded-full"
                      style="width: {barWidth(
                        ref.count,
                        analytics.top_referrers
                      )}%"
                    ></div>
                  </div>
                  <span class="text-gray-600 font-medium w-8 text-right"
                    >{ref.count}</span
                  >
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Top countries -->
      {#if analytics.top_countries.length > 0}
        <div class="bg-white border border-gray-200 rounded-xl p-4">
          <h3 class="text-xs font-semibold text-gray-600 mb-3">
            Top countries
          </h3>
          <div class="space-y-1.5">
            {#each analytics.top_countries.slice(0, 5) as country}
              <div class="flex items-center justify-between gap-3 text-sm">
                <span class="flex items-center gap-1.5 text-gray-700 truncate">
                  <span>{countryFlag(country.country)}</span>
                  {countryCodeToName(country.country)}
                </span>
                <div class="flex items-center gap-2 flex-shrink-0">
                  <div
                    class="w-12 h-1.5 bg-gray-100 rounded-full overflow-hidden"
                  >
                    <div
                      class="h-full bg-green-400 rounded-full"
                      style="width: {barWidth(
                        country.count,
                        analytics.top_countries
                      )}%"
                    ></div>
                  </div>
                  <span class="text-gray-600 font-medium w-8 text-right"
                    >{country.count}</span
                  >
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
