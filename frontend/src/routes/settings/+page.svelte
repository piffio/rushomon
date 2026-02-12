<script lang="ts">
	import Header from '$lib/components/Header.svelte';
	import { authApi } from '$lib/api/auth';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	async function handleLogout() {
		try {
			await authApi.logout();
			window.location.href = '/';
		} catch (error) {
			console.error('Logout failed:', error);
			window.location.href = '/';
		}
	}
</script>

<svelte:head>
	<title>Settings - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50 flex flex-col">
	<Header user={data.user} currentPage="settings" />

	<main class="flex-1 container mx-auto px-4 py-12">
		<div class="max-w-3xl mx-auto">
			<h1 class="text-3xl font-bold text-gray-900 mb-2">Settings</h1>
			<p class="text-gray-600 mb-8">Manage your account preferences and configuration.</p>

			<!-- Coming Soon Notice -->
			<div class="bg-white rounded-2xl border-2 border-gray-200 p-8 text-center">
				<div
					class="w-16 h-16 bg-orange-100 rounded-full flex items-center justify-center mx-auto mb-4"
				>
					<svg class="w-8 h-8 text-orange-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
						/>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							stroke-width="2"
							d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
						/>
					</svg>
				</div>
				<h2 class="text-xl font-semibold text-gray-900 mb-2">Settings Coming Soon</h2>
				<p class="text-gray-600 mb-6">
					We're working on building comprehensive settings for your account. Check back soon!
				</p>
				<p class="text-sm text-gray-500 mb-6">
					Future features will include: Profile management, API keys, preferences, and more.
				</p>
				<button
					onclick={handleLogout}
					class="px-6 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg transition-colors"
				>
					Log out
				</button>
			</div>

			<!-- User Info Preview -->
			{#if data.user}
				<div class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6">
					<h3 class="font-semibold text-gray-900 mb-4">Your Account</h3>
					<div class="space-y-3 text-sm">
						<div class="flex justify-between">
							<span class="text-gray-600">Email</span>
							<span class="text-gray-900 font-medium">{data.user.email}</span>
						</div>
						<div class="flex justify-between">
							<span class="text-gray-600">Name</span>
							<span class="text-gray-900 font-medium">{data.user.name || 'Not set'}</span>
						</div>
						<div class="flex justify-between">
							<span class="text-gray-600">Role</span>
							<span class="text-gray-900 font-medium"
								>{data.user.role === 'admin' ? 'Administrator' : 'Member'}</span
							>
						</div>
						<div class="flex justify-between">
							<span class="text-gray-600">Joined</span>
							<span class="text-gray-900 font-medium"
								>{new Date(data.user.created_at * 1000).toLocaleDateString()}</span
							>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</main>
</div>
