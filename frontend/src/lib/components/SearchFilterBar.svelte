<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import type { TagWithCount } from "$lib/types/api";

	interface FilterState {
		search: string;
		status: "all" | "active" | "disabled";
		sort: "created" | "updated" | "clicks" | "title" | "code";
		tags: string[];
	}

	let {
		search = $bindable(""),
		status = $bindable("all"),
		sort = $bindable("created"),
		selectedTags = $bindable<string[]>([]),
		availableTags = [],
		resultCount = 0,
		totalCount = 0,
		currentPage = 1,
		pageSize = 10,
		isSearching = false,
	}: {
		search?: string;
		status?: "all" | "active" | "disabled";
		sort?: "created" | "updated" | "clicks" | "title" | "code";
		selectedTags?: string[];
		availableTags?: TagWithCount[];
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
	let showTagDropdown = $state(false);

	// Sync local search with prop
	$effect(() => {
		localSearch = search;
	});

	function dispatchChange() {
		dispatch("change", { search, status, sort, tags: selectedTags });
	}

	function handleSearchChange(value: string) {
		localSearch = value;
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}
		debounceTimer = setTimeout(() => {
			search = value;
			dispatchChange();
		}, 300);
	}

	function handleStatusChange(newStatus: "all" | "active" | "disabled") {
		status = newStatus;
		dispatchChange();
	}

	function handleSortChange(
		newSort: "created" | "updated" | "clicks" | "title" | "code",
	) {
		sort = newSort;
		dispatchChange();
	}

	function clearSearch() {
		localSearch = "";
		search = "";
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}
		dispatchChange();
	}

	function toggleTag(tag: string) {
		if (selectedTags.includes(tag)) {
			selectedTags = selectedTags.filter((t) => t !== tag);
		} else {
			selectedTags = [...selectedTags, tag];
		}
		dispatchChange();
	}

	function removeTag(tag: string) {
		selectedTags = selectedTags.filter((t) => t !== tag);
		dispatchChange();
	}

	function clearAllTags() {
		selectedTags = [];
		dispatchChange();
	}

	// Deterministic color from tag name
	const TAG_COLORS = [
		"bg-blue-100 text-blue-800",
		"bg-green-100 text-green-800",
		"bg-purple-100 text-purple-800",
		"bg-yellow-100 text-yellow-800",
		"bg-pink-100 text-pink-800",
		"bg-indigo-100 text-indigo-800",
		"bg-orange-100 text-orange-800",
		"bg-teal-100 text-teal-800",
	];

	function tagColor(tag: string): string {
		let hash = 0;
		for (let i = 0; i < tag.length; i++) {
			hash = (hash * 31 + tag.charCodeAt(i)) & 0xffffffff;
		}
		return TAG_COLORS[Math.abs(hash) % TAG_COLORS.length];
	}

	const unselectedTags = $derived(
		availableTags.filter((t) => !selectedTags.includes(t.name)),
	);

	const isFiltered = $derived(
		search.trim() !== "" || status !== "all" || selectedTags.length > 0,
	);
	const showFrom = $derived((currentPage - 1) * pageSize + 1);
	const showTo = $derived(Math.min(currentPage * pageSize, totalCount));
</script>

