<script lang="ts">
	import Header from '$lib/components/Header.svelte';
	import CreateLinkForm from '$lib/components/CreateLinkForm.svelte';
	import LinkList from '$lib/components/LinkList.svelte';
	import { linksApi } from '$lib/api/links';
	import type { PageData } from './$types';
	import type { Link, ApiError } from '$lib/types/api';

	let { data }: { data: PageData } = $props();

	let links = $state<Link[]>([...data.links]);
	let loading = $state(false);
	let currentPage = $state(1);
	let error = $state('');

	async function handleLinkCreated(newLink: Link) {
		// Add new link to the beginning of the list
		links = [newLink, ...links];
	}

	async function handleDelete(id: string) {
		error = '';
		try {
			await linksApi.delete(id);
			// Remove from list
			links = links.filter((link) => link.id !== id);
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || 'Failed to delete link';
		}
	}

	async function handlePageChange(page: number) {
		if (page < 1) return;

		loading = true;
		error = '';

		try {
			const newLinks = await linksApi.list(page, 20);
			links = newLinks;
			currentPage = page;
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || 'Failed to load links';
		} finally {
			loading = false;
		}
	}

	// Determine if there are more pages (simple check: if we got 20 links, there might be more)
	let hasMore = $derived(links.length === 20);
</script>

<svelte:head>
	<title>Dashboard - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
	<Header user={data.user} />

	<main class="container mx-auto px-4 py-8">
		<div class="max-w-6xl mx-auto">
			<h1 class="text-3xl font-bold text-gray-900 mb-8">Your Short Links</h1>

			<!-- Error Message -->
			{#if error}
				<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-6">
					{error}
				</div>
			{/if}

			<!-- Create Link Form -->
			<CreateLinkForm onLinkCreated={handleLinkCreated} />

			<!-- Links List -->
			<LinkList
				{links}
				{loading}
				onDelete={handleDelete}
				onPageChange={handlePageChange}
				{currentPage}
				{hasMore}
			/>
		</div>
	</main>
</div>
