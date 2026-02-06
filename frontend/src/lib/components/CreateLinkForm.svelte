<script lang="ts">
	import { linksApi } from '$lib/api/links';
	import type { Link, ApiError } from '$lib/types/api';

	let {
		onLinkCreated
	}: {
		onLinkCreated: (link: Link) => void;
	} = $props();

	let destinationUrl = $state('');
	let shortCode = $state('');
	let title = $state('');
	let isSubmitting = $state(false);
	let error = $state('');
	let success = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		success = false;
		isSubmitting = true;

		// Validation
		if (!destinationUrl.trim()) {
			error = 'Destination URL is required';
			isSubmitting = false;
			return;
		}

		if (!destinationUrl.startsWith('http://') && !destinationUrl.startsWith('https://')) {
			error = 'URL must start with http:// or https://';
			isSubmitting = false;
			return;
		}

		if (shortCode && (shortCode.length < 4 || shortCode.length > 10)) {
			error = 'Custom code must be 4-10 characters';
			isSubmitting = false;
			return;
		}

		if (shortCode && !/^[a-zA-Z0-9]+$/.test(shortCode)) {
			error = 'Custom code can only contain letters and numbers';
			isSubmitting = false;
			return;
		}

		try {
			const link = await linksApi.create({
				destination_url: destinationUrl.trim(),
				short_code: shortCode.trim() || undefined,
				title: title.trim() || undefined
			});

			// Success!
			success = true;
			onLinkCreated(link);

			// Clear form
			setTimeout(() => {
				destinationUrl = '';
				shortCode = '';
				title = '';
				success = false;
			}, 2000);
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || 'Failed to create link';
		} finally {
			isSubmitting = false;
		}
	}
</script>

<div class="bg-white rounded-lg shadow-md p-6 mb-8">
	<h2 class="text-xl font-semibold text-gray-900 mb-4">Create New Short Link</h2>

	<form onsubmit={handleSubmit} class="space-y-4">
		<!-- Destination URL -->
		<div>
			<label for="destination-url" class="block text-sm font-medium text-gray-700 mb-1">
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
				<span class="text-gray-500 text-xs font-normal">(Optional, 4-10 alphanumeric characters)</span>
			</label>
			<input
				type="text"
				id="short-code"
				bind:value={shortCode}
				placeholder="my-link"
				pattern="[a-zA-Z0-9]{4,10}"
				class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
			/>
		</div>

		<!-- Title -->
		<div>
			<label for="title" class="block text-sm font-medium text-gray-700 mb-1">
				Title <span class="text-gray-500 text-xs font-normal">(Optional)</span>
			</label>
			<input
				type="text"
				id="title"
				bind:value={title}
				placeholder="My Awesome Link"
				maxlength="200"
				class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
			/>
		</div>

		<!-- Error Message -->
		{#if error}
			<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm">
				{error}
			</div>
		{/if}

		<!-- Success Message -->
		{#if success}
			<div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm">
				✓ Link created successfully!
			</div>
		{/if}

		<!-- Submit Button -->
		<button
			type="submit"
			disabled={isSubmitting}
			class="w-full bg-gray-900 text-white px-6 py-3 rounded-lg font-semibold hover:bg-gray-800 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
		>
			{isSubmitting ? 'Creating...' : 'Create Short Link'}
		</button>
	</form>
</div>
