<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { page } from "$app/stores";
	import Logo from "$lib/components/Logo.svelte";
	import { authApi, type AuthProvider } from "$lib/api/auth";

	let providers = $state<AuthProvider[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	let signupsDisabled = $derived(
		$page.url.searchParams.get("error") === "signups_disabled",
	);

	onMount(async () => {
		try {
			providers = await authApi.getProviders();
		} catch (err) {
			console.error("Failed to load auth providers:", err);
			error = "Unable to load sign-in options. Please try again.";
		} finally {
			loading = false;
		}
	});

	function getProviderIcon(name: string): string {
		if (name === "github") return "github";
		if (name === "google") return "google";
		return "default";
	}
</script>

<svelte:head>
	<title>Sign In – Rushomon</title>
</svelte:head>

<div
	class="min-h-screen bg-gradient-to-br from-gray-50 to-orange-50 flex flex-col items-center justify-center px-4"
>
	<div class="w-full max-w-sm">
		<!-- Logo -->
		<div class="flex justify-center mb-8">
			<a href="/" class="flex items-center gap-2 group">
				<Logo href="/" />
			</a>
		</div>

		<!-- Card -->
		<div
			class="bg-white rounded-2xl shadow-lg border border-gray-100 px-8 py-10"
		>
			<h1 class="text-2xl font-bold text-gray-900 text-center mb-2">
				Sign in
			</h1>
			<p class="text-gray-500 text-center text-sm mb-8">
				Choose a sign-in method to continue
			</p>

			<!-- Signups Disabled Error -->
			{#if signupsDisabled}
				<div
					class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl mb-6 text-sm"
				>
					<p class="font-medium">
						New signups are currently disabled
					</p>
					<p class="mt-1 text-red-600">
						Contact the administrator if you need access.
					</p>
				</div>
			{/if}

			<!-- Provider Buttons -->
			{#if loading}
				<div class="flex flex-col gap-3">
					<div
						class="h-12 bg-gray-100 rounded-xl animate-pulse"
					></div>
					<div
						class="h-12 bg-gray-100 rounded-xl animate-pulse"
					></div>
				</div>
			{:else if error}
				<div
					class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-xl text-sm text-center"
				>
					{error}
				</div>
			{:else if providers.length === 0}
				<div
					class="bg-yellow-50 border border-yellow-200 text-yellow-800 px-4 py-3 rounded-xl text-sm text-center"
				>
					No sign-in methods are configured. Please contact the
					administrator.
				</div>
			{:else}
				<div class="flex flex-col gap-3">
					{#each providers as provider}
						{@const icon = getProviderIcon(provider.name)}
						{@const redirect =
							$page.url.searchParams.get("redirect")}
						<a
							href={authApi.getProviderLoginUrl(
								provider.name,
								redirect ?? undefined,
							)}
							class="flex items-center justify-center gap-3 w-full px-4 py-3 rounded-xl border font-medium text-sm transition-all duration-150
								{provider.name === 'github'
								? 'bg-gray-900 hover:bg-gray-800 text-white border-gray-900'
								: provider.name === 'google'
									? 'bg-white hover:bg-gray-50 text-gray-800 border-gray-300 shadow-sm'
									: 'bg-orange-500 hover:bg-orange-600 text-white border-orange-500'}"
						>
							{#if icon === "github"}
								<!-- GitHub Mark SVG -->
								<svg
									class="w-5 h-5"
									viewBox="0 0 24 24"
									fill="currentColor"
									aria-hidden="true"
								>
									<path
										fill-rule="evenodd"
										clip-rule="evenodd"
										d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z"
									/>
								</svg>
							{:else if icon === "google"}
								<!-- Google "G" SVG -->
								<svg
									class="w-5 h-5"
									viewBox="0 0 24 24"
									aria-hidden="true"
								>
									<path
										d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
										fill="#4285F4"
									/>
									<path
										d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
										fill="#34A853"
									/>
									<path
										d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
										fill="#FBBC05"
									/>
									<path
										d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
										fill="#EA4335"
									/>
								</svg>
							{/if}
							Continue with {provider.label}
						</a>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Back link -->
		<p class="text-center text-sm text-gray-500 mt-6">
			<a href="/" class="hover:text-orange-600 transition-colors"
				>← Back to home</a
			>
		</p>
	</div>
</div>
