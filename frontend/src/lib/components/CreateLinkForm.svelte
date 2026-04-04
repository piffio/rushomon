<script lang="ts">
	import { linksApi } from "$lib/api/links";
	import type { Link, ApiError, UtmParams } from "$lib/types/api";
	import { fetchUrlTitle, debounce } from "$lib/utils/url-title";
	import { 
		DEFAULT_MIN_CUSTOM_CODE_LENGTH,
		MAX_SHORT_CODE_LENGTH
	} from "$lib/constants";

	let {
		onLinkCreated,
		isPro = false,
		minShortCodeLength = DEFAULT_MIN_CUSTOM_CODE_LENGTH,
	}: {
		onLinkCreated: (link: Link) => void;
		isPro?: boolean;
		minShortCodeLength?: number;
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

	// Pro features
	let showUtmBuilder = $state(false);
	let utmSource = $state("");
	let utmMedium = $state("");
	let utmCampaign = $state("");
	let utmTerm = $state("");
	let utmContent = $state("");
	let forwardQueryParams = $state(false);

	function hasUtmParams(): boolean {
		return !!(
			utmSource.trim() ||
			utmMedium.trim() ||
			utmCampaign.trim() ||
			utmTerm.trim() ||
			utmContent.trim()
		);
	}

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
			if (trimmedShortCode.length < minShortCodeLength || trimmedShortCode.length > MAX_SHORT_CODE_LENGTH) {
				error = `Custom code must be ${minShortCodeLength}-${MAX_SHORT_CODE_LENGTH} characters`;
				isSubmitting = false;
				return;
			}

			if (!/^[a-zA-Z0-9-/]+$/.test(trimmedShortCode)) {
				error =
					"Custom code can only contain letters, numbers, hyphens, and forward slashes";
			}

			if (
				trimmedShortCode.startsWith("-") ||
				trimmedShortCode.endsWith("-")
			) {
				error = "Custom code cannot start or end with a hyphen";
				isSubmitting = false;
				return;
			}

			if (
				trimmedShortCode.startsWith("/") ||
				trimmedShortCode.endsWith("/")
			) {
				error = "Custom code cannot start or end with a forward slash";
				isSubmitting = false;
				return;
			}

			if (trimmedShortCode.includes("//")) {
				error =
					"Custom code cannot contain consecutive forward slashes";
				isSubmitting = false;
				return;
			}

			// Segment validation
			const segments = trimmedShortCode.split("/");
			if (segments.length > 3) {
				error =
					"Custom code can have at most 3 segments separated by slashes";
				isSubmitting = false;
				return;
			}

			for (const segment of segments) {
				if (segment.length < 1 || segment.length > 50) {
					error = "Each segment must be 1-50 characters long";
					isSubmitting = false;
					return;
				}
				if (segment.startsWith("-") || segment.endsWith("-")) {
					error = "Segment cannot start or end with a hyphen";
					isSubmitting = false;
					return;
				}
			}
		}

		// Build UTM params if any filled
		const utmParams: UtmParams | undefined = hasUtmParams()
			? {
					utm_source: utmSource.trim() || undefined,
					utm_medium: utmMedium.trim() || undefined,
					utm_campaign: utmCampaign.trim() || undefined,
					utm_term: utmTerm.trim() || undefined,
					utm_content: utmContent.trim() || undefined,
				}
			: undefined;

		try {
			const link = await linksApi.create({
				destination_url: destinationUrl.trim(),
				short_code: shortCode.trim() || undefined,
				title: title.trim() || undefined,
				expires_at: expiresAt
					? Math.floor(new Date(expiresAt).getTime() / 1000)
					: undefined,
				utm_params: utmParams,
				forward_query_params:
					isPro && forwardQueryParams ? true : undefined,
				redirect_type: "301", // Default to 301 for CreateLinkForm
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
				utmSource = "";
				utmMedium = "";
				utmCampaign = "";
				utmTerm = "";
				utmContent = "";
				forwardQueryParams = false;
				showUtmBuilder = false;
				hasUserEnteredTitle = false;
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
			<label for="short-code" class="block text-sm font-medium text-gray-700 mb-1">
    			Custom Short Code
				<span class="text-gray-500 text-xs font-normal">
				(Optional, {minShortCodeLength}-{MAX_SHORT_CODE_LENGTH} characters: letters, numbers, hyphens, forward slashes)
			</span>
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

		<!-- Pro Features: UTM Builder + Query Forwarding -->
		{#if isPro}
			<!-- UTM Builder -->
			<div class="border border-gray-200 rounded-lg overflow-hidden">
				<button
					type="button"
					class="w-full flex items-center justify-between px-4 py-3 bg-gray-50 hover:bg-gray-100 transition-colors text-sm font-medium text-gray-700"
					onclick={() => (showUtmBuilder = !showUtmBuilder)}
				>
					<span class="flex items-center gap-2">
						<svg
							class="w-4 h-4 text-indigo-500"
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
						UTM Parameters
						{#if hasUtmParams()}
							<span
								class="bg-indigo-100 text-indigo-700 text-xs px-2 py-0.5 rounded-full"
								>active</span
							>
						{/if}
					</span>
					<svg
						class="w-4 h-4 text-gray-400 transition-transform {showUtmBuilder
							? 'rotate-180'
							: ''}"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M19 9l-7 7-7-7"
						/>
					</svg>
				</button>
				{#if showUtmBuilder}
					<div class="p-4 space-y-3 border-t border-gray-200">
						<p class="text-xs text-gray-500">
							These UTM parameters will be appended to the
							destination URL on every redirect.
						</p>
						<div class="grid grid-cols-2 gap-3">
							<div>
								<label
									for="utm-source"
									class="block text-xs font-medium text-gray-600 mb-1"
									>Source</label
								>
								<input
									type="text"
									id="utm-source"
									bind:value={utmSource}
									placeholder="e.g. newsletter"
									class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
								/>
							</div>
							<div>
								<label
									for="utm-medium"
									class="block text-xs font-medium text-gray-600 mb-1"
									>Medium</label
								>
								<input
									type="text"
									id="utm-medium"
									bind:value={utmMedium}
									placeholder="e.g. email"
									class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
								/>
							</div>
							<div>
								<label
									for="utm-campaign"
									class="block text-xs font-medium text-gray-600 mb-1"
									>Campaign</label
								>
								<input
									type="text"
									id="utm-campaign"
									bind:value={utmCampaign}
									placeholder="e.g. spring_sale"
									class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
								/>
							</div>
							<div>
								<label
									for="utm-term"
									class="block text-xs font-medium text-gray-600 mb-1"
									>Term</label
								>
								<input
									type="text"
									id="utm-term"
									bind:value={utmTerm}
									placeholder="e.g. running+shoes"
									class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
								/>
							</div>
							<div class="col-span-2">
								<label
									for="utm-content"
									class="block text-xs font-medium text-gray-600 mb-1"
									>Content</label
								>
								<input
									type="text"
									id="utm-content"
									bind:value={utmContent}
									placeholder="e.g. banner_top"
									class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
								/>
							</div>
						</div>
					</div>
				{/if}
			</div>

			<!-- Forward Query Params Toggle -->
			<div
				class="flex items-start gap-3 p-4 border border-gray-200 rounded-lg"
			>
				<div class="flex-1">
					<label
						for="forward-query-params"
						class="block text-sm font-medium text-gray-700"
					>
						Forward visitor query parameters
					</label>
					<p class="text-xs text-gray-500 mt-0.5">
						Append any query params from the short link URL to the
						destination. Visitor params override UTM params on
						conflict.
					</p>
				</div>
				<input
					type="checkbox"
					id="forward-query-params"
					bind:checked={forwardQueryParams}
					class="mt-0.5 h-4 w-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500"
				/>
			</div>
		{:else}
			<!-- Upsell for free tier -->
			<div
				class="flex items-center gap-2 p-3 bg-gray-50 border border-dashed border-gray-300 rounded-lg text-sm text-gray-500"
			>
				<svg
					class="w-4 h-4 text-amber-500 shrink-0"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M13 10V3L4 14h7v7l9-11h-7z"
					/>
				</svg>
				<span
					><strong class="text-gray-700">Pro feature:</strong> UTM
					parameters &amp; query forwarding —
					<a href="/billing" class="text-indigo-600 hover:underline"
						>Upgrade to Pro</a
					></span
				>
			</div>
		{/if}

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
