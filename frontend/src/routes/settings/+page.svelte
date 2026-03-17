<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import { orgsApi } from "$lib/api/orgs";
	import { billingApi } from "$lib/api/billing";
	import { apiClient } from "$lib/api/client";
	import { apiKeysApi, type ApiKey } from "$lib/api/settings";
	import type { PageData } from "./$types";
	import type { OrgWithRole } from "$lib/types/api";
	import type { BillingStatus } from "$lib/api/billing";

	let { data }: { data: PageData } = $props();

	// Version info from API
	let versionInfo = $state({
		version: "Loading...",
		name: "Rushomon",
		build_timestamp: "unknown",
		git_commit: "unknown",
	});

	// User organizations
	let userOrgs = $state<OrgWithRole[]>([]);
	let orgsLoading = $state(true);

	// Billing status (needed to know if user is billing owner)
	let billingStatus = $state<BillingStatus | null>(null);

	// API Keys State
	let apiKeys = $state<ApiKey[]>([]);
	let keysLoading = $state(true);
	let isCreateModalOpen = $state(false);
	let newKeyName = $state("");
	let newKeyExpires = $state<number | null>(30); // Default 30 days
	let newlyGeneratedToken = $state<string | null>(null);
	let isCreatingKey = $state(false);

	// Helper for version API URL
	const versionApiUrl = `${apiClient["baseUrl"]}/api/version`;

	$effect(() => {
		apiClient
			.get<{
				version: string;
				name: string;
				build_timestamp: string;
				git_commit: string;
			}>("/api/version")
			.then((d) => {
				versionInfo = d;
			})
			.catch(() => {
				versionInfo.version = "Error";
			});

		orgsApi
			.listMyOrgs()
			.then((res) => {
				userOrgs = res.orgs;
			})
			.catch(() => {})
			.finally(() => {
				orgsLoading = false;
			});

		billingApi
			.getStatus()
			.then((status) => {
				billingStatus = status;
			})
			.catch(() => {});

		apiKeysApi.list()
			.then((res) => {
				apiKeys = res;
			})
			.catch(() => {})
			.finally(() => {
				keysLoading = false;
			});
	});

	function formatDate(ts: number): string {
		return new Date(ts * 1000).toLocaleDateString();
	}

	const tierColors: Record<string, string> = {
		free: "bg-gray-100 text-gray-600",
		pro: "bg-blue-100 text-blue-700",
		business: "bg-amber-100 text-amber-700",
	};

	async function handleCreateKey() {
		if (!newKeyName.trim()) return;
		isCreatingKey = true;
		try {
			const res = await apiKeysApi.create(newKeyName, newKeyExpires);
			newlyGeneratedToken = res.raw_token;
			// Refresh the list immediately
			apiKeys = await apiKeysApi.list();
		} catch (e) {
			alert("Failed to create API key");
		} finally {
			isCreatingKey = false;
		}
	}

	async function handleRevokeKey(id: string) {
		if (!confirm("Are you sure you want to revoke this key? Any integrations using it will immediately break.")) return;
		try {
			await apiKeysApi.revoke(id);
			apiKeys = apiKeys.filter(k => k.id !== id);
		} catch (e) {
			alert("Failed to revoke key");
		}
	}

	function copyToken() {
		if (newlyGeneratedToken) {
			navigator.clipboard.writeText(newlyGeneratedToken);
			alert("Copied to clipboard!");
		}
	}
</script>

