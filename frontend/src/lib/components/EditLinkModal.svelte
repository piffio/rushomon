<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import type { Link, UpdateLinkRequest } from '$lib/types/api';
	import { linksApi } from '$lib/api/links';

	let {
		link,
		isOpen = $bindable(false)
	}: {
		link: Link;
		isOpen?: boolean;
	} = $props();

	const dispatch = createEventDispatcher<{ close: void; updated: Link }>();

	let destinationUrl = $state(link.destination_url);
	let title = $state(link.title || '');
	let expiresAt = $state(
		link.expires_at ? new Date(link.expires_at * 1000).toISOString().slice(0, 16) : ''
	);
	let status = $state(link.status);

	let isSubmitting = $state(false);
	let error = $state('');

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		isSubmitting = true;

		try {
			const updateData: UpdateLinkRequest = {};

			// Only send changed fields
			if (destinationUrl !== link.destination_url) {
				updateData.destination_url = destinationUrl;
			}
			if (title !== (link.title || '')) {
				updateData.title = title || undefined;
			}
			if (status !== link.status) {
				updateData.status = status;
			}
			if (expiresAt) {
				updateData.expires_at = Math.floor(new Date(expiresAt).getTime() / 1000);
			} else if (link.expires_at) {
				updateData.expires_at = undefined; // Clear expiration
			}

			const updatedLink = await linksApi.update(link.id, updateData);
			dispatch('updated', updatedLink);
			dispatch('close');
			isOpen = false;
		} catch (err: any) {
			error = err.message || 'Failed to update link';
		} finally {
			isSubmitting = false;
		}
	}

	function handleClose() {
		if (!isSubmitting) {
			dispatch('close');
			isOpen = false;
		}
	}
</script>

{#if isOpen}
	<div class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4">
		<div class="bg-white rounded-lg shadow-xl max-w-md w-full">
			<div class="p-6">
				<h3 class="text-xl font-bold text-gray-900 mb-4">Edit Link</h3>

				<form onsubmit={handleSubmit}>
					<!-- Short Code (read-only) -->
					<div class="mb-4">
						<label for="short-code-readonly" class="block text-sm font-medium text-gray-700 mb-1">
							Short Code
						</label>
						<input
							type="text"
							id="short-code-readonly"
							value={link.short_code}
							disabled
							class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-gray-100 text-gray-500 cursor-not-allowed"
						/>
						<p class="text-xs text-gray-500 mt-1">Short codes cannot be changed</p>
					</div>

					<!-- Destination URL -->
					<div class="mb-4">
						<label for="destination-url-edit" class="block text-sm font-medium text-gray-700 mb-1">
							Destination URL <span class="text-red-500">*</span>
						</label>
						<input
							type="url"
							id="destination-url-edit"
							bind:value={destinationUrl}
							required
							placeholder="https://example.com"
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
						/>
					</div>

					<!-- Title -->
					<div class="mb-4">
						<label for="title-edit" class="block text-sm font-medium text-gray-700 mb-1">
							Title <span class="text-gray-500 text-xs font-normal">(Optional)</span>
						</label>
						<input
							type="text"
							id="title-edit"
							bind:value={title}
							placeholder="My Link"
							maxlength="200"
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
						/>
					</div>

					<!-- Status -->
					<div class="mb-4">
						<label for="status-edit" class="block text-sm font-medium text-gray-700 mb-1">
							Status
						</label>
						<select
							id="status-edit"
							bind:value={status}
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
						>
							<option value="active">Active - Link redirects normally</option>
							<option value="disabled">Disabled - Link returns 404</option>
						</select>
						<p class="text-xs text-gray-500 mt-1">
							Disabled links don't redirect but keep the short code reserved
						</p>
					</div>

					<!-- Expiration Date -->
					<div class="mb-4">
						<label for="expires-at-edit" class="block text-sm font-medium text-gray-700 mb-1">
							Expiration Date <span class="text-gray-500 text-xs font-normal">(Optional)</span>
						</label>
						<input
							type="datetime-local"
							id="expires-at-edit"
							bind:value={expiresAt}
							class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
						/>
						<p class="text-xs text-gray-500 mt-1">Leave empty for no expiration</p>
					</div>

					{#if error}
						<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm mb-4">
							{error}
						</div>
					{/if}

					<div class="flex gap-3 justify-end">
						<button
							type="button"
							class="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
							onclick={handleClose}
							disabled={isSubmitting}
						>
							Cancel
						</button>
						<button
							type="submit"
							class="px-6 py-2 bg-gray-900 text-white rounded-lg font-semibold hover:bg-gray-800 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
							disabled={isSubmitting}
						>
							{isSubmitting ? 'Saving...' : 'Save Changes'}
						</button>
					</div>
				</form>
			</div>
		</div>
	</div>
{/if}
