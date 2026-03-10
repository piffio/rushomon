<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import { orgsApi } from "$lib/api/orgs";
	import { billingApi } from "$lib/api/billing";
	import { apiClient } from "$lib/api/client";
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
	});

	function formatDate(ts: number): string {
		return new Date(ts * 1000).toLocaleDateString();
	}

	const tierColors: Record<string, string> = {
		free: "bg-gray-100 text-gray-600",
		pro: "bg-blue-100 text-blue-700",
		business: "bg-amber-100 text-amber-700",
	};
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
