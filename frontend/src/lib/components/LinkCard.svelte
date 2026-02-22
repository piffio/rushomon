<script lang="ts">
	import type { Link } from "$lib/types/api";
	import {
		PUBLIC_VITE_API_BASE_URL,
		PUBLIC_VITE_SHORT_LINK_BASE_URL,
	} from "$env/static/public";

	let {
		link,
		onDelete,
		onEdit,
		onTagClick,
	}: {
		link: Link;
		onDelete: (id: string) => void;
		onEdit: (link: Link) => void;
		onTagClick?: (tag: string) => void;
	} = $props();

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

	const MAX_VISIBLE_TAGS = 3;
	const visibleTags = $derived(link.tags?.slice(0, MAX_VISIBLE_TAGS) ?? []);
	const hiddenTagCount = $derived(
		Math.max(0, (link.tags?.length ?? 0) - MAX_VISIBLE_TAGS),
	);

	const SHORT_LINK_BASE =
		PUBLIC_VITE_SHORT_LINK_BASE_URL ||
		PUBLIC_VITE_API_BASE_URL ||
		"http://localhost:8787";
	const shortUrl = $derived(`${SHORT_LINK_BASE}/${link.short_code}`);

	let showDeleteConfirm = $state(false);
	let copySuccess = $state(false);

	async function copyToClipboard() {
		try {
			await navigator.clipboard.writeText(shortUrl);
			copySuccess = true;
			setTimeout(() => (copySuccess = false), 2000);
		} catch (err) {
			console.error("Failed to copy:", err);
		}
	}

	function formatDate(timestamp: number): string {
		return new Date(timestamp * 1000).toLocaleDateString();
	}

	// Extract domain from URL for cleaner display
	function getDomain(url: string): string {
		try {
			const urlObj = new URL(url);
			return urlObj.hostname.replace("www.", "");
		} catch {
			return url;
		}
	}
</script>

<div
	class="border-2 border-gray-200 hover:border-orange-500 rounded-xl p-5 transition-all duration-300 hover:shadow-lg bg-white"
>
	<!-- Header: Title + Actions -->
	<div class="flex items-start justify-between gap-4 mb-2">
		<div class="flex-1 min-w-0">
			<!-- Title as Main Element (or short code if no title) -->
			<h3 class="text-lg font-semibold text-gray-900 truncate mb-1">
				{link.title || link.short_code}
			</h3>

			<!-- Short Link → Destination URL -->
			<div
				class="flex items-center gap-2 text-sm text-gray-600 flex-wrap"
			>
				<a
					href={shortUrl}
					target="_blank"
					rel="noopener noreferrer"
					class="font-medium text-orange-600 hover:text-orange-700 hover:underline"
				>
					{link.short_code}
				</a>
				<span class="text-gray-400">→</span>
				<a
					href={link.destination_url}
					target="_blank"
					rel="noopener noreferrer"
					class="text-gray-600 hover:text-gray-900 hover:underline truncate"
					title={link.destination_url}
				>
					{getDomain(link.destination_url)}
				</a>
			</div>
		</div>

		<!-- Action Buttons (Top Right) -->
		<div class="flex items-center gap-1 flex-shrink-0">
			<!-- Status Badge -->
			{#if link.status === "disabled"}
				<span
					class="px-2.5 py-1 text-xs font-medium bg-gray-200 text-gray-700 rounded-full mr-1"
				>
					Disabled
				</span>
			{/if}

			<!-- Analytics Button -->
			<a
				href="/dashboard/links/{link.short_code}"
				class="p-2 text-gray-600 hover:text-orange-600 hover:bg-orange-50 rounded-lg transition-colors"
				title="View analytics"
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
						d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
					/>
				</svg>
			</a>

			<!-- Edit Button -->
			<button
				onclick={() => onEdit(link)}
				class="p-2 text-gray-600 hover:text-orange-600 hover:bg-orange-50 rounded-lg transition-colors"
				title="Edit link"
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
						d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"
					/>
				</svg>
			</button>

			<!-- Delete Button -->
			<div class="relative">
				{#if showDeleteConfirm}
					<div
						class="absolute right-0 top-full mt-2 bg-white border-2 border-gray-200 rounded-xl shadow-xl p-4 z-10 min-w-[220px]"
					>
						<p class="text-sm font-medium text-gray-900 mb-2">
							Delete this link?
						</p>
						<p class="text-xs text-gray-600 mb-3">
							This action cannot be undone.
						</p>
						<div class="flex gap-2">
							<button
								onclick={() => {
									showDeleteConfirm = false;
									onDelete(link.id);
								}}
								class="flex-1 px-3 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 text-sm font-medium transition-colors"
							>
								Delete
							</button>
							<button
								onclick={() => (showDeleteConfirm = false)}
								class="flex-1 px-3 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 text-sm font-medium transition-colors"
							>
								Cancel
							</button>
						</div>
					</div>
				{/if}
				<button
					onclick={() => (showDeleteConfirm = !showDeleteConfirm)}
					class="p-2 text-gray-600 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
					title="Delete link"
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
							d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
						/>
					</svg>
				</button>
			</div>
		</div>
	</div>

	<!-- Tags Row -->
	{#if link.tags && link.tags.length > 0}
		<div class="flex flex-wrap gap-1.5 mt-3 mb-1">
			{#each visibleTags as tag (tag)}
				<button
					type="button"
					class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium transition-opacity hover:opacity-75 {tagColor(
						tag,
					)}"
					onclick={() => onTagClick?.(tag)}
					title={onTagClick ? `Filter by "${tag}"` : tag}
				>
					{tag}
				</button>
			{/each}
			{#if hiddenTagCount > 0}
				<span
					class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-600"
				>
					+{hiddenTagCount} more
				</span>
			{/if}
		</div>
	{/if}

	<!-- Stats Row with Copy Button -->
	<div
		class="flex items-center justify-between text-sm text-gray-500 pt-3 border-t border-gray-100"
	>
		<!-- Stats -->
		<div class="flex items-center gap-4 flex-wrap">
			<!-- Click Count -->
			<div class="flex items-center gap-1.5">
				<svg
					class="w-4 h-4 text-gray-400"
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
					>{link.click_count}</span
				>
			</div>

			<!-- Created Date -->
			<div class="flex items-center gap-1.5">
				<svg
					class="w-4 h-4 text-gray-400"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"
					/>
				</svg>
				<span>{formatDate(link.created_at)}</span>
			</div>

			<!-- Expiration Date (if set) -->
			{#if link.expires_at}
				<div
					class="flex items-center gap-1.5 {link.expires_at * 1000 <
					Date.now()
						? 'text-red-600'
						: ''}"
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
							d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
						/>
					</svg>
					<span>Expires {formatDate(link.expires_at)}</span>
					{#if link.expires_at * 1000 < Date.now()}
						<span class="font-medium">⚠</span>
					{/if}
				</div>
			{/if}
		</div>

		<!-- Copy Button (Right side) -->
		<button
			onclick={copyToClipboard}
			class="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-lg transition-all duration-300 {copySuccess
				? 'bg-green-100 text-green-700'
				: 'bg-gray-100 hover:bg-gray-200 text-gray-700'}"
			title="Copy short link"
		>
			{#if copySuccess}
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
						d="M5 13l4 4L19 7"
					/>
				</svg>
				<span>Copied</span>
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
				<span>Copy</span>
			{/if}
		</button>
	</div>
</div>
