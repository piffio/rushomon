<script lang="ts">
	let {
		currentPage,
		totalPages,
		onPageChange,
		loading = false
	}: {
		currentPage: number;
		totalPages: number;
		onPageChange: (page: number) => void;
		loading?: boolean;
	} = $props();

	// Calculate visible page numbers
	const visiblePages = $derived(() => {
		if (totalPages <= 7) {
			// Show all pages if 7 or fewer
			return Array.from({ length: totalPages }, (_, i) => i + 1);
		}

		const pages: (number | string)[] = [];

		// Always show first page
		pages.push(1);

		if (currentPage <= 3) {
			// Near start: [1] [2] [3] [4] [5] ... [last]
			for (let i = 2; i <= 5; i++) {
				pages.push(i);
			}
			pages.push('...');
			pages.push(totalPages);
		} else if (currentPage >= totalPages - 2) {
			// Near end: [1] ... [last-4] [last-3] [last-2] [last-1] [last]
			pages.push('...');
			for (let i = totalPages - 4; i <= totalPages; i++) {
				pages.push(i);
			}
		} else {
			// Middle: [1] ... [current-1] [current] [current+1] ... [last]
			pages.push('...');
			pages.push(currentPage - 1);
			pages.push(currentPage);
			pages.push(currentPage + 1);
			pages.push('...');
			pages.push(totalPages);
		}

		return pages;
	});

	function handlePageClick(page: number) {
		if (page !== currentPage && !loading) {
			onPageChange(page);
		}
	}
</script>

<nav class="flex items-center justify-center gap-2" aria-label="Pagination">
	<!-- Previous Button -->
	<button
		onclick={() => handlePageClick(currentPage - 1)}
		disabled={currentPage <= 1 || loading}
		class="px-4 py-2 rounded-lg transition-all duration-300 flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed {currentPage <=
		1 || loading
			? 'bg-gray-100 text-gray-400'
			: 'bg-gray-100 hover:bg-gray-200 text-gray-700'}"
		aria-label="Previous page"
	>
		<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
		</svg>
		Previous
	</button>

	<!-- Page Numbers -->
	<div class="flex items-center gap-2">
		{#each visiblePages() as page}
			{#if page === '...'}
				<span class="px-3 py-2 text-gray-500">...</span>
			{:else}
				<button
					onclick={() => handlePageClick(page as number)}
					disabled={loading}
					class="px-4 py-2 rounded-lg transition-all duration-300 disabled:opacity-50 {page ===
					currentPage
						? 'bg-gradient-to-r from-orange-500 to-orange-600 text-white shadow-lg'
						: 'bg-gray-100 hover:bg-gray-200 text-gray-700'}"
					aria-label="Go to page {page}"
					aria-current={page === currentPage ? 'page' : undefined}
				>
					{page}
				</button>
			{/if}
		{/each}
	</div>

	<!-- Next Button -->
	<button
		onclick={() => handlePageClick(currentPage + 1)}
		disabled={currentPage >= totalPages || loading}
		class="px-4 py-2 rounded-lg transition-all duration-300 flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed {currentPage >=
		totalPages || loading
			? 'bg-gray-100 text-gray-400'
			: 'bg-gray-100 hover:bg-gray-200 text-gray-700'}"
		aria-label="Next page"
	>
		Next
		<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
			<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
		</svg>
	</button>

	<!-- Loading Spinner -->
	{#if loading}
		<div class="ml-2" role="status" aria-label="Loading">
			<svg
				class="animate-spin h-5 w-5 text-orange-600"
				xmlns="http://www.w3.org/2000/svg"
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
					d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
				></path>
			</svg>
		</div>
	{/if}
</nav>
