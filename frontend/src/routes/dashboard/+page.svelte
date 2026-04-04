<script lang="ts">
	import LinkList from "$lib/components/LinkList.svelte";
	import LinkModal from "$lib/components/LinkModal.svelte";
	import QRCodeModal from "$lib/components/QRCodeModal.svelte";
	import Pagination from "$lib/components/Pagination.svelte";
	import SearchFilterBar from "$lib/components/SearchFilterBar.svelte";
	import { linksApi, tagsApi } from "$lib/api/links";
	import { goto, invalidate } from "$app/navigation";
	import { onDestroy, onMount } from "svelte";
	import type { PageData } from "./$types";
	import type {
		Link,
		ApiError,
		PaginationMeta,
		UsageResponse,
		TagWithCount,
		PaginatedResponse,
	} from "$lib/types/api";
  import { DEFAULT_MIN_CUSTOM_CODE_LENGTH, DEFAULT_SYSTEM_MIN_CODE_LENGTH } from "$lib/constants";

	let { data }: { data: PageData } = $props();

	let links = $state<Link[]>([]);
	let pagination = $state<PaginationMeta | null>(null);
	let stats = $state<{
		total_links: number;
		active_links: number;
		total_clicks: number;
	} | null>(null);
	let loading = $state(false);
	let isSearching = $state(false);
	let error = $state<string>("");
	let editingLink = $state<Link | null>(null);
	let isModalOpen = $state(false);
	let selectedQRLink = $state<Link | null>(null);
	let isQRModalOpen = $state(false);
	let usage = $state<UsageResponse | null>(null);
	let orgLogoUrl = $state<string | null>(null);
	let orgId = $state<string>("");
	let isActionsMenuOpen = $state(false);
	let isExporting = $state(false);
	let deletingLinkId = $state<string | null>(null);
	let recentlySavedLinkId = $state<string | null>(null);

	let highlightTimer: ReturnType<typeof setTimeout>;

	// Filter states - initialize from data props
	let search = $state<string>("");
	let status = $state<"all" | "active" | "disabled">("all");
	let sort = $state<"created" | "updated" | "clicks" | "title" | "code">(
		"created",
	);
	let selectedTags = $state<string[]>([]);
	let availableTags = $state<TagWithCount[]>([]);

	const effectiveMinLength = $derived(
		(data as any).publicSettings?.min_custom_code_length || DEFAULT_MIN_CUSTOM_CODE_LENGTH
	);

	// Initialize from data props using derived
	$effect(() => {
		search = (data as any).initialSearch || "";
		status = (data as any).initialStatus || "all";
		sort = (data as any).initialSort || "created";
		selectedTags = (data as any).initialTags || [];
		orgId = (data as any).orgId || "";
	});

	onMount(async () => {
		try {
			availableTags = await tagsApi.list();
		} catch {
			// Non-critical
		}
	});

	// Initialize links, pagination, and stats from data (runs on mount and when data changes)
	$effect(() => {
		if (data.paginatedLinks) {
			links = [...data.paginatedLinks.data];
			pagination = data.paginatedLinks.pagination;
			stats = data.paginatedLinks.stats || null;
		} else {
			links = [];
			pagination = null;
			stats = null;
		}
		const d = data as Record<string, any>;
		usage = (d.usage as UsageResponse) || null;
		orgLogoUrl = (d as any).orgLogoUrl ?? null;
		// Update filter states from data
		search = d.initialSearch || "";
		status = d.initialStatus || "all";
		sort = d.initialSort || "created";
		selectedTags = d.initialTags || [];
	});

	let linksUsagePercent = $derived(
		usage?.limits.max_links_per_month
			? Math.min(
					100,
					Math.round(
						(usage.usage.links_created_this_month /
							usage.limits.max_links_per_month) *
							100,
					),
				)
			: 0,
	);
	let linksAtLimit = $derived(
		usage?.limits.max_links_per_month
			? usage.usage.links_created_this_month >=
				usage.limits.max_links_per_month
			: false,
	);

	// Calculate reset countdown
	type ResetInfo = { text: string; date: string } | null;
	let resetInfo = $state<ResetInfo>(null);

	$effect(() => {
		if (!usage?.next_reset?.timestamp) {
			resetInfo = null;
			return;
		}
		const now = Date.now() / 1000;
		const diffSeconds = usage.next_reset.timestamp - now;

		if (diffSeconds <= 0) {
			// Reset time passed, refresh the page to get new usage data
			setTimeout(() => window.location.reload(), 5000);
			resetInfo = { text: "Refreshing...", date: "" };
			return;
		}

		const diffDays = Math.floor(diffSeconds / (60 * 60 * 24));
		const diffHours = Math.floor((diffSeconds % (60 * 60 * 24)) / (60 * 60));
		const diffMinutes = Math.floor((diffSeconds % (60 * 60)) / 60);

		let text = "";
		if (diffDays > 0) {
			text = `in ${diffDays}d ${diffHours}h`;
		} else if (diffHours > 0) {
			text = `in ${diffHours}h ${diffMinutes}m`;
		} else {
			text = `in ${diffMinutes}m`;
		}

		// Format reset date in user's local timezone
		const resetDate = new Date(usage.next_reset.timestamp * 1000);
		const dateStr = resetDate.toLocaleDateString(undefined, {
			month: "short",
			day: "numeric",
		});

		resetInfo = { text, date: dateStr };
	});

	// Helper to refresh all dashboard data (stats, usage, links)
	async function refreshDashboardData() {
		await invalidate("app:dashboard");
		// Also refresh tags
		try {
			availableTags = await tagsApi.list();
		} catch {
			// Non-critical
		}
	}

	function handleEdit(link: Link) {
		editingLink = link;
		isModalOpen = true;
	}

	function handleCreateNew() {
		editingLink = null;
		isModalOpen = true;
	}

	async function handleLinkSaved(event: CustomEvent<Link>) {
		const savedLink = event.detail;
		if (editingLink) {
			// Update existing link in place
			links = links.map((l) => (l.id === savedLink.id ? savedLink : l));
			refreshDashboardData();
		} else {
			// New link created! 
			// Clear all filters and jump to page 1 so it appears at the top
			search = "";
			status = "all";
			sort = "created";
			selectedTags = [];

			await goto("/dashboard", { invalidateAll: true });
		}

		// Highlight the saved link temporarily
		recentlySavedLinkId = savedLink.id;
		if (highlightTimer) clearTimeout(highlightTimer); 
		highlightTimer = setTimeout(() => {
			recentlySavedLinkId = null;
		}, 2500);
	}

	onDestroy(() => {
		if (highlightTimer) clearTimeout(highlightTimer);
	});

	async function handleDelete(id: string) {
		deletingLinkId = id;
		error = "";
		try {
			await linksApi.delete(id);
			// Remove from list
			links = links.filter((link) => link.id !== id);

			// Refresh all dashboard data to update stats
			refreshDashboardData();
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to delete link";
		} finally {
			deletingLinkId = null;
		}
	}

	async function handlePageChange(page: number) {
		if (page < 1) return;

		loading = true;
		isSearching = true;
		error = "";

		try {
			const params = new URLSearchParams();
			params.set("page", page.toString());
			if (search.trim()) params.set("search", search.trim());
			if (status !== "all") params.set("status", status);
			if (sort !== "created") params.set("sort", sort);
			if (selectedTags.length > 0)
				params.set("tags", selectedTags.join(","));
			const queryString = params.toString();

			await goto(`/dashboard${queryString ? `?${queryString}` : ""}`, {
				replaceState: true,
				keepFocus: true,
			});

			const paginatedLinks = await linksApi.list(
				page,
				10,
				search || undefined,
				status === "all" ? undefined : status,
				sort,
				selectedTags.length > 0 ? selectedTags : undefined,
			);
			links = paginatedLinks.data;
			pagination = paginatedLinks.pagination;
			stats = paginatedLinks.stats || null;
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to load links";
		} finally {
			loading = false;
			isSearching = false;
		}
	}

	async function handleFilterChange(
		event: CustomEvent<{
			search: string;
			status: "all" | "active" | "disabled";
			sort: "created" | "updated" | "clicks" | "title" | "code";
			tags: string[];
		}>,
	) {
		const {
			search: newSearch,
			status: newStatus,
			sort: newSort,
			tags: newTags,
		} = event.detail;

		search = newSearch;
		status = newStatus;
		sort = newSort;
		// selectedTags is bindable, no need to reassign
		isSearching = true;
		error = "";

		try {
			const params = new URLSearchParams();
			params.set("page", "1");
			if (search.trim()) params.set("search", search.trim());
			if (status !== "all") params.set("status", status);
			if (sort !== "created") params.set("sort", sort);
			if (selectedTags.length > 0) {
				params.set("tags", selectedTags.join(","));
			}
			const queryString = params.toString();

			await goto(`/dashboard${queryString ? `?${queryString}` : ""}`, {
				replaceState: true,
				keepFocus: true,
			});

			const paginatedLinks = await linksApi.list(
				1,
				10,
				search || undefined,
				status === "all" ? undefined : status,
				sort,
				selectedTags.length > 0 ? selectedTags : undefined,
			);
			links = paginatedLinks.data;
			pagination = paginatedLinks.pagination;
			stats = paginatedLinks.stats || null;
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to load links";
		} finally {
			isSearching = false;
		}
	}

	function handleShowQR(link: Link) {
		selectedQRLink = link;
		isQRModalOpen = true;
	}

	function handleCloseQR() {
		isQRModalOpen = false;
		selectedQRLink = null;
	}

	function handleTagClick(tag: string) {
		if (!selectedTags.includes(tag)) {
			selectedTags = [...selectedTags, tag];
			handleFilterChange(
				new CustomEvent("change", {
					detail: { search, status, sort, tags: selectedTags },
				}),
			);
		}
	}

	async function handleExport() {
		isActionsMenuOpen = false;
		isExporting = true;
		error = "";
		try {
			const blob = await linksApi.export();
			const url = URL.createObjectURL(blob);
			const a = document.createElement("a");
			const date = new Date().toISOString().slice(0, 10);
			a.href = url;
			a.download = `rushomon-links-${date}.csv`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to export links";
		} finally {
			isExporting = false;
		}
	}
