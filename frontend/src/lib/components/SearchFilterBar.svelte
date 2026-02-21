<script lang="ts">
	import { createEventDispatcher } from "svelte";

	interface FilterState {
		search: string;
		status: "all" | "active" | "disabled";
		sort: "created" | "updated" | "clicks" | "title" | "code";
	}

	let {
		search = $bindable(""),
		status = $bindable("all"),
		sort = $bindable("created"),
		resultCount = 0,
		totalCount = 0,
		currentPage = 1,
		pageSize = 10,
		isSearching = false,
	}: {
		search?: string;
		status?: "all" | "active" | "disabled";
		sort?: "created" | "updated" | "clicks" | "title" | "code";
		resultCount?: number;
		totalCount?: number;
		currentPage?: number;
		pageSize?: number;
		isSearching?: boolean;
	} = $props();

	const dispatch = createEventDispatcher<{
		change: FilterState;
	}>();

	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let localSearch = $state(search);

	// Sync local search with prop
	$effect(() => {
		localSearch = search;
	});

	function handleSearchChange(value: string) {
		localSearch = value;
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}
		debounceTimer = setTimeout(() => {
			search = value;
			dispatch("change", { search, status, sort });
		}, 300);
	}

	function handleStatusChange(newStatus: "all" | "active" | "disabled") {
		status = newStatus;
		dispatch("change", { search, status, sort });
	}

	function handleSortChange(
		newSort: "created" | "updated" | "clicks" | "title" | "code",
	) {
		sort = newSort;
		dispatch("change", { search, status, sort });
	}

	function clearSearch() {
		localSearch = "";
		search = "";
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}
		dispatch("change", { search, status, sort });
	}

	const isFiltered = $derived(search.trim() !== "" || status !== "all");
	const showFrom = $derived((currentPage - 1) * pageSize + 1);
	const showTo = $derived(Math.min(currentPage * pageSize, totalCount));
</script>

<div class="bg-white rounded-2xl border-2 border-gray-200 p-4 sm:p-6">
	<div
		class="flex flex-col lg:flex-row gap-4 lg:items-center lg:justify-between"
	>
		<!-- Search Input -->
		<div class="flex-1 min-w-0">
			<div class="relative">
				<div
					class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none"
				>
					<svg
						class="w-5 h-5 text-gray-400"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
						/>
					</svg>
				</div>
				<input
					type="text"
					value={localSearch}
					oninput={(e) => handleSearchChange(e.currentTarget.value)}
					placeholder="Search links by title, code, or URL..."
					class="w-full pl-10 pr-10 py-3 border-2 border-gray-200 rounded-xl focus:border-orange-500 focus:outline-none transition-colors"
					maxlength="100"
				/>
				{#if localSearch}
					<button
						onclick={clearSearch}
						title="Clear search"
						class="absolute inset-y-0 right-0 pr-3 flex items-center text-gray-400 hover:text-gray-600 transition-colors"
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
				{/if}
			</div>
		</div>

		<!-- Filters Row -->
		<div class="flex flex-col sm:flex-row gap-3 lg:gap-4">
			<!-- Status Filter -->
			<div class="flex items-center gap-2">
				<span class="text-sm text-gray-600 whitespace-nowrap"
					>Status:</span
				>
				<div class="flex bg-gray-100 rounded-lg p-1">
					<button
						onclick={() => handleStatusChange("all")}
						class="px-3 py-1.5 text-sm rounded-md transition-colors {status ===
						'all'
							? 'bg-white text-gray-900 shadow-sm'
							: 'text-gray-600 hover:text-gray-900'}"
					>
						All
					</button>
					<button
						onclick={() => handleStatusChange("active")}
						class="px-3 py-1.5 text-sm rounded-md transition-colors {status ===
						'active'
							? 'bg-green-100 text-green-800 shadow-sm'
							: 'text-gray-600 hover:text-gray-900'}"
					>
						Active
					</button>
					<button
						onclick={() => handleStatusChange("disabled")}
						class="px-3 py-1.5 text-sm rounded-md transition-colors {status ===
						'disabled'
							? 'bg-gray-200 text-gray-800 shadow-sm'
							: 'text-gray-600 hover:text-gray-900'}"
					>
						Disabled
					</button>
				</div>
			</div>

			<!-- Sort Dropdown -->
			<div class="flex items-center gap-2">
				<span class="text-sm text-gray-600 whitespace-nowrap"
					>Sort:</span
				>
				<select
					value={sort}
					onchange={(e) =>
						handleSortChange(
							e.currentTarget.value as FilterState["sort"],
						)}
					class="px-3 py-2 border-2 border-gray-200 rounded-lg text-sm focus:border-orange-500 focus:outline-none bg-white min-w-[140px]"
				>
					<option value="created">Newest First</option>
					<option value="updated">Recently Updated</option>
					<option value="clicks">Most Clicks</option>
					<option value="title">Title A–Z</option>
					<option value="code">Short Code A–Z</option>
				</select>
			</div>
		</div>
	</div>

	<!-- Count row -->
	<div
		class="mt-3 pt-3 border-t border-gray-100 flex items-center gap-2 text-sm"
	>
		{#if isSearching}
			<span class="text-gray-400 flex items-center gap-1.5">
				<svg
					class="animate-spin w-3.5 h-3.5"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<circle
						class="opacity-25"
						cx="12"
						cy="12"
						r="10"
						stroke="currentColor"
						stroke-width="4"
						fill="none"
					/>
					<path
						class="opacity-75"
						fill="currentColor"
						d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
					/>
				</svg>
				Searching…
			</span>
		{:else if totalCount === 0}
			<span class="text-gray-400">No links</span>
		{:else if isFiltered}
			<span class="text-gray-500">
				<span class="font-medium text-gray-900">{totalCount}</span>
				{totalCount === 1 ? "result" : "results"}
				{#if search}
					for "<span class="font-medium text-gray-900">{search}</span
					>"
				{/if}
				{#if status !== "all"}
					<span class="text-gray-400"
						>({status === "active"
							? "active only"
							: "disabled only"})</span
					>
				{/if}
			</span>
		{:else}
			<span class="text-gray-500">
				Showing <span class="font-medium text-gray-900"
					>{showFrom}–{showTo}</span
				>
				of <span class="font-medium text-gray-900">{totalCount}</span>
				{totalCount === 1 ? "link" : "links"}
			</span>
		{/if}
	</div>
</div>
