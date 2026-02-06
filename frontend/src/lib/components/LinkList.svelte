<script lang="ts">
	import LinkCard from './LinkCard.svelte';
	import type { Link } from '$lib/types/api';

	let {
		links,
		loading = false,
		onDelete,
		onPageChange,
		currentPage = 1,
		hasMore = false
	}: {
		links: Link[];
		loading?: boolean;
		onDelete: (id: string) => void;
		onPageChange: (page: number) => void;
		currentPage?: number;
		hasMore?: boolean;
	} = $props();
</script>

<div>
	{#if loading}
		<!-- Loading State -->
		<div class="space-y-4">
			{#each [1, 2, 3] as _}
				<div class="border border-gray-200 rounded-lg p-4 animate-pulse">
					<div class="h-6 bg-gray-200 rounded w-1/3 mb-3"></div>
					<div class="h-4 bg-gray-200 rounded w-2/3 mb-2"></div>
					<div class="h-4 bg-gray-200 rounded w-1/2"></div>
				</div>
			{/each}
		</div>
	{:else if links.length === 0}
		<!-- Empty State -->
		<div class="text-center py-12">
			<div class="text-6xl mb-4">🔗</div>
			<h3 class="text-xl font-semibold text-gray-900 mb-2">No links yet</h3>
			<p class="text-gray-600">Create your first short link to get started!</p>
		</div>
	{:else}
		<!-- Links Grid -->
		<div class="space-y-4 mb-6">
			{#each links as link (link.id)}
				<LinkCard {link} {onDelete} />
			{/each}
		</div>

		<!-- Pagination -->
		{#if currentPage > 1 || hasMore}
			<div class="flex justify-between items-center">
				<button
					onclick={() => onPageChange(currentPage - 1)}
					disabled={currentPage === 1}
					class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
				>
					← Previous
				</button>

				<span class="text-sm text-gray-600">Page {currentPage}</span>

				<button
					onclick={() => onPageChange(currentPage + 1)}
					disabled={!hasMore}
					class="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
				>
					Next →
				</button>
			</div>
		{/if}
	{/if}
</div>