</script>

<svelte:head>
	<title>Dashboard - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
	{#if data.user}
		<!-- Slim Title Bar -->
		<div class="border-b border-gray-200 bg-white">
			<div
				class="max-w-6xl mx-auto px-6 py-4 flex items-center justify-between"
			>
				<h1 class="text-xl font-semibold text-gray-900">My Links</h1>
				<div class="flex items-center gap-2">
					<button
						onclick={handleCreateNew}
						disabled={linksAtLimit}
						class="bg-gradient-to-r from-orange-500 to-orange-600 text-white px-5 py-2 rounded-lg shadow hover:shadow-md transition-all duration-200 font-semibold text-sm disabled:opacity-50 disabled:cursor-not-allowed"
						title={linksAtLimit
							? "Monthly link limit reached"
							: "Create a new short link"}
					>
						+ New Link
					</button>

					<!-- Actions dropdown (export / import) -->
					<div class="relative">
						<button
							onclick={() =>
								(isActionsMenuOpen = !isActionsMenuOpen)}
							class="p-2 rounded-lg border border-gray-200 bg-white text-gray-600 hover:bg-gray-50 hover:text-gray-900 transition-colors"
							title="More actions"
							aria-label="More actions"
						>
							<svg
								class="w-5 h-5"
								fill="currentColor"
								viewBox="0 0 20 20"
							>
								<path
									d="M6 10a2 2 0 11-4 0 2 2 0 014 0zM12 10a2 2 0 11-4 0 2 2 0 014 0zM16 12a2 2 0 100-4 2 2 0 000 4z"
								/>
							</svg>
						</button>

						{#if isActionsMenuOpen}
							<!-- Click-outside overlay -->
							<button
								class="fixed inset-0 z-10 cursor-default bg-transparent border-0 p-0"
								onclick={() => (isActionsMenuOpen = false)}
								aria-label="Close menu"
								tabindex="-1"
							></button>
							<div
								class="absolute right-0 mt-1 w-48 bg-white border border-gray-200 rounded-xl shadow-lg z-20 overflow-hidden"
							>
								<button
									onclick={handleExport}
									disabled={isExporting}
									class="w-full flex items-center gap-2 px-4 py-2.5 text-sm text-gray-700 hover:bg-gray-50 transition-colors disabled:opacity-50"
								>
									{#if isExporting}
										<svg
											class="animate-spin h-4 w-4 text-gray-500"
											viewBox="0 0 24 24"
											fill="none"
										>
											<circle
												class="opacity-25"
												cx="12"
												cy="12"
												r="10"
												stroke="currentColor"
												stroke-width="4"
											/>
											<path
												class="opacity-75"
												fill="currentColor"
												d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
											/>
										</svg>
									{:else}
										<svg
											class="w-4 h-4 text-gray-500"
											fill="none"
											stroke="currentColor"
											viewBox="0 0 24 24"
										>
											<path
												stroke-linecap="round"
												stroke-linejoin="round"
												stroke-width="2"
												d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
											/>
										</svg>
									{/if}
									{isExporting
										? "Exporting…"
										: "Export as CSV"}
								</button>
								<a
									href="/dashboard/import"
									onclick={() => (isActionsMenuOpen = false)}
									class="flex items-center gap-2 px-4 py-2.5 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
								>
									<svg
										class="w-4 h-4 text-gray-500"
										fill="none"
										stroke="currentColor"
										viewBox="0 0 24 24"
									>
										<path
											stroke-linecap="round"
											stroke-linejoin="round"
											stroke-width="2"
											d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l4-4m0 0l4 4m-4-4v12"
										/>
									</svg>
									Import from CSV
								</a>
							</div>
						{/if}
					</div>
				</div>
			</div>
		</div>

		<!-- Stats + Free Tier Strip -->
		<div class="border-b border-gray-200 bg-white">
			<div
				class="max-w-6xl mx-auto px-6 py-3 flex flex-wrap items-center gap-x-5 gap-y-2"
			>
				<!-- Stat pills -->
				<span class="flex items-center gap-1.5 text-sm text-gray-600">
					<svg
						class="w-4 h-4 text-orange-500"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"
						/>
					</svg>
					<span class="font-semibold text-gray-900"
						>{stats?.total_links ?? 0}</span
					> links
				</span>
				<span class="text-gray-300 hidden sm:inline">·</span>
				<span class="flex items-center gap-1.5 text-sm text-gray-600">
					<svg
						class="w-4 h-4 text-blue-500"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M15 15l-2 5L9 9l11 4-5 2zm0 0l5 5M7.188 2.239l.777 2.897M5.136 7.965l-2.898-.777M13.95 4.05l-2.122 2.122m-5.657 5.656l-2.12 2.122"
						/>
					</svg>
					<span class="font-semibold text-gray-900"
						>{stats?.total_clicks ?? 0}</span
					> clicks
				</span>
				<span class="text-gray-300 hidden sm:inline">·</span>
				<span class="flex items-center gap-1.5 text-sm text-gray-600">
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
					<span class="font-semibold text-gray-900"
						>{stats?.active_links ?? 0}</span
					> active
				</span>

				<!-- Usage counter for tiers with limits -->
				{#if usage && usage.limits.max_links_per_month}
					<span class="text-gray-300 hidden sm:inline">·</span>
					<span class="flex flex-col gap-1 text-sm">
						<span class="flex items-center gap-2">
							<span class="text-gray-500">This month:</span>
							<span
								class="font-semibold {linksAtLimit
									? 'text-red-600'
									: 'text-gray-900'}"
							>
								{usage.usage.links_created_this_month} / {usage
									.limits.max_links_per_month}
							</span>
							<div
								class="w-20 bg-gray-200 rounded-full h-1.5 hidden sm:block"
							>
								<div
									class="h-1.5 rounded-full transition-all duration-500 {linksAtLimit
										? 'bg-red-500'
										: linksUsagePercent >= 80
											? 'bg-amber-500'
											: 'bg-orange-500'}"
									style="width: {linksUsagePercent}%"
								></div>
							</div>
							{#if linksAtLimit}
								<span class="text-xs text-red-600 font-medium"
									>Limit reached</span
								>
							{/if}
							{#if usage.tier === "free"}
								<a
									href="/pricing"
									class="text-xs text-orange-600 hover:text-orange-700 font-medium"
									>Upgrade →</a
								>
							{/if}
						</span>
						{#if resetInfo}
							<span class="text-xs text-gray-400">
								Resets {resetInfo.date} UTC (00:00) {resetInfo.text}
							</span>
						{/if}
					</span>
				{/if}
			</div>
		</div>

		<!-- Main Content Area -->
		<main class="max-w-6xl mx-auto px-6 py-4">
			<!-- Error Message -->
			{#if error}
				<div
					class="bg-red-50 border-2 border-red-200 text-red-700 px-6 py-4 rounded-2xl mb-6"
				>
					{error}
				</div>
			{/if}

			<!-- Search and Filter Bar -->
			<SearchFilterBar
				bind:search
				bind:status
				bind:sort
				bind:selectedTags
				{availableTags}
				resultCount={links.length}
				totalCount={pagination?.total ?? 0}
				currentPage={pagination?.page ?? 1}
				pageSize={pagination?.limit ?? 10}
				{isSearching}
				on:change={handleFilterChange}
			/>

			<!-- Links List -->
			<div class="mt-6">
				<LinkList
					{links}
					{loading}
					deletingLinkId={deletingLinkId}
					recentlySavedLinkId={recentlySavedLinkId}
					isFiltered={search.trim() !== "" ||
						status !== "all" ||
						selectedTags.length > 0}
					onDelete={handleDelete}
					onEdit={handleEdit}
					onTagClick={handleTagClick}
					onShowQR={handleShowQR}
				/>
			</div>

			<!-- Pagination -->
			{#if pagination && pagination.total_pages > 1}
				<div class="mt-8">
					<Pagination
						currentPage={pagination.page}
						totalPages={pagination.total_pages}
						onPageChange={handlePageChange}
						{loading}
					/>
				</div>
			{/if}

			<!-- Link Modal (Create/Edit) -->
			<LinkModal
				link={editingLink}
				bind:isOpen={isModalOpen}
				{usage}
				minShortCodeLength={effectiveMinLength}
				on:saved={handleLinkSaved}
			/>

			<!-- QR Code Modal -->
			<QRCodeModal
				link={selectedQRLink}
				isOpen={isQRModalOpen}
				onClose={handleCloseQR}
				tier={usage?.tier ?? "free"}
				{orgLogoUrl}
				{orgId}
			/>
		</main>
	{:else}
		<!-- User data not available, this should not happen with proper auth -->
		<div class="min-h-screen bg-gray-50 flex items-center justify-center">
			<div class="text-center">
				<p class="text-gray-600 mb-4">Authentication required</p>
				<a href="/" class="text-orange-600 hover:text-orange-700"
					>Return to homepage</a
				>
			</div>
		</div>
	{/if}
</div>
