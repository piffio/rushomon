<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import type { Link, LinkStatus } from "$lib/types/api";
	import { linksApi } from "$lib/api/links";
	import { fetchUrlTitle, debounce } from "$lib/utils/url-title";

	interface Props {
		link?: Link | null;
		isOpen?: boolean;
	}

	let { link = null, isOpen = $bindable(false) }: Props = $props();

	const dispatch = createEventDispatcher<{ saved: Link }>();

	// Determine mode
	const isEditMode = $derived(!!link);
	const modalTitle = $derived(
		isEditMode ? "Edit Link" : "Create New Short Link",
	);

	// Form state
	let destinationUrl = $state("");
	let shortCode = $state("");
	let title = $state("");
	let expiresAt = $state("");
	let status = $state<LinkStatus>("active");

	let loading = $state(false);
	let error = $state("");
	let isFetchingTitle = $state(false);
	let hasUserEnteredTitle = $state(false);

	// Reset form fields
	function resetForm() {
		destinationUrl = "";
		shortCode = "";
		title = "";
		expiresAt = "";
		status = "active";
		error = "";
		isFetchingTitle = false;
		hasUserEnteredTitle = false;
	}

	// Update form when link prop changes (for edit mode) OR when modal opens
	$effect(() => {
		if (isOpen) {
			if (link) {
				// Edit mode: populate with link data
				destinationUrl = link.destination_url;
				shortCode = link.short_code;
				title = link.title || "";
				expiresAt = link.expires_at
					? new Date(link.expires_at * 1000)
							.toISOString()
							.slice(0, 16)
					: "";
				status = link.status;
				error = "";
			} else {
				// Create mode: reset form
				resetForm();
			}
		}
	});

	function handleClose() {
		if (!loading) {
			isOpen = false;
			resetForm();
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			handleClose();
		}
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();
		loading = true;
		error = "";

		try {
			const linkData: any = {
				destination_url: destinationUrl,
				title: title || undefined,
				expires_at: expiresAt
					? Math.floor(new Date(expiresAt).getTime() / 1000)
					: undefined,
			};

			let savedLink: Link;

			if (isEditMode && link) {
				// Update existing link
				linkData.status = status;
				savedLink = await linksApi.update(link.id, linkData);
			} else {
				// Create new link
				if (shortCode) {
					linkData.short_code = shortCode;
				}
				savedLink = await linksApi.create(linkData);
			}

			// Dispatch event with saved link
			dispatch("saved", savedLink);

			// Reset form and close modal
			resetForm();
			isOpen = false;
		} catch (err: any) {
			error = err.message || "An error occurred";
		} finally {
			loading = false;
		}
	}

	// Debounced function to fetch title when URL changes (only in create mode)
	const fetchTitleForUrl = debounce(async (url: string) => {
		// Don't fetch if in edit mode, user has already entered a title, or if URL is invalid
		if (
			isEditMode ||
			hasUserEnteredTitle ||
			!url.trim() ||
			(!url.startsWith("http://") && !url.startsWith("https://"))
		) {
			return;
		}

		isFetchingTitle = true;

		try {
			const fetchedTitle = await fetchUrlTitle(url.trim());
			// Only set the title if user hasn't entered one and we got a valid title
			if (!hasUserEnteredTitle && fetchedTitle) {
				title = fetchedTitle;
			}
		} catch (err) {
			// Silently handle errors - title fetching is optional
			console.debug("Failed to fetch URL title:", err);
		} finally {
			isFetchingTitle = false;
		}
	}, 500); // 500ms debounce delay

	// Handle URL changes (only in create mode)
	$effect(() => {
		if (destinationUrl && !isEditMode) {
			fetchTitleForUrl(destinationUrl);
		}
	});

	// Handle manual title changes
	$effect(() => {
		// Mark that user has entered a title if it's not empty
		if (title.trim()) {
			hasUserEnteredTitle = true;
		}
	});
</script>