<div class="bg-white rounded-2xl border-2 border-gray-200 p-4 sm:p-6">
	<!-- Top row: search + filters -->
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
		<div class="flex flex-wrap gap-3 lg:gap-4 items-center">
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
					<option value="title">Title A-Z</option>
					<option value="code">Short Code A-Z</option>
				</select>
			</div>

			<!-- Tag Filter -->
			{#if availableTags.length > 0}
				<div class="flex items-center gap-2">
					<span class="text-sm text-gray-600 whitespace-nowrap"
						>Tags:</span
					>
					<div class="relative">
						<button
							type="button"
							onclick={() => (showTagDropdown = !showTagDropdown)}
							class="flex items-center gap-1.5 px-3 py-2 border-2 rounded-lg text-sm transition-colors {selectedTags.length >
							0
								? 'border-orange-500 bg-orange-50 text-orange-700'
								: 'border-gray-200 bg-white text-gray-700 hover:border-gray-300'}"
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
									d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
								/>
							</svg>
							{#if selectedTags.length > 0}
								{selectedTags.length} tag{selectedTags.length >
								1
									? "s"
									: ""}
							{:else}
								Filter
							{/if}
							<svg
								class="w-3 h-3 ml-0.5"
								fill="none"
								stroke="currentColor"
								viewBox="0 0 24 24"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2.5"
									d={showTagDropdown
										? "M5 15l7-7 7 7"
										: "M19 9l-7 7-7-7"}
								/>
							</svg>
						</button>

						{#if showTagDropdown}
							<div
								class="absolute right-0 top-full mt-1 z-20 bg-white border-2 border-gray-200 rounded-xl shadow-lg min-w-[200px] max-h-64 overflow-y-auto"
							>
								{#if availableTags.length === 0}
									<p class="px-3 py-2 text-sm text-gray-400">
										No tags yet
									</p>
								{:else}
									{#each availableTags as tag (tag.name)}
										<button
											type="button"
											class="w-full flex items-center justify-between px-3 py-2 text-sm hover:bg-gray-50 transition-colors text-left"
											onclick={() => toggleTag(tag.name)}
										>
											<span
												class="flex items-center gap-2"
											>
												{#if selectedTags.includes(tag.name)}
													<svg
														class="w-4 h-4 text-orange-500 flex-shrink-0"
														fill="currentColor"
														viewBox="0 0 20 20"
													>
														<path
															fill-rule="evenodd"
															d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
															clip-rule="evenodd"
														/>
													</svg>
												{:else}
													<span
														class="w-4 h-4 flex-shrink-0 border-2 border-gray-300 rounded"
													></span>
												{/if}
												<span
													class="inline-block w-2 h-2 rounded-full flex-shrink-0 {tagColor(
														tag.name,
													).split(' ')[0]}"
												></span>
												<span class="truncate"
													>{tag.name}</span
												>
											</span>
											<span
												class="text-xs text-gray-400 ml-2 flex-shrink-0"
												>{tag.count}</span
											>
										</button>
									{/each}
									{#if selectedTags.length > 0}
										<div
											class="border-t border-gray-100 px-3 py-2"
										>
											<button
												type="button"
												class="text-xs text-orange-600 hover:text-orange-700 font-medium"
												onclick={clearAllTags}
											>
												Clear all tags
											</button>
										</div>
									{/if}
								{/if}
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	</div>

	<!-- Selected tag chips -->
	{#if selectedTags.length > 0}
		<div class="mt-3 flex flex-wrap gap-1.5">
			{#each selectedTags as tag (tag)}
				<span
					class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-medium {tagColor(
						tag,
					)}"
				>
					{tag}
					<button
						type="button"
						onclick={() => removeTag(tag)}
						class="hover:opacity-70 transition-opacity"
						aria-label="Remove tag filter {tag}"
					>
						<svg
							class="w-3 h-3"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2.5"
								d="M6 18L18 6M6 6l12 12"
							/>
						</svg>
					</button>
				</span>
			{/each}
		</div>
	{/if}

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
				Searching...
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
				{#if selectedTags.length > 0}
					<span class="text-gray-400"
						>· {selectedTags.length} tag{selectedTags.length > 1
							? "s"
							: ""}</span
					>
				{/if}
			</span>
		{:else}
			<span class="text-gray-500">
				Showing <span class="font-medium text-gray-900"
					>{showFrom}-{showTo}</span
				>
				of <span class="font-medium text-gray-900">{totalCount}</span>
				{totalCount === 1 ? "link" : "links"}
			</span>
		{/if}
	</div>
</div>
