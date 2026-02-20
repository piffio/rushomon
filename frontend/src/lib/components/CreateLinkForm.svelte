<script lang="ts">
	import { linksApi } from "$lib/api/links";
	import type { Link, ApiError } from "$lib/types/api";
	import { fetchUrlTitle, debounce } from "$lib/utils/url-title";

	let {
		onLinkCreated,
	}: {
		onLinkCreated: (link: Link) => void;
	} = $props();

	let destinationUrl = $state("");
	let shortCode = $state("");
	let title = $state("");
	let expiresAt = $state("");
	let isSubmitting = $state(false);
	let error = $state("");
	let success = $state(false);
	let isFetchingTitle = $state(false);
	let hasUserEnteredTitle = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = "";
		success = false;
		isSubmitting = true;

		// Validation
		if (!destinationUrl.trim()) {
			error = "Destination URL is required";
			isSubmitting = false;
			return;
		}

		if (
			!destinationUrl.startsWith("http://") &&
			!destinationUrl.startsWith("https://")
		) {
			error = "URL must start with http:// or https://";
			isSubmitting = false;
			return;
		}

		// Trim and validate short code
		const trimmedShortCode = shortCode.trim();
		if (trimmedShortCode) {
			if (trimmedShortCode.length < 4 || trimmedShortCode.length > 10) {
				error = "Custom code must be 4-10 characters";
				isSubmitting = false;
				return;
			}

			if (!/^[a-zA-Z0-9]+$/.test(trimmedShortCode)) {
				error = "Custom code can only contain letters and numbers";
				isSubmitting = false;
				return;
			}
		}

		try {
			const link = await linksApi.create({
				destination_url: destinationUrl.trim(),
				short_code: shortCode.trim() || undefined,
				title: title.trim() || undefined,
				expires_at: expiresAt
					? Math.floor(new Date(expiresAt).getTime() / 1000)
					: undefined,
			});

			// Success!
			success = true;
			onLinkCreated(link);

			// Clear form
			setTimeout(() => {
				destinationUrl = "";
				shortCode = "";
				title = "";
				expiresAt = "";
				success = false;
			}, 2000);
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to create link";
		} finally {
			isSubmitting = false;
		}
	}

	// Debounced function to fetch title when URL changes
	const fetchTitleForUrl = debounce(async (url: string) => {
		// Don't fetch if user has already entered a title or if URL is invalid
		if (
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

	// Handle URL changes
	$effect(() => {
		if (destinationUrl) {
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

<div class="bg-white rounded-lg shadow-md p-6 mb-8">
	<h2 class="text-xl font-semibold text-gray-900 mb-4">
		Create New Short Link
	</h2>

	<form onsubmit={handleSubmit} class="space-y-4">
		<!-- Destination URL -->
		<div>
			<label
				for="destination-url"
				class="block text-sm font-medium text-gray-700 mb-1"
			>
				Destination URL <span class="text-red-500">*</span>
			</label>
			<input
				type="url"
				id="destination-url"
				bind:value={destinationUrl}
				placeholder="https://example.com/very/long/url"
				required
				class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
			/>
		</div>

		<!-- Custom Short Code -->
		<div>
			<label
				for="short-code"
				class="block text-sm font-medium text-gray-700 mb-1"
			>
				Custom Short Code
				<span class="text-gray-500 text-xs font-normal"
					>(Optional, 4-10 alphanumeric characters)</span
				>
			</label>
			<input
				type="text"
				id="short-code"
				bind:value={shortCode}
				placeholder="my-link"
				class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
			/>
		</div>

		<!-- Title -->
		<div>
			<label
				for="title"
				class="block text-sm font-medium text-gray-700 mb-1"
			>
				Title <span class="text-gray-500 text-xs font-normal"
					>(Optional)</span
				>
			</label>
			<input
				type="text"
				id="title"
				bind:value={title}
				placeholder="My Awesome Link"
				maxlength="200"
				class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed"
				disabled={isFetchingTitle}
			/>
			{#if isFetchingTitle}
				<p class="text-xs text-gray-500 mt-1 flex items-center gap-1">
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
				<p class="text-xs text-gray-500 mt-1">Custom title entered</p>
			{:else}
				<p class="text-xs text-gray-500 mt-1">
					Title will be fetched automatically
				</p>
			{/if}
		</div>

		<!-- Expiration Date -->
		<div>
			<label
				for="expires-at"
				class="block text-sm font-medium text-gray-700 mb-1"
			>
				Expiration Date <span class="text-gray-500 text-xs font-normal"
					>(Optional)</span
				>
			</label>
			<input
				type="datetime-local"
				id="expires-at"
				bind:value={expiresAt}
				class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
			/>
			<p class="text-xs text-gray-500 mt-1">
				Leave empty for links that never expire
			</p>
		</div>

		<!-- Error Message -->
		{#if error}
			<div
				class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm"
			>
				{error}
			</div>
		{/if}

		<!-- Success Message -->
		{#if success}
			<div
				class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm"
			>
				✓ Link created successfully!
			</div>
		{/if}

		<!-- Submit Button -->
		<button
			type="submit"
			disabled={isSubmitting}
			class="w-full bg-gray-900 text-white px-6 py-3 rounded-lg font-semibold hover:bg-gray-800 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
		>
			{isSubmitting ? "Creating..." : "Create Short Link"}
		</button>
	</form>
</div>
