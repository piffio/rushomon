<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import LinkList from "$lib/components/LinkList.svelte";
	import LinkModal from "$lib/components/LinkModal.svelte";
	import Pagination from "$lib/components/Pagination.svelte";
	import { linksApi } from "$lib/api/links";
	import { goto } from "$app/navigation";
	import type { PageData } from "./$types";
	import type {
		Link,
		ApiError,
		PaginationMeta,
		UsageResponse,
	} from "$lib/types/api";

	let { data }: { data: PageData } = $props();

	let links = $state<Link[]>([]);
	let pagination = $state<PaginationMeta | null>(null);
	let stats = $state<{
		total_links: number;
		active_links: number;
		total_clicks: number;
	} | null>(null);
	let loading = $state(false);
	let error = $state("");
	let editingLink = $state<Link | null>(null);
	let isModalOpen = $state(false);
	let usage = $state<UsageResponse | null>(null);

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

	function handleEdit(link: Link) {
		editingLink = link;
		isModalOpen = true;
	}

	function handleCreateNew() {
		editingLink = null;
		isModalOpen = true;
	}

	function handleLinkSaved(event: CustomEvent<Link>) {
		const savedLink = event.detail;

		if (editingLink) {
			// Update existing link
			links = links.map((l) => (l.id === savedLink.id ? savedLink : l));
		} else {
			// Add new link to beginning
			links = [savedLink, ...links];
			// Update pagination total
			if (pagination && pagination.total !== undefined) {
				pagination = { ...pagination, total: pagination.total + 1 };
			}
		}
	}

	async function handleDelete(id: string) {
		error = "";
		try {
			await linksApi.delete(id);
			// Remove from list
			links = links.filter((link) => link.id !== id);
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to delete link";
		}
	}

	async function handlePageChange(page: number) {
		if (page < 1) return;

		loading = true;
		error = "";

		try {
			// Update URL with new page (enables browser back/forward and shareable URLs)
			await goto(`/dashboard?page=${page}`, {
				replaceState: true,
				keepFocus: true,
			});

			const paginatedLinks = await linksApi.list(page, 10);
			links = paginatedLinks.data;
			pagination = paginatedLinks.pagination;
			stats = paginatedLinks.stats || null;
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to load links";
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Dashboard - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
	{#if data.user}
		<Header user={data.user} currentPage="dashboard" />

		<!-- Header Section -->
		<div
			class="bg-gradient-to-br from-gray-50 to-gray-100 border-b border-gray-200"
		>
			<div class="max-w-6xl mx-auto px-6 py-8">
				<div class="flex items-center justify-between">
					<div>
						<h1
							class="text-3xl md:text-4xl font-bold text-gray-900"
						>
							Rushomon Links
						</h1>
						{#if pagination}
							<p class="text-gray-600 mt-2">
								Showing {(pagination.page - 1) *
									pagination.limit +
									1}–{Math.min(
									pagination.page * pagination.limit,
									pagination.total,
								)} of {pagination.total} link{pagination.total !==
								1
									? "s"
									: ""}
							</p>
						{/if}
					</div>
					<button
						onclick={handleCreateNew}
						disabled={linksAtLimit}
						class="bg-gradient-to-r from-orange-500 to-orange-600 text-white px-6 py-3 rounded-lg shadow-lg hover:shadow-xl transition-all duration-300 font-semibold disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:shadow-lg"
						title={linksAtLimit
							? "Monthly link limit reached"
							: "Create a new short link"}
					>
						+ New Link
					</button>
				</div>
			</div>
		</div>

		<!-- Stats Cards Section -->
		<div class="max-w-6xl mx-auto px-6 py-6">
			<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
				<!-- Total Links -->
				<div
					class="bg-white rounded-2xl border-2 border-gray-200 p-6 transition-all duration-300 hover:border-orange-500 hover:shadow-lg"
				>
					<div class="flex items-center gap-4">
						<div
							class="w-12 h-12 bg-orange-100 rounded-lg flex items-center justify-center flex-shrink-0"
						>
							<svg
								class="w-6 h-6 text-orange-600"
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
						</div>
						<div>
							<p class="text-gray-600 text-sm">Total Links</p>
							<p class="text-2xl font-bold text-gray-900">
								{stats?.total_links ?? 0}
							</p>
						</div>
					</div>
				</div>

				<!-- Total Clicks -->
				<div
					class="bg-white rounded-2xl border-2 border-gray-200 p-6 transition-all duration-300 hover:border-blue-500 hover:shadow-lg"
				>
					<div class="flex items-center gap-4">
						<div
							class="w-12 h-12 bg-blue-100 rounded-lg flex items-center justify-center flex-shrink-0"
						>
							<svg
								class="w-6 h-6 text-blue-600"
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
						</div>
						<div>
							<p class="text-gray-600 text-sm">Total Clicks</p>
							<p class="text-2xl font-bold text-gray-900">
								{stats?.total_clicks ?? 0}
							</p>
						</div>
					</div>
				</div>

				<!-- Active Links -->
				<div
					class="bg-white rounded-2xl border-2 border-gray-200 p-6 transition-all duration-300 hover:border-green-500 hover:shadow-lg"
				>
					<div class="flex items-center gap-4">
						<div
							class="w-12 h-12 bg-green-100 rounded-lg flex items-center justify-center flex-shrink-0"
						>
							<svg
								class="w-6 h-6 text-green-600"
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
						</div>
						<div>
							<p class="text-gray-600 text-sm">Active Links</p>
							<p class="text-2xl font-bold text-gray-900">
								{stats?.active_links ?? 0}
							</p>
						</div>
					</div>
				</div>
			</div>
		</div>

		<!-- Usage Indicators (free tier only) -->
		{#if usage && usage.tier === "free" && usage.limits.max_links_per_month}
			<div class="max-w-6xl mx-auto px-6">
				<div
					class="bg-white rounded-2xl border-2 p-6 {linksAtLimit
						? 'border-amber-300 bg-amber-50'
						: 'border-gray-200'}"
				>
					<div class="flex items-center justify-between mb-4">
						<div class="flex items-center gap-2">
							<h3
								class="text-sm font-semibold text-gray-700 uppercase tracking-wider"
							>
								Free Tier Usage
							</h3>
							<span
								class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800"
							>
								Free
							</span>
						</div>
						<a
							href="/pricing"
							class="text-sm text-orange-600 hover:text-orange-700 font-medium"
						>
							View plans &rarr;
						</a>
					</div>

					<div class="space-y-4">
						{#if usage.limits.max_links_per_month}
							<div>
								<div
									class="flex items-center justify-between mb-1.5"
								>
									<span class="text-sm text-gray-600"
										>Links created this month</span
									>
									<span
										class="text-sm font-medium {linksAtLimit
											? 'text-red-600'
											: 'text-gray-900'}"
									>
										{usage.usage.links_created_this_month} /
										{usage.limits.max_links_per_month}
									</span>
								</div>
								<div
									class="w-full bg-gray-200 rounded-full h-2"
								>
									<div
										class="h-2 rounded-full transition-all duration-500 {linksAtLimit
											? 'bg-red-500'
											: linksUsagePercent >= 80
												? 'bg-amber-500'
												: 'bg-orange-500'}"
										style="width: {linksUsagePercent}%"
									></div>
								</div>
								{#if linksAtLimit}
									<p class="text-xs text-red-600 mt-1">
										Monthly link limit reached. Limits reset
										on the 1st of each month.
									</p>
								{/if}
							</div>
						{/if}
					</div>
				</div>
			</div>
		{/if}

		<!-- Main Content Area -->
		<main class="max-w-6xl mx-auto px-6 py-6">
			<!-- Error Message -->
			{#if error}
				<div
					class="bg-red-50 border-2 border-red-200 text-red-700 px-6 py-4 rounded-2xl mb-6"
				>
					{error}
				</div>
			{/if}

			<!-- Links List -->
			<LinkList
				{links}
				{loading}
				onDelete={handleDelete}
				onEdit={handleEdit}
			/>

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
				on:saved={handleLinkSaved}
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
