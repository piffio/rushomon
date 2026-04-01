<script lang="ts">
  import type {
    OrgAnalyticsResponse,
    TopLinkCount,
    UserAgentCount
  } from "$lib/types/api";
  import { analyticsApi } from "$lib/api/analytics";
  import { linksApi } from "$lib/api/links";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
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
  import OrgLinkSlideOver from "$lib/components/OrgLinkSlideOver.svelte";
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

  const { data } = $props();

  const tier = $derived((data.tier as string) || "free");

  function hasTierAccess(requiredTier: string): boolean {
    const tierLevels: Record<string, number> = {
      free: 0,
      pro: 1,
      business: 2,
      unlimited: 3
    };
    return (tierLevels[tier] ?? 0) >= (tierLevels[requiredTier] ?? 0);
  }

  // ── Time range state ──────────────────────────────────────────────────────
  const timeRanges = [
    { label: "Last 7 days", value: 7, minTier: "free" },
    { label: "Last 30 days", value: 30, minTier: "pro" },
    { label: "Last 90 days", value: 90, minTier: "pro" },
    { label: "Last year", value: 365, minTier: "pro" },
    { label: "Last 3 years", value: 0, minTier: "business" }
  ];

  function getUpsellMessage(range: (typeof timeRanges)[0]): string {
    if (range.minTier === "business") {
      return "Upgrade to Business for unlimited analytics history";
    }
    return `Upgrade to ${range.minTier.charAt(0).toUpperCase() + range.minTier.slice(1)} to access ${range.label.toLowerCase()} analytics`;
  }

  let loadingRange = $state<number | string | null>(null);
  let lockedPopoverOpen = $state<string | null>(null);

  // Custom date range state
  let showDatePicker = $state(false);
  let customStartInput = $state("");
  let customEndInput = $state("");
  let customRangeLoading = $state(false);

  // ── Filter state ──────────────────────────────────────────────────────────
  let activeCountries = $state<string[]>([]);
  let activeReferrers = $state<string[]>([]);
  let filterLoading = $state(false);

  // ── Slide-over state ──────────────────────────────────────────────────────
  let slideOverLink = $state<TopLinkCount | null>(null);

  // ── Analytics data (may be re-fetched on filter change) ───────────────────
  let analytics = $derived(data.analytics);

  $effect(() => {
    // Reset loading state when data loads
    if (data.analytics) {
      loadingRange = null;
    }
  });

  // ── Derived display data (filtered client-side for breakdowns) ────────────
  const displayReferrers = $derived(() => {
    if (!analytics) return [];
    if (activeReferrers.length === 0) return analytics.top_referrers;
    return analytics.top_referrers.filter((r) =>
      activeReferrers.includes(r.referrer)
    );
  });

  const displayCountries = $derived(() => {
    if (!analytics) return [];
    if (activeCountries.length === 0) return analytics.top_countries;
    return analytics.top_countries.filter((c) =>
      activeCountries.includes(c.country)
    );
  });

  // User agent parsing
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
        count: a.count
      };
    });
  }

  function aggregateByKey(
    parsed: ParsedUA[],
    key: "browser" | "os"
  ): { name: string; count: number }[] {
    const map = new Map<string, number>();
    for (const p of parsed) map.set(p[key], (map.get(p[key]) || 0) + p.count);
    return Array.from(map.entries())
      .map(([name, count]) => ({ name, count }))
      .sort((a, b) => b.count - a.count);
  }

  const parsedUAs = $derived(
    analytics ? parseUserAgents(analytics.top_user_agents) : []
  );
  const browserData = $derived(aggregateByKey(parsedUAs, "browser"));
  const osData = $derived(aggregateByKey(parsedUAs, "os"));

  // ── Country helpers ───────────────────────────────────────────────────────
  function countryCodeToName(code: string): string {
    if (!code || code === "Unknown") return code;
    return countries.getName(code.toUpperCase(), "en") || code;
  }

  function countryFlag(code: string): string {
    if (!code || code === "Unknown" || code.length !== 2) return "🌍";
    const pts = code
      .toUpperCase()
      .split("")
      .map((c) => 127397 + c.charCodeAt(0));
    return String.fromCodePoint(...pts);
  }

  // ── Summary stats ─────────────────────────────────────────────────────────
  const avgClicksPerLink = $derived(() => {
    if (!analytics || analytics.unique_links_clicked === 0) return 0;
    return Math.round(analytics.total_clicks / analytics.unique_links_clicked);
  });

  // ── Chart setup ───────────────────────────────────────────────────────────
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
    "#6366f1"
  ];

  let clicksChartCanvas: HTMLCanvasElement = $state(null!);
  let browserChartCanvas: HTMLCanvasElement = $state(null!);
  let osChartCanvas: HTMLCanvasElement = $state(null!);
  let clicksChart: Chart | null = null;
  let browserChart: Chart | null = null;
  let osChart: Chart | null = null;

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
            pointRadius: values.length > 60 ? 0 : 3,
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
          tooltip: {
            mode: "index",
            intersect: false
          }
        },
        scales: {
          x: {
            grid: { display: false },
            ticks: {
              maxTicksLimit: 10,
              font: { size: 11 }
            }
          },
          y: {
            beginAtZero: true,
            ticks: {
              precision: 0,
              font: { size: 11 }
            },
            grid: { color: "rgba(0,0,0,0.05)" }
          }
        },
        interaction: {
          mode: "nearest",
          axis: "x",
          intersect: false
        }
      }
    });
  }

  function createDoughnutChart(
    canvas: HTMLCanvasElement,
    existing: Chart | null,
    items: { name: string; count: number }[]
  ): Chart | null {
    if (!canvas || items.length === 0) return existing;
    if (existing) existing.destroy();
    const total = items.reduce((s, i) => s + i.count, 0);
    return new Chart(canvas, {
      type: "doughnut",
      data: {
        labels: items.map(
          (idx, i) =>
            `${items[i].name} (${((items[i].count / total) * 100).toFixed(1)}%)`
        ),
        datasets: [
          {
            data: items.map((i) => i.count),
            backgroundColor: chartColors.slice(0, items.length),
            borderWidth: 2,
            borderColor: "#fff"
          }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: {
            position: "bottom",
            labels: { padding: 12, font: { size: 11 } }
          },
          tooltip: {
            callbacks: {
              label: (ctx) => {
                const pct = ((ctx.parsed / total) * 100).toFixed(1);
                return `${ctx.label}: ${ctx.parsed} (${pct}%)`;
              }
            }
          }
        },
        cutout: "60%"
      }
    });
  }

  function rebuildCharts() {
    setTimeout(() => {
      createClicksChart();
      browserChart = createDoughnutChart(
        browserChartCanvas,
        browserChart,
        browserData
      );
      osChart = createDoughnutChart(osChartCanvas, osChart, osData);
    }, 0);
  }

  onMount(() => {
    rebuildCharts();
    document.addEventListener("click", handleGlobalClick);
  });

  onDestroy(() => {
    clicksChart?.destroy();
    browserChart?.destroy();
    osChart?.destroy();
    if (browser) {
      document.removeEventListener("click", handleGlobalClick);
    }
  });

  $effect(() => {
    if (analytics) rebuildCharts();
  });

  // ── Navigation ────────────────────────────────────────────────────────────
  function handleGlobalClick(event: MouseEvent) {
    const target = event.target as Element;
    if (!target.closest(".relative")) lockedPopoverOpen = null;
    if (
      !target.closest(".date-picker-popover") &&
      !target.closest(".date-picker-trigger")
    )
      showDatePicker = false;
  }

  function selectTimeRange(range: (typeof timeRanges)[0]) {
    if (!hasTierAccess(range.minTier)) {
      lockedPopoverOpen = range.label;
      return;
    }
    loadingRange = range.value;
    const params = new URLSearchParams();
    if (range.value !== 7) params.set("days", range.value.toString());
    const q = params.toString();
    goto(`/dashboard/analytics${q ? `?${q}` : ""}`, {
      invalidateAll: true
    });
  }

  function clearCustomRange() {
    goto("/dashboard/analytics", { invalidateAll: true });
  }

  // ── Date range validation ───────────────────────────────────────────────────
  function validateDateRange() {
    if (!customStartInput || !customEndInput) return;

    const startDate = new Date(customStartInput);
    const endDate = new Date(customEndInput);
    const now = new Date();

    // Calculate max days allowed based on tier
    const maxDays = hasTierAccess("business") ? 3 * 365 : 365; // 3 years for Business, 1 year for Pro
    const maxDate = new Date(now.getTime() - maxDays * 24 * 60 * 60 * 1000);

    // Adjust start date if it's too old
    if (startDate < maxDate) {
      customStartInput = maxDate.toISOString().split("T")[0];
    }

    // Adjust end date if it's after today
    if (endDate > now) {
      customEndInput = now.toISOString().split("T")[0];
    }

    // Ensure start date is before end date
    if (new Date(customStartInput) > new Date(customEndInput)) {
      customEndInput = customStartInput;
    }
  }

  async function applyCustomRange() {
    if (!customStartInput || !customEndInput) return;
    if (!hasTierAccess("pro")) {
      lockedPopoverOpen = "custom";
      showDatePicker = false;
      return;
    }

    // Validate dates before applying
    validateDateRange();

    const startTs = Math.floor(new Date(customStartInput).getTime() / 1000);
    const endTs = Math.floor(
      new Date(customEndInput + "T23:59:59").getTime() / 1000
    );
    customRangeLoading = true;
    showDatePicker = false;
    goto(`/dashboard/analytics?start=${startTs}&end=${endTs}`, {
      invalidateAll: true
    });
  }

  // ── Click-to-filter ───────────────────────────────────────────────────────
  let filterDebounce: ReturnType<typeof setTimeout> | null = null;

  function toggleCountry(code: string) {
    if (activeCountries.includes(code)) {
      activeCountries = activeCountries.filter((c) => c !== code);
    } else {
      activeCountries = [...activeCountries, code];
    }
    scheduleFetch();
  }

  function toggleReferrer(ref: string) {
    if (activeReferrers.includes(ref)) {
      activeReferrers = activeReferrers.filter((r) => r !== ref);
    } else {
      activeReferrers = [...activeReferrers, ref];
    }
    scheduleFetch();
  }

  function clearAllFilters() {
    activeCountries = [];
    activeReferrers = [];
    analytics = data.analytics;
    rebuildCharts();
  }

  function scheduleFetch() {
    if (filterDebounce) clearTimeout(filterDebounce);
    filterDebounce = setTimeout(async () => {
      filterLoading = true;
      try {
        const params = buildFilteredParams();
        let result: OrgAnalyticsResponse;
        if (data.isCustomRange && data.startParam && data.endParam) {
          result = await analyticsApi.getOrgAnalyticsCustomRange(
            parseInt(data.startParam),
            parseInt(data.endParam)
          );
        } else {
          result = await analyticsApi.getOrgAnalytics(data.days ?? 7);
        }
        analytics = result;
        rebuildCharts();
      } finally {
        filterLoading = false;
      }
    }, 300);
  }

  function buildFilteredParams(): URLSearchParams {
    const params = new URLSearchParams();
    if (activeCountries.length === 1) params.set("country", activeCountries[0]);
    if (activeReferrers.length === 1)
      params.set("referrer", activeReferrers[0]);
    return params;
  }

  const hasActiveFilters = $derived(
    activeCountries.length > 0 || activeReferrers.length > 0
  );

  // ── Bar width helper ──────────────────────────────────────────────────────
  function barWidth(count: number, items: { count: number }[]): number {
    const max = items[0]?.count ?? 1;
    return max > 0 ? Math.round((count / max) * 100) : 0;
  }

  // ── Custom range chip label ───────────────────────────────────────────────
  const customRangeLabel = $derived(() => {
    if (!data.isCustomRange || !data.startParam || !data.endParam) return null;
    const s = new Date(parseInt(data.startParam) * 1000).toLocaleDateString(
      "en-US",
      {
        month: "short",
        day: "numeric"
      }
    );
    const e = new Date(parseInt(data.endParam) * 1000).toLocaleDateString(
      "en-US",
      {
        month: "short",
        day: "numeric"
      }
    );
    return `${s} – ${e}`;
  });

  const currentDays = $derived(data.days ?? 7);

  // ── Dynamic time range label ─────────────────────────────────────────────────
  function getTimeRangeLabel(): string {
    if (data.isCustomRange) return "Custom Range";
    const days = data.days ?? 7;
    if (days === 7) return "Last 7 days";
    if (days === 30) return "Last 30 days";
    if (days === 90) return "Last 90 days";
    if (days === 365) return "Last 1 year";
    if (days === 0) return "Last 3 years";
    return "Last 7 days";
  }

  const timeRangeLabel = $derived(getTimeRangeLabel());
