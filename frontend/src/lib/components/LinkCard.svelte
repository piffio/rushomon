<script lang="ts">
	import type { Link } from '$lib/types/api';
	import { PUBLIC_VITE_SHORT_LINK_BASE_URL } from '$env/static/public';

	let {
		link,
		onDelete,
		onEdit
	}: {
		link: Link;
		onDelete: (id: string) => void;
		onEdit: (link: Link) => void;
	} = $props();

	const SHORT_LINK_BASE = PUBLIC_VITE_SHORT_LINK_BASE_URL || 'http://localhost:8787';
	const shortUrl = $derived(`${SHORT_LINK_BASE}/${link.short_code}`);

	let showDeleteConfirm = $state(false);
	let copySuccess = $state(false);

	async function copyToClipboard() {
		try {
			await navigator.clipboard.writeText(shortUrl);
			copySuccess = true;
			setTimeout(() => (copySuccess = false), 2000);
		} catch (err) {
			console.error('Failed to copy:', err);
		}
	}

	function formatDate(timestamp: number): string {
		return new Date(timestamp * 1000).toLocaleDateString();
	}

	function truncateUrl(url: string, maxLength: number = 50): string {
		if (url.length <= maxLength) return url;
		return url.substring(0, maxLength) + '...';
	}
</script>

<div class="border border-gray-200 rounded-lg p-4 hover:shadow-md transition-shadow">
	<!-- Short URL with Copy Button -->
	<div class="flex items-center justify-between mb-3">
		<div class="flex items-center gap-2 flex-1 min-w-0">
			<a
				href={shortUrl}
				target="_blank"
				rel="noopener noreferrer"
				class="text-lg font-semibold text-gray-900 hover:text-gray-700 truncate"
			>
				{link.short_code}
			</a>
			{#if link.status === 'disabled'}
				<span class="flex-shrink-0 px-2 py-0.5 text-xs bg-gray-200 text-gray-700 rounded">
					Disabled
				</span>
			{/if}
			<button
				onclick={copyToClipboard}
				class="flex-shrink-0 px-3 py-1 text-sm bg-gray-100 hover:bg-gray-200 rounded transition-colors"
				title="Copy to clipboard"
			>
				{copySuccess ? '✓ Copied' : '📋 Copy'}
			</button>
		</div>
	</div>

	<!-- Title (if set) -->
	{#if link.title}
		<p class="text-sm font-medium text-gray-700 mb-2">{link.title}</p>
	{/if}

	<!-- Destination URL -->
	<a
		href={link.destination_url}
		target="_blank"
		rel="noopener noreferrer"
		class="text-sm text-gray-600 hover:text-gray-900 block mb-3"
		title={link.destination_url}
	>
		→ {truncateUrl(link.destination_url)}
	</a>

	<!-- Stats and Actions -->
	<div class="flex items-center justify-between text-sm text-gray-500">
		<div class="flex items-center gap-4 flex-wrap">
			<!-- Click Count -->
			<div class="flex items-center gap-1">
				<span class="font-semibold text-gray-900">{link.click_count}</span>
				<span>click{link.click_count !== 1 ? 's' : ''}</span>
			</div>

			<!-- Created Date -->
			<div>Created {formatDate(link.created_at)}</div>

			<!-- Expiration Date (if set) -->
			{#if link.expires_at}
				<div class="flex items-center gap-1">
					<span>Expires {formatDate(link.expires_at)}</span>
					{#if link.expires_at * 1000 < Date.now()}
						<span class="text-red-600 font-medium">⚠ Expired</span>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Action Buttons -->
		<div class="flex items-center gap-2">
			<!-- Edit Button -->
			<button
				onclick={() => onEdit(link)}
				class="px-3 py-1 text-gray-700 hover:bg-gray-100 rounded transition-colors"
				title="Edit link"
			>
				Edit
			</button>

			<!-- Delete Button -->
			<div class="relative">
				{#if showDeleteConfirm}
					<div class="absolute right-0 bottom-full mb-2 bg-white border border-gray-200 rounded-lg shadow-lg p-3 z-10 min-w-[200px]">
						<p class="text-sm text-gray-700 mb-3">Delete this link?</p>
						<div class="flex gap-2">
							<button
								onclick={() => {
									showDeleteConfirm = false;
									onDelete(link.id);
								}}
								class="flex-1 px-3 py-1 bg-red-600 text-white rounded hover:bg-red-700 text-sm"
							>
								Delete
							</button>
							<button
								onclick={() => (showDeleteConfirm = false)}
								class="flex-1 px-3 py-1 bg-gray-200 text-gray-700 rounded hover:bg-gray-300 text-sm"
							>
								Cancel
							</button>
						</div>
					</div>
				{/if}
				<button
					onclick={() => (showDeleteConfirm = !showDeleteConfirm)}
					class="px-3 py-1 text-red-600 hover:bg-red-50 rounded transition-colors"
				>
					Delete
				</button>
			</div>
		</div>
	</div>
</div>