{#if isOpen}
	<div
		class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4"
		onclick={handleBackdropClick}
		role="dialog"
		aria-modal="true"
		aria-labelledby="modal-title"
	>
		<div
			class="bg-white rounded-2xl shadow-2xl max-w-2xl w-full max-h-[90vh] overflow-y-auto"
			onclick={(e) => e.stopPropagation()}
		>
			<!-- Modal Header -->
			<div
				class="border-b border-gray-200 px-6 py-4 flex justify-between items-center"
			>
				<h2
					id="modal-title"
					class="text-xl font-semibold text-gray-900"
				>
					{modalTitle}
				</h2>
				<button
					onclick={handleClose}
					disabled={loading}
					class="text-gray-400 hover:text-gray-600 transition-colors disabled:opacity-50"
					aria-label="Close modal"
				>
					<svg
						class="w-6 h-6"
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
			</div>

			<!-- Modal Body -->
			<form onsubmit={handleSubmit} class="px-6 py-6 space-y-6">
				{#if error}
					<div
						class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm"
					>
						{error}
					</div>
				{/if}

				<!-- Destination URL -->
				<div>
					<label
						for="destination-url"
						class="block text-sm font-medium text-gray-700 mb-2"
					>
						Destination URL <span class="text-red-500">*</span>
					</label>
					<input
						id="destination-url"
						type="url"
						bind:value={destinationUrl}
						required
						placeholder="https://example.com/very/long/url"
						class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent"
						disabled={loading}
					/>
				</div>

				<!-- Short Code -->
				<div>
					<label
						for="short-code"
						class="block text-sm font-medium text-gray-700 mb-2"
					>
						{#if isEditMode}
							Short Code (Read-only)
						{:else}
							Custom Short Code (Optional)
						{/if}
					</label>
					<input
						id="short-code"
						type="text"
						bind:value={shortCode}
						disabled={isEditMode || loading}
						placeholder={isEditMode ? "" : "my-link"}
						class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed"
					/>
					<p class="text-xs text-gray-500 mt-1">
						{#if isEditMode}
							Short codes cannot be changed after creation
						{:else}
							4-10 alphanumeric characters. Leave empty for
							auto-generated code
						{/if}
					</p>
				</div>

				<!-- Title -->
				<div>
					<label
						for="title"
						class="block text-sm font-medium text-gray-700 mb-2"
					>
						Title (Optional)
					</label>
					<input
						id="title"
						type="text"
						bind:value={title}
						placeholder="My Awesome Link"
						maxlength="200"
						class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed"
						disabled={loading || isFetchingTitle}
					/>
					{#if isFetchingTitle && !isEditMode}
						<p
							class="text-xs text-gray-500 mt-1 flex items-center gap-1"
						>
							<svg
								class="animate-spin h-3 w-3"
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
							Fetching title...
						</p>
					{:else if hasUserEnteredTitle}
						<p class="text-xs text-gray-500 mt-1">
							Custom title entered
						</p>
					{:else if !isEditMode}
						<p class="text-xs text-gray-500 mt-1">
							Title will be fetched automatically
						</p>
					{:else}
						<p class="text-xs text-gray-500 mt-1">
							Edit title manually
						</p>
					{/if}
				</div>

				<!-- Expiration Date -->
				<div>
					<label
						for="expires-at"
						class="block text-sm font-medium text-gray-700 mb-2"
					>
						Expiration Date (Optional)
					</label>
					<input
						id="expires-at"
						type="datetime-local"
						bind:value={expiresAt}
						class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent"
						disabled={loading}
					/>
					<p class="text-xs text-gray-500 mt-1">
						Leave empty for links that never expire
					</p>
				</div>

				<!-- Status (Edit mode only) -->
				{#if isEditMode}
					<div>
						<label
							for="status"
							class="block text-sm font-medium text-gray-700 mb-2"
						>
							Status
						</label>
						<select
							id="status"
							bind:value={status}
							disabled={loading}
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent disabled:opacity-50"
						>
							<option value="active"
								>Active - Link redirects normally</option
							>
							<option value="disabled"
								>Disabled - Link returns 404</option
							>
						</select>
						<p class="text-xs text-gray-500 mt-1">
							Disabled links don't redirect but keep the short
							code reserved
						</p>
					</div>
				{/if}

				<!-- Action Buttons -->
				<div class="flex gap-3 pt-4">
					<button
						type="button"
						onclick={handleClose}
						disabled={loading}
						class="flex-1 px-6 py-3 border border-gray-300 rounded-lg text-gray-700 font-medium hover:bg-gray-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
					>
						Cancel
					</button>
					<button
						type="submit"
						disabled={loading}
						class="flex-1 px-6 py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm hover:shadow-md disabled:opacity-50 disabled:cursor-not-allowed"
					>
						{loading
							? "Saving..."
							: isEditMode
								? "Save Changes"
								: "Create Short Link"}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}
