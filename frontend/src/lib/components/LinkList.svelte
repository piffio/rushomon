<script lang="ts">
	import LinkCard from "./LinkCard.svelte";
	import type { Link } from "$lib/types/api";

	let {
		links,
		loading = false,
		isFiltered = false,
		onDelete,
		onEdit,
		onTagClick,
	}: {
		links: Link[];
		loading?: boolean;
		isFiltered?: boolean;
		onDelete: (id: string) => void;
		onEdit: (link: Link) => void;
		onTagClick?: (tag: string) => void;
	} = $props();
</script>

<div>
	{#if loading}
		<!-- Loading State -->
		<div class="space-y-4">
			{#each [1, 2, 3] as _}
				<div
					class="border-2 border-gray-200 rounded-2xl p-6 animate-pulse transition-all duration-300"
				>
					<div class="h-6 bg-gray-200 rounded w-1/3 mb-3"></div>
					<div class="h-4 bg-gray-200 rounded w-2/3 mb-2"></div>
					<div class="h-4 bg-gray-200 rounded w-1/2"></div>
				</div>
			{/each}
		</div>
	{:else if links.length === 0}
		{#if isFiltered}
			<!-- No Search Results State -->
			<div
				class="text-center py-16 bg-white rounded-2xl border-2 border-gray-200"
			>
				<div class="text-6xl mb-4">🔍</div>
				<h3 class="text-xl font-semibold text-gray-900 mb-2">
					No links match your search
				</h3>
				<p class="text-gray-600 mb-4">
					Try adjusting your search terms or filters.
				</p>
			</div>
		{:else}
			<!-- Empty State -->
			<div
				class="text-center py-16 bg-white rounded-2xl border-2 border-gray-200"
			>
				<div class="text-6xl mb-4">🔗</div>
				<h3 class="text-xl font-semibold text-gray-900 mb-2">
					No links yet
				</h3>
				<p class="text-gray-600">
					Create your first short link to get started!
				</p>
			</div>
		{/if}
	{:else}
		<!-- Links Grid -->
		<div class="space-y-4">
			{#each links as link (link.id)}
				<LinkCard {link} {onDelete} {onEdit} {onTagClick} />
			{/each}
		</div>
	{/if}
</div>