<svelte:head>
	<title>Account Settings - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50 flex flex-col">
	<Header user={data.user} currentPage="settings" />

	<main class="flex-1 container mx-auto px-4 py-12">
		<div class="max-w-3xl mx-auto">
			<h1 class="text-3xl font-bold text-gray-900 mb-2">
				Account Settings
			</h1>
			<p class="text-gray-600 mb-8">
				Manage your personal account preferences and configuration.
			</p>

			<!-- User Info Preview -->
			{#if data.user}
				<div
					class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6"
				>
					<h3 class="font-semibold text-gray-900 mb-4">
						Your Account
					</h3>
					<div class="space-y-3 text-sm">
						<div class="flex justify-between">
							<span class="text-gray-600">Email</span>
							<span class="text-gray-900 font-medium"
								>{data.user.email}</span
							>
						</div>
						<div class="flex justify-between">
							<span class="text-gray-600">Name</span>
							<span class="text-gray-900 font-medium"
								>{data.user.name || "Not set"}</span
							>
						</div>
						<div class="flex justify-between">
							<span class="text-gray-600">Joined</span>
							<span class="text-gray-900 font-medium"
								>{formatDate(data.user.created_at)}</span
							>
						</div>
					</div>
				</div>

				<!-- Organizations Section -->
				<div
					class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6"
				>
					<h3 class="font-semibold text-gray-900 mb-4">
						Your Organizations
					</h3>
					{#if orgsLoading}
						<div
							class="flex items-center gap-2 text-sm text-gray-500"
						>
							<svg
								class="animate-spin h-4 w-4"
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
							Loading organizations...
						</div>
					{:else if userOrgs.length === 0}
						<p class="text-sm text-gray-500">
							You don't belong to any organizations yet.
						</p>
					{:else}
						<ul class="divide-y divide-gray-100">
							{#each userOrgs as org}
								<li
									class="flex items-center justify-between py-3 first:pt-0 last:pb-0"
								>
									<div class="flex items-center gap-3">
										<span
											class="w-8 h-8 rounded-md bg-gradient-to-br from-orange-400 to-orange-600 flex items-center justify-center text-white text-xs font-bold flex-shrink-0"
										>
											{org.name.charAt(0).toUpperCase()}
										</span>
										<div>
											<p
												class="text-sm font-medium text-gray-900"
											>
												{org.name}
											</p>
											<p
												class="text-xs text-gray-500 capitalize"
											>
												{org.role}
											</p>
										</div>
									</div>
									<div class="flex items-center gap-3">
										<span
											class="text-xs px-2 py-0.5 rounded-full capitalize font-medium {tierColors[
												org.tier
											] ?? 'bg-gray-100 text-gray-600'}"
										>
											{org.tier}
										</span>
										{#if billingStatus?.is_billing_owner && org.role === "owner"}
											<a
												href="/billing"
												class="text-xs text-orange-600 hover:text-orange-700 font-medium transition-colors"
												>Billing →</a
											>
										{/if}
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</div>

				<div class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6">
					<div class="flex justify-between items-center mb-4">
						<div>
							<h3 class="font-semibold text-gray-900">Developer API Keys</h3>
							<p class="text-xs text-gray-500 mt-1">Manage Personal Access Tokens for programmatic access to the API.</p>
						</div>
						<button 
							onclick={() => { isCreateModalOpen = true; newlyGeneratedToken = null; newKeyName = ""; }}
							class="px-4 py-2 bg-gray-900 text-white text-sm font-medium rounded-lg hover:bg-gray-800 transition-colors"
						>
							Generate New Key
						</button>
					</div>

					{#if keysLoading}
						<div class="flex items-center gap-2 text-sm text-gray-500 py-2">
							<svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
								<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
								<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
							</svg>
							Loading keys...
						</div>
					{:else if apiKeys.length === 0}
						<p class="text-sm text-gray-500 py-2 border-t border-gray-100">You don't have any active API keys.</p>
					{:else}
						<ul class="divide-y divide-gray-100 border-t border-gray-100">
							{#each apiKeys as key}
								<li class="flex items-center justify-between py-4 first:pt-4 last:pb-0">
									<div>
										<p class="text-sm font-medium text-gray-900">{key.name}</p>
										<p class="text-xs text-gray-500 font-mono mt-1">{key.hint}</p>
										<p class="text-xs text-gray-400 mt-1.5">
											Created {formatDate(key.created_at)} 
											{#if key.last_used_at} • Last used {formatDate(key.last_used_at)}{/if}
										</p>
									</div>
									<button 
										onclick={() => handleRevokeKey(key.id)}
										class="text-xs text-red-600 hover:text-red-800 font-medium px-3 py-1.5 rounded-md bg-red-50 hover:bg-red-100 transition-colors"
									>
										Revoke
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</div>
			{/if}

			<!-- Version Information -->
			<div class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6">
				<h3 class="font-semibold text-gray-900 mb-4">
					Application Version
				</h3>
				<div class="space-y-3 text-sm">
					<div class="flex justify-between">
						<span class="text-gray-600">Version</span>
						<span class="text-gray-900 font-medium"
							>{versionInfo.version}</span
						>
					</div>
					<div class="flex justify-between">
						<span class="text-gray-600">Application</span>
						<span class="text-gray-900 font-medium"
							>{versionInfo.name}</span
						>
					</div>
					<div class="flex justify-between">
						<span class="text-gray-600">Build Timestamp</span>
						<span class="text-gray-900 font-medium"
							>{versionInfo.build_timestamp}</span
						>
					</div>
					<div class="flex justify-between">
						<span class="text-gray-600">Git Commit</span>
						<span
							class="text-gray-900 font-medium text-xs font-mono"
						>
							{versionInfo.git_commit}
						</span>
					</div>
				</div>

				<div class="mt-4 pt-4 border-t border-gray-200">
					<p class="text-xs text-gray-500">
						For detailed version information, visit the
						<a
							href={versionApiUrl}
							target="_blank"
							class="text-blue-600 hover:text-blue-800 underline"
							>version API</a
						>
						or check the
						<a
							href="https://github.com/piffio/rushomon/releases"
							target="_blank"
							class="text-blue-600 hover:text-blue-800 underline"
							>GitHub releases</a
						>.
					</p>
				</div>
			</div>
		</div>
	</main>
</div>

{#if isCreateModalOpen}
	<div class="fixed inset-0 bg-gray-900/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
		<div class="bg-white rounded-2xl shadow-xl w-full max-w-md p-6">
			<h2 class="text-xl font-bold text-gray-900 mb-4">Generate API Key</h2>
			
			{#if newlyGeneratedToken}
				<div class="bg-amber-50 border border-amber-200 rounded-xl p-5 mb-5 shadow-sm">
					<p class="text-sm text-amber-800 font-bold mb-2 flex items-center gap-2">
						<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg>
						Copy this key now!
					</p>
					<p class="text-xs text-amber-700 mb-4">For your security, we only show this token once. If you lose it, you will need to revoke it and generate a new one.</p>
					<div class="flex gap-2">
						<input type="text" readonly value={newlyGeneratedToken} class="flex-1 text-sm font-mono bg-white border border-amber-300 rounded-lg px-3 py-2 text-gray-900 outline-none" />
						<button onclick={copyToken} class="px-4 py-2 bg-amber-600 text-white text-sm font-medium rounded-lg hover:bg-amber-700 transition-colors shadow-sm">Copy</button>
					</div>
				</div>
				<button onclick={() => isCreateModalOpen = false} class="w-full py-2.5 bg-gray-100 text-gray-900 font-medium rounded-lg hover:bg-gray-200 transition-colors">
					I have copied it safely
				</button>
			{:else}
				<div class="space-y-5 mb-8">
					<div>
						<label for="keyName" class="block text-sm font-medium text-gray-700 mb-1.5">Key Name</label>
						<input 
							id="keyName" 
							type="text" 
							bind:value={newKeyName} 
							placeholder="e.g., Zapier Integration" 
							class="w-full border border-gray-300 rounded-lg px-3 py-2.5 text-sm focus:ring-2 focus:ring-gray-900 focus:border-gray-900 outline-none transition-shadow" 
							/>
					</div>
					<div>
						<label for="keyExpiration" class="block text-sm font-medium text-gray-700 mb-1.5">Expiration</label>
						<select id="keyExpiration" bind:value={newKeyExpires} class="w-full border border-gray-300 rounded-lg px-3 py-2.5 text-sm focus:ring-2 focus:ring-gray-900 focus:border-gray-900 outline-none bg-white">
							<option value={7}>7 days</option>
							<option value={30}>30 days</option>
							<option value={90}>90 days</option>
							<option value={null}>Never expire</option>
						</select>
					</div>
				</div>
				<div class="flex justify-end gap-3 pt-2 border-t border-gray-100">
					<button onclick={() => isCreateModalOpen = false} class="px-4 py-2 text-sm font-medium text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors">
						Cancel
					</button>
					<button 
						onclick={handleCreateKey} 
						disabled={!newKeyName.trim() || isCreatingKey} 
						class="px-5 py-2 bg-gray-900 text-white text-sm font-medium rounded-lg hover:bg-gray-800 disabled:opacity-50 transition-colors shadow-sm flex items-center justify-center min-w-[120px]"
					>
						{#if isCreatingKey}
							<svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" fill="none" viewBox="0 0 24 24">
								<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
								<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
							</svg>
							Generating
						{:else}
							Generate Key
						{/if}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}