</script>

<svelte:head>
  <title>Analytics - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
  {#if !data.analytics && !filterLoading}
    <div class="max-w-6xl mx-auto px-4 py-16 text-center text-gray-500">
      <p>Could not load analytics data. Please try again.</p>
    </div>
  {:else}
    <div class="max-w-6xl mx-auto px-4 py-6 space-y-6">
      <!-- Page title + time range controls -->
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h1 class="text-xl font-semibold text-gray-900">Analytics</h1>

        <div class="flex items-center gap-2 flex-wrap">
          <!-- Preset time range buttons -->
          {#each timeRanges as range}
            {@const isLocked = !hasTierAccess(range.minTier)}
            {@const isSelected =
              !data.isCustomRange && currentDays === range.value}
            <div class="relative">
              <button
                onclick={() => selectTimeRange(range)}
                disabled={loadingRange !== null}
                class="px-4 py-2 text-sm font-medium rounded-lg transition-colors flex items-center gap-1.5 justify-center min-h-[44px]
									{isLocked
                  ? 'bg-gray-100 border border-gray-200 text-gray-400 cursor-pointer hover:bg-gray-200'
                  : isSelected
                    ? 'bg-orange-600 text-white'
                    : 'bg-white border border-gray-200 text-gray-700 hover:bg-gray-50'}
									{loadingRange !== null ? ' opacity-60 cursor-not-allowed' : ''}"
              >
                {#if isLocked}🔒{/if}
                {range.label}
                {#if loadingRange === range.value}
                  <svg
                    class="animate-spin h-4 w-4"
                    fill="none"
                    viewBox="0 0 24 24"
                  >
                    <circle
                      class="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      stroke-width="4"
                    ></circle>
                    <path
                      class="opacity-75"
                      fill="currentColor"
                      d="m4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    ></path>
                  </svg>
                {/if}
              </button>

              {#if lockedPopoverOpen === range.label}
                <div
                  class="absolute top-full left-0 mt-2 w-56 bg-white border border-gray-200 rounded-xl shadow-lg p-3 z-20 text-sm"
                >
                  <p class="text-gray-700 mb-2">
                    {getUpsellMessage(range)}
                  </p>
                  <a
                    href="/pricing"
                    class="block w-full text-center px-3 py-1.5 bg-orange-500 text-white rounded-lg font-medium hover:bg-orange-600 transition-colors"
                  >
                    Upgrade to {range.minTier === "business"
                      ? "Business"
                      : "Pro"} →
                  </a>
                  <button
                    onclick={() => (lockedPopoverOpen = null)}
                    class="mt-1 w-full text-center text-xs text-gray-400 hover:text-gray-600"
                    >Dismiss</button
                  >
                </div>
              {/if}
            </div>
          {/each}

          <!-- Custom date range picker -->
          <div class="relative">
            <button
              disabled={loadingRange !== null}
              class="px-4 py-2 text-sm font-medium rounded-lg transition-colors flex items-center gap-1.5 justify-center min-h-[44px]
								{!hasTierAccess('pro')
                ? 'bg-gray-100 border border-gray-200 text-gray-400 cursor-pointer hover:bg-gray-200'
                : data.isCustomRange
                  ? 'bg-orange-600 text-white'
                  : 'bg-white border border-gray-200 text-gray-700 hover:bg-gray-50'}
								{loadingRange !== null ? ' opacity-60 cursor-not-allowed' : ''}"
              onclick={(e) => {
                e.stopPropagation();
                if (!hasTierAccess("pro")) {
                  lockedPopoverOpen = "custom";
                  return;
                }
                showDatePicker = !showDatePicker;
              }}
            >
              {#if !hasTierAccess("pro")}🔒{/if}
              📅 Custom
            </button>

            {#if lockedPopoverOpen === "custom"}
              <div
                class="absolute top-full right-0 mt-2 w-56 bg-white border border-gray-200 rounded-xl shadow-lg p-3 z-20 text-sm"
              >
                <p class="text-gray-700 mb-2">
                  Custom date ranges are available on Pro.
                </p>
                <a
                  href="/pricing"
                  class="block w-full text-center px-3 py-1.5 bg-orange-500 text-white rounded-lg font-medium hover:bg-orange-600 transition-colors"
                >
                  Upgrade to Pro →
                </a>
                <button
                  onclick={() => (lockedPopoverOpen = null)}
                  class="mt-1 w-full text-center text-xs text-gray-400 hover:text-gray-600"
                  >Dismiss</button
                >
              </div>
            {/if}

            {#if showDatePicker}
              <div
                role="dialog"
                aria-labelledby="date-picker-title"
                tabindex="0"
                class="date-picker-popover absolute top-full right-0 mt-2 bg-white border border-gray-200 rounded-xl shadow-lg p-4 z-20 w-64"
                onclick={(e) => e.stopPropagation()}
                onkeydown={(e) => {
                  if (e.key === "Escape") {
                    showDatePicker = false;
                  }
                }}
              >
                <p
                  id="date-picker-title"
                  class="text-xs font-medium text-gray-600 mb-2"
                >
                  Select date range
                </p>
                <div class="space-y-2">
                  <div>
                    <label class="text-xs text-gray-500" for="start-date"
                      >From</label
                    >
                    <input
                      id="start-date"
                      type="date"
                      bind:value={customStartInput}
                      class="w-full mt-0.5 px-2 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-300"
                      onblur={() => validateDateRange()}
                    />
                  </div>
                  <div>
                    <label class="text-xs text-gray-500" for="end-date"
                      >To</label
                    >
                    <input
                      id="end-date"
                      type="date"
                      bind:value={customEndInput}
                      class="w-full mt-0.5 px-2 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-300"
                      onblur={() => validateDateRange()}
                    />
                  </div>
                  <button
                    onclick={applyCustomRange}
                    disabled={!customStartInput ||
                      !customEndInput ||
                      customRangeLoading}
                    class="w-full mt-1 px-3 py-1.5 bg-orange-500 text-white rounded-lg text-sm font-medium hover:bg-orange-600 disabled:opacity-50 transition-colors"
                  >
                    Apply
                  </button>
                </div>
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- Custom range chip -->
      {#if customRangeLabel()}
        <div class="flex items-center gap-2">
          <span
            class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-sm bg-orange-50 text-orange-700 border border-orange-200"
          >
            📅 {customRangeLabel()}
            <button
              onclick={clearCustomRange}
              class="hover:text-orange-900 font-bold">×</button
            >
          </span>
        </div>
      {/if}

      <!-- Active filter chips -->
      {#if hasActiveFilters}
        <div class="flex flex-wrap items-center gap-2">
          <span class="text-xs text-gray-500 font-medium">Filtered by:</span>
          {#each activeCountries as code}
            <span
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-sm bg-blue-50 text-blue-700 border border-blue-200"
            >
              {countryFlag(code)}
              {countryCodeToName(code)}
              <button
                onclick={() => toggleCountry(code)}
                class="hover:text-blue-900 font-bold">×</button
              >
            </span>
          {/each}
          {#each activeReferrers as ref}
            <span
              class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-sm bg-purple-50 text-purple-700 border border-purple-200"
            >
              🔗 {ref}
              <button
                onclick={() => toggleReferrer(ref)}
                class="hover:text-purple-900 font-bold">×</button
              >
            </span>
          {/each}
          <button
            onclick={clearAllFilters}
            class="text-xs text-gray-400 hover:text-gray-600 underline"
            >Clear all</button
          >
        </div>
      {/if}

      {#if analytics}
        <!-- Clicks over time chart -->
        <div class="bg-white rounded-xl border-2 border-gray-200 p-6">
          <h2 class="text-lg font-semibold text-gray-900 mb-4">
            Clicks Over Time
          </h2>
          {#if analytics.clicks_over_time.length > 0}
            <div class="h-64">
              <canvas bind:this={clicksChartCanvas}></canvas>
            </div>
          {:else}
            <div
              class="h-64 flex items-center justify-center text-gray-400 text-sm px-4"
            >
              No click data for this period
            </div>
          {/if}
        </div>

        <!-- Summary stat pills -->
        <div class="grid grid-cols-3 gap-4">
          <div class="bg-white rounded-xl border border-gray-200 p-4">
            <p
              class="text-xs text-gray-500 font-medium uppercase tracking-wide"
            >
              Total clicks {timeRangeLabel}
            </p>
            <p class="text-2xl font-bold text-gray-900 mt-1">
              {analytics.total_clicks.toLocaleString()}
            </p>
          </div>
          <div class="bg-white rounded-xl border border-gray-200 p-4">
            <p
              class="text-xs text-gray-500 font-medium uppercase tracking-wide"
            >
              Links clicked {timeRangeLabel}
            </p>
            <p class="text-2xl font-bold text-gray-900 mt-1">
              {analytics.unique_links_clicked.toLocaleString()}
            </p>
          </div>
          <div class="bg-white rounded-xl border border-gray-200 p-4">
            <p
              class="text-xs text-gray-500 font-medium uppercase tracking-wide"
            >
              Avg. Clicks / Link
            </p>
            <p class="text-2xl font-bold text-gray-900 mt-1">
              {avgClicksPerLink()}
            </p>
          </div>
        </div>

        <!-- Free tier teaser -->
        {#if analytics.analytics_gated && !hasTierAccess("pro")}
          <div
            class="bg-gradient-to-r from-orange-50 to-amber-50 border border-orange-200 rounded-xl p-5 flex flex-col sm:flex-row items-center justify-between gap-4"
          >
            <div>
              <p class="font-semibold text-orange-900">
                You're viewing 7 days of data
              </p>
              <p class="text-sm text-orange-700 mt-0.5">
                Upgrade to Pro to unlock up to 1 year of analytics history,
                custom date ranges, and more.
              </p>
            </div>
            <a
              href="/pricing"
              class="flex-shrink-0 px-4 py-2 bg-orange-500 text-white rounded-lg font-semibold text-sm hover:bg-orange-600 transition-colors shadow-sm"
              >Upgrade to Pro →</a
            >
          </div>
        {/if}

        <!-- Top links + Top referrers (two column) -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div class="bg-white rounded-xl border border-gray-200 p-5">
            <h2 class="text-sm font-semibold text-gray-700 mb-4">Top links</h2>
            {#if analytics.top_links.length > 0}
              <div class="space-y-1">
                {#each analytics.top_links as link, i}
                  <button
                    class="w-full flex items-center justify-between gap-3 px-3 py-2 rounded-lg text-sm hover:bg-gray-50 transition-colors group text-left"
                    onclick={() => (slideOverLink = link)}
                  >
                    <div class="flex items-center gap-2 min-w-0">
                      <span class="text-xs text-gray-400 w-4 flex-shrink-0"
                        >{i + 1}</span
                      >
                      <div class="min-w-0">
                        <span
                          class="font-medium text-orange-600 group-hover:underline block truncate"
                          >/{link.short_code}</span
                        >
                        {#if link.title}
                          <span class="text-xs text-gray-400 block truncate"
                            >{link.title}</span
                          >
                        {/if}
                      </div>
                    </div>
                    <div class="flex items-center gap-2 flex-shrink-0">
                      <div
                        class="w-16 h-1.5 bg-gray-100 rounded-full overflow-hidden"
                      >
                        <div
                          class="h-full bg-orange-400 rounded-full"
                          style="width: {barWidth(
                            link.count,
                            analytics.top_links
                          )}%"
                        ></div>
                      </div>
                      <span class="text-gray-700 font-medium w-12 text-right"
                        >{link.count.toLocaleString()}</span
                      >
                    </div>
                  </button>
                {/each}
              </div>
            {:else}
              <p class="text-sm text-gray-400 py-4 text-center">
                No data for this period
              </p>
            {/if}
          </div>

          <div class="bg-white rounded-xl border border-gray-200 p-5">
            <h2 class="text-sm font-semibold text-gray-700 mb-4">
              Top referrers
            </h2>
            {#if analytics.top_referrers.length > 0}
              <div class="space-y-1">
                {#each analytics.top_referrers as ref}
                  <button
                    class="w-full flex items-center justify-between gap-3 px-3 py-2 rounded-lg text-sm transition-colors text-left {activeReferrers.includes(
                      ref.referrer
                    )
                      ? 'bg-purple-50 ring-1 ring-purple-200'
                      : 'hover:bg-gray-50'}"
                    onclick={() => toggleReferrer(ref.referrer)}
                  >
                    <span class="text-gray-700 truncate">{ref.referrer}</span>
                    <div class="flex items-center gap-2 flex-shrink-0">
                      <div
                        class="w-16 h-1.5 bg-gray-100 rounded-full overflow-hidden"
                      >
                        <div
                          class="h-full bg-blue-500 rounded-full"
                          style="width: {barWidth(
                            ref.count,
                            analytics.top_referrers
                          )}%"
                        ></div>
                      </div>
                      <span class="text-gray-700 font-medium w-12 text-right"
                        >{ref.count.toLocaleString()}</span
                      >
                    </div>
                  </button>
                {/each}
              </div>
            {:else}
              <p class="text-sm text-gray-400 py-4 text-center">
                No data for this period
              </p>
            {/if}
          </div>
        </div>

        <!-- Top countries -->
        <div class="bg-white rounded-xl border border-gray-200 p-5">
          <h2 class="text-sm font-semibold text-gray-700 mb-4">
            Top countries
          </h2>
          {#if analytics.top_countries.length > 0}
            {@const maxCount = analytics.top_countries[0]?.count ?? 1}
            <div class="space-y-1">
              {#each analytics.top_countries as country}
                <div
                  class="flex items-center justify-between gap-3 px-3 py-2 rounded-lg text-sm"
                >
                  <div class="flex items-center gap-2 text-gray-700 truncate">
                    <span class="text-base">{countryFlag(country.country)}</span
                    >
                    {countryCodeToName(country.country)}
                  </div>
                  <div class="flex items-center gap-2 flex-shrink-0">
                    <div
                      class="w-16 h-1.5 bg-gray-100 rounded-full overflow-hidden"
                    >
                      <div
                        class="h-full bg-green-500 rounded-full"
                        style="width: {(country.count / maxCount) * 100}%"
                      ></div>
                    </div>
                    <span class="text-gray-700 font-medium w-12 text-right"
                      >{country.count.toLocaleString()}</span
                    >
                  </div>
                </div>
              {/each}
            </div>
          {:else}
            <p class="text-sm text-gray-400 py-4 text-center">
              No data for this period
            </p>
          {/if}
        </div>

        <!-- Browser + OS doughnut charts -->
        <div class="space-y-4">
          <div class="bg-white rounded-xl border border-gray-200 p-5">
            <h2 class="text-sm font-semibold text-gray-700 mb-3">Browsers</h2>
            {#if browserData.length > 0}
              <div class="h-44">
                <canvas bind:this={browserChartCanvas}></canvas>
              </div>
            {:else}
              <div
                class="h-44 flex items-center justify-center text-gray-400 text-sm"
              >
                No data
              </div>
            {/if}
          </div>
          <div class="bg-white rounded-xl border border-gray-200 p-5">
            <h2 class="text-sm font-semibold text-gray-700 mb-3">
              Operating systems
            </h2>
            {#if osData.length > 0}
              <div class="h-44">
                <canvas bind:this={osChartCanvas}></canvas>
              </div>
            {:else}
              <div
                class="h-44 flex items-center justify-center text-gray-400 text-sm"
              >
                No data
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <!-- Loading skeleton -->
        <div class="space-y-4">
          {#each [1, 2, 3] as _}
            <div
              class="bg-white rounded-xl border border-gray-200 p-5 h-32 animate-pulse"
            ></div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- Per-link slide-over -->
{#if slideOverLink}
  <OrgLinkSlideOver
    link={slideOverLink}
    days={currentDays}
    onclose={() => (slideOverLink = null)}
  />
{/if}
