<script lang="ts">
	import { createEventDispatcher, onMount } from "svelte";
	import type {
		Link,
		LinkStatus,
		TagWithCount,
		UsageResponse,
		UtmParams,
	} from "$lib/types/api";
	import { linksApi, tagsApi } from "$lib/api/links";
	import { fetchUrlTitle, debounce } from "$lib/utils/url-title";
	import TagInput from "$lib/components/TagInput.svelte";

	interface Props {
		link?: Link | null;
		isOpen?: boolean;
		usage?: UsageResponse | null;
	}

	let {
		link = null,
		isOpen = $bindable(false),
		usage = null,
	}: Props = $props();

	const dispatch = createEventDispatcher<{ saved: Link }>();

	// Determine mode and tier-based custom code availability
	const isEditMode = $derived(!!link);
	const allowCustomShortCode = $derived(
		usage?.limits?.allow_custom_short_code ?? false,
	);
	const isPro = $derived(
		(usage?.limits?.allow_custom_short_code ?? false) === true,
	);
	const modalTitle = $derived(
		isEditMode ? "Edit Link" : "Create New Short Link",
	);

	// Form state
	let destinationUrl = $state("");
	let shortCode = $state("");
	let title = $state("");
	let expiresAt = $state("");
	let status = $state<LinkStatus>("active");
	let tags = $state<string[]>([]);

	// Pro feature state
	let showUtmBuilder = $state(false);
	let utmSource = $state("");
	let utmMedium = $state("");
	let utmCampaign = $state("");
	let utmTerm = $state("");
	let utmContent = $state("");
	let utmRef = $state("");
	let forwardQueryParams = $state(false);

	function hasUtmParams(): boolean {
		return !!(
			utmSource.trim() ||
			utmMedium.trim() ||
			utmCampaign.trim() ||
			utmTerm.trim() ||
			utmContent.trim() ||
			utmRef.trim()
		);
	}

	function buildPreviewUrl(): string {
		if (!destinationUrl) return "";

		const params = [];
		if (utmSource.trim())
			params.push(`utm_source=${encodeURIComponent(utmSource.trim())}`);
		if (utmMedium.trim())
			params.push(`utm_medium=${encodeURIComponent(utmMedium.trim())}`);
		if (utmCampaign.trim())
			params.push(
				`utm_campaign=${encodeURIComponent(utmCampaign.trim())}`,
			);
		if (utmTerm.trim())
			params.push(`utm_term=${encodeURIComponent(utmTerm.trim())}`);
		if (utmContent.trim())
			params.push(`utm_content=${encodeURIComponent(utmContent.trim())}`);
		if (utmRef.trim())
			params.push(`utm_ref=${encodeURIComponent(utmRef.trim())}`);

		if (params.length === 0) return destinationUrl;

		const separator = destinationUrl.includes("?") ? "&" : "?";
		return destinationUrl + separator + params.join("&");
	}

	let loading = $state(false);
	let error = $state("");
	let isFetchingTitle = $state(false);
	let hasUserEnteredTitle = $state(false);
	let availableTags = $state<TagWithCount[]>([]);

	onMount(async () => {
		try {
			availableTags = await tagsApi.list();
		} catch {
			// Non-critical: autocomplete just won't show suggestions
		}
	});

	// Refresh available tags when modal opens
	$effect(() => {
		if (isOpen) {
			refreshAvailableTags();
		}
	});

	async function refreshAvailableTags() {
		try {
			availableTags = await tagsApi.list();
		} catch {
			// Non-critical: autocomplete just won't show suggestions
		}
	}

	// Reset form fields
	function resetForm() {
		destinationUrl = "";
		shortCode = "";
		title = "";
		expiresAt = "";
		status = "active";
		tags = [];
		error = "";
		isFetchingTitle = false;
		hasUserEnteredTitle = false;
		utmSource = "";
		utmMedium = "";
		utmCampaign = "";
		utmTerm = "";
		utmContent = "";
		utmRef = "";
		forwardQueryParams = false;
		showUtmBuilder = false;
		populatedLinkId = null;
	}

	// Track when we've populated the form to prevent re-population when user types
	let populatedLinkId = $state<string | null>(null);

	// Update form when link prop changes (for edit mode) OR when modal opens
	$effect(() => {
		if (isOpen) {
			if (link && link.id !== populatedLinkId) {
				// Edit mode: populate with link data (only if not already populated)
				destinationUrl = link.destination_url;
				shortCode = link.short_code;
				title = link.title || "";
				expiresAt = link.expires_at
					? new Date(link.expires_at * 1000)
							.toISOString()
							.slice(0, 16)
					: "";
				status = link.status;
				tags = [...(link.tags ?? [])];
				// UTM + forwarding
				utmSource = link.utm_params?.utm_source || "";
				utmMedium = link.utm_params?.utm_medium || "";
				utmCampaign = link.utm_params?.utm_campaign || "";
				utmTerm = link.utm_params?.utm_term || "";
				utmContent = link.utm_params?.utm_content || "";
				utmRef = link.utm_params?.utm_ref || "";
				showUtmBuilder = false; // Always start collapsed
				forwardQueryParams = link.forward_query_params ?? false;
				error = "";
				populatedLinkId = link.id;
			} else if (!link) {
				// Create mode: reset form
				resetForm();
				populatedLinkId = null;
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
			const utmParams: UtmParams | undefined = isPro
				? {
						utm_source: utmSource.trim() || undefined,
						utm_medium: utmMedium.trim() || undefined,
						utm_campaign: utmCampaign.trim() || undefined,
						utm_term: utmTerm.trim() || undefined,
						utm_content: utmContent.trim() || undefined,
						utm_ref: utmRef.trim() || undefined,
					}
				: undefined;

			const linkData: any = {
				destination_url: destinationUrl,
				title: title || undefined,
				expires_at: expiresAt
					? Math.floor(new Date(expiresAt).getTime() / 1000)
					: undefined,
				tags: tags.length > 0 ? tags : [],
				utm_params: utmParams,
				forward_query_params: isPro ? forwardQueryParams : undefined,
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
		role="button"
		tabindex="0"
		onkeydown={(e) => {
			if (e.key === "Escape") {
				e.preventDefault();
				handleClose();
			}
		}}
	>
		<div
			class="bg-white rounded-2xl shadow-2xl max-w-2xl w-full max-h-[90vh] overflow-y-auto"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			aria-labelledby="modal-title"
			tabindex="0"
			onkeydown={(e) => e.key === "Escape" && handleClose()}
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
						class="block text-sm font-medium text-gray-700 mb-2 flex items-center gap-2"
					>
						{#if isEditMode}
							Short Code (Read-only)
						{:else}
							Custom Short Code (Optional)
							{#if !allowCustomShortCode}
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
										d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
									/>
								</svg>
							{/if}
						{/if}
					</label>
					<input
						id="short-code"
						type="text"
						bind:value={shortCode}
						disabled={isEditMode ||
							loading ||
							!allowCustomShortCode}
						placeholder={isEditMode
							? ""
							: allowCustomShortCode
								? "my-link"
								: "Available on Pro"}
						class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-orange-500 focus:border-transparent disabled:bg-gray-100 disabled:cursor-not-allowed"
					/>
					<p class="text-xs text-gray-500 mt-1">
						{#if isEditMode}
							Short codes cannot be changed after creation
						{:else if !allowCustomShortCode}
							<a
								href="/pricing"
								class="text-orange-600 hover:text-orange-700 hover:underline"
								>Upgrade to Pro</a
							> to use custom short codes
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

				<!-- Tags -->
				<div>
					<p class="block text-sm font-medium text-gray-700 mb-2">
						Tags (Optional)
					</p>
					<TagInput
						bind:tags
						{availableTags}
						placeholder="Add tags..."
						disabled={loading}
					/>
				</div>

				<!-- Pro Features: UTM Builder + Query Forwarding -->
				{#if isPro}
					<!-- UTM Builder -->
					<div
						class="border border-gray-200 rounded-lg overflow-hidden"
					>
						<button
							type="button"
							class="w-full flex items-center justify-between px-4 py-3 bg-gray-50 hover:bg-gray-100 transition-colors text-sm font-medium text-gray-700"
							onclick={() => (showUtmBuilder = !showUtmBuilder)}
							disabled={loading}
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
								<p class="text-xs text-gray-500 mb-4">
									Appended to the destination URL on every
									redirect.
								</p>
								<div class="space-y-3">
									<!-- Source -->
									<div class="flex items-center gap-3">
										<div
											class="w-8 h-8 rounded-lg bg-gray-100 flex items-center justify-center flex-shrink-0"
										>
											<svg
												class="w-4 h-4 text-gray-600"
												fill="none"
												stroke="currentColor"
												viewBox="0 0 24 24"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2"
													d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
												/>
											</svg>
										</div>
										<label
											for="modal-utm-source"
											class="text-sm font-medium text-gray-700 w-20 flex-shrink-0"
											>Source</label
										>
										<input
											type="text"
											id="modal-utm-source"
											bind:value={utmSource}
											placeholder="e.g. newsletter"
											disabled={loading}
											class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent disabled:bg-gray-100"
										/>
									</div>

									<!-- Medium -->
									<div class="flex items-center gap-3">
										<div
											class="w-8 h-8 rounded-lg bg-gray-100 flex items-center justify-center flex-shrink-0"
										>
											<svg
												class="w-4 h-4 text-gray-600"
												fill="none"
												stroke="currentColor"
												viewBox="0 0 24 24"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2"
													d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z"
												/>
											</svg>
										</div>
										<label
											for="modal-utm-medium"
											class="text-sm font-medium text-gray-700 w-20 flex-shrink-0"
											>Medium</label
										>
										<input
											type="text"
											id="modal-utm-medium"
											bind:value={utmMedium}
											placeholder="e.g. email"
											disabled={loading}
											class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent disabled:bg-gray-100"
										/>
									</div>

									<!-- Campaign -->
									<div class="flex items-center gap-3">
										<div
											class="w-8 h-8 rounded-lg bg-gray-100 flex items-center justify-center flex-shrink-0"
										>
											<svg
												class="w-4 h-4 text-gray-600"
												fill="none"
												stroke="currentColor"
												viewBox="0 0 24 24"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2"
													d="M11 3.055A9.001 9.001 0 1020.945 13H11V3.055z"
												/>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2"
													d="M20.488 9H15V3.512A9.025 9.025 0 0120.488 9z"
												/>
											</svg>
										</div>
										<label
											for="modal-utm-campaign"
											class="text-sm font-medium text-gray-700 w-20 flex-shrink-0"
											>Campaign</label
										>
										<input
											type="text"
											id="modal-utm-campaign"
											bind:value={utmCampaign}
											placeholder="e.g. spring_sale"
											disabled={loading}
											class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent disabled:bg-gray-100"
										/>
									</div>

									<!-- Term -->
									<div class="flex items-center gap-3">
										<div
											class="w-8 h-8 rounded-lg bg-gray-100 flex items-center justify-center flex-shrink-0"
										>
											<svg
												class="w-4 h-4 text-gray-600"
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
										<label
											for="modal-utm-term"
											class="text-sm font-medium text-gray-700 w-20 flex-shrink-0"
											>Term</label
										>
										<input
											type="text"
											id="modal-utm-term"
											bind:value={utmTerm}
											placeholder="e.g. running+shoes"
											disabled={loading}
											class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent disabled:bg-gray-100"
										/>
									</div>

									<!-- Content -->
									<div class="flex items-center gap-3">
										<div
											class="w-8 h-8 rounded-lg bg-gray-100 flex items-center justify-center flex-shrink-0"
										>
											<svg
												class="w-4 h-4 text-gray-600"
												fill="none"
												stroke="currentColor"
												viewBox="0 0 24 24"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2"
													d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
												/>
											</svg>
										</div>
										<label
											for="modal-utm-content"
											class="text-sm font-medium text-gray-700 w-20 flex-shrink-0"
											>Content</label
										>
										<input
											type="text"
											id="modal-utm-content"
											bind:value={utmContent}
											placeholder="e.g. banner_top"
											disabled={loading}
											class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent disabled:bg-gray-100"
										/>
									</div>

									<!-- Ref -->
									<div class="flex items-center gap-3">
										<div
											class="w-8 h-8 rounded-lg bg-gray-100 flex items-center justify-center flex-shrink-0"
										>
											<svg
												class="w-4 h-4 text-gray-600"
												fill="none"
												stroke="currentColor"
												viewBox="0 0 24 24"
											>
												<path
													stroke-linecap="round"
													stroke-linejoin="round"
													stroke-width="2"
													d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m9.032 4.026a9.001 9.001 0 01-7.432 0m9.032-4.026A9.001 9.001 0 0112 3c-4.474 0-8.268 3.12-9.032 7.326M15 12a3 3 0 11-6 0 3 3 0 016 0z"
												/>
											</svg>
										</div>
										<label
											for="modal-utm-ref"
											class="text-sm font-medium text-gray-700 w-20 flex-shrink-0"
											>Referral</label
										>
										<input
											type="text"
											id="modal-utm-ref"
											bind:value={utmRef}
											placeholder="e.g. affiliate123"
											disabled={loading}
											class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent disabled:bg-gray-100"
										/>
									</div>
								</div>

								<!-- URL Preview -->
								{#if destinationUrl && hasUtmParams()}
									<div class="px-4 pb-4">
										<div
											class="text-sm font-medium text-gray-700 mb-1 block"
										>
											URL Preview
										</div>
										<div
											class="p-3 bg-gray-50 rounded-lg border border-gray-200"
										>
											{@html `<p class="text-xs font-mono text-gray-600 break-all">${buildPreviewUrl()}</p>`}
										</div>
									</div>
								{/if}
							</div>
						{/if}
					</div>

					<!-- Forward Query Params Toggle -->
					<div
						class="flex items-start gap-3 p-4 border border-gray-200 rounded-lg"
					>
						<div class="flex-1">
							<label
								for="modal-forward-query-params"
								class="block text-sm font-medium text-gray-700"
							>
								Forward visitor query parameters
							</label>
							<p class="text-xs text-gray-500 mt-0.5">
								Appends query params from the short link URL to
								the destination (e.g. <code
									class="bg-gray-100 px-1 rounded"
									>?ref=tw</code
								> passes through). Visitor params override UTM params
								on conflict.
							</p>
						</div>
						<input
							type="checkbox"
							id="modal-forward-query-params"
							bind:checked={forwardQueryParams}
							disabled={loading}
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
							><strong class="text-gray-700">Pro feature:</strong>
							UTM parameters &amp; query forwarding —
							<a
								href="/pricing"
								class="text-orange-600 hover:underline"
								>Upgrade to Pro</a
							></span
						>
					</div>
				{/if}

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
