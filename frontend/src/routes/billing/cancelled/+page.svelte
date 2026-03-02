<script lang="ts">
	import { onMount } from 'svelte';
	import Header from '$lib/components/Header.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import { authApi } from '$lib/api/auth';
	import type { User } from '$lib/types/api';

	let currentUser = $state<User | undefined>(undefined);

	onMount(async () => {
		try {
			currentUser = await authApi.me();
		} catch {
			// Not critical for this page
		}
	});
</script>

<svelte:head>
	<title>Checkout Cancelled - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50 flex flex-col">
	<Header user={currentUser} currentPage="dashboard" />

	<main class="flex-1 flex items-center justify-center px-4 py-20">
		<div class="max-w-md w-full bg-white rounded-2xl border border-gray-200 p-10 text-center">
			<div class="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mx-auto mb-6">
				<svg class="w-8 h-8 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
				</svg>
			</div>

			<h1 class="text-2xl font-bold text-gray-900 mb-3">Checkout cancelled</h1>
			<p class="text-gray-600 mb-8">
				No worries — you haven't been charged. You can upgrade whenever you're ready.
			</p>

			<div class="flex flex-col gap-3">
				<a
					href="/pricing"
					class="block w-full px-6 py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm text-center"
				>
					View Plans
				</a>
				<a
					href="/dashboard"
					class="block w-full px-6 py-3 border border-gray-200 text-gray-700 rounded-lg font-semibold hover:bg-gray-50 transition-colors text-center"
				>
					Back to Dashboard
				</a>
			</div>
		</div>
	</main>

	<Footer />
</div>
