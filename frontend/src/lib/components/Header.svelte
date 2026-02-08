<script lang="ts">
	import { authApi } from "$lib/api/auth";
	import { goto } from "$app/navigation";
	import type { User } from "$lib/types/api";

	let { user }: { user: User } = $props();
	let showMenu = $state(false);

	async function handleLogout() {
		try {
			// Close the dropdown menu immediately
			showMenu = false;

			await authApi.logout();
			// Successfully logged out, redirect to homepage
			window.location.href = "/";
		} catch (error) {
			console.error("Logout failed:", error);
			// Even if logout fails, try to redirect to homepage
			window.location.href = "/";
		}
	}
</script>

<header class="bg-white border-b border-gray-200">
	<div class="container mx-auto px-4 py-4">
		<div class="flex justify-between items-center">
			<!-- Logo -->
			<a href="/dashboard" class="text-2xl font-bold text-gray-900">
				Rushomon
			</a>

			<!-- User Menu -->
			<div class="relative">
				<button
					onclick={() => (showMenu = !showMenu)}
					class="flex items-center gap-3 px-4 py-2 rounded-lg hover:bg-gray-50 transition-colors"
				>
					{#if user.avatar_url}
						<img
							src={user.avatar_url}
							alt={user.name || user.email}
							class="w-8 h-8 rounded-full"
						/>
					{:else}
						<div
							class="w-8 h-8 rounded-full bg-gray-300 flex items-center justify-center"
						>
							<span class="text-gray-600 font-medium">
								{(user.name || user.email)
									.charAt(0)
									.toUpperCase()}
							</span>
						</div>
					{/if}
					<span class="text-sm font-medium text-gray-700">
						{user.name || user.email}
					</span>
					<svg
						class="w-4 h-4 text-gray-500 transition-transform"
						class:rotate-180={showMenu}
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

				{#if showMenu}
					<div
						class="absolute right-0 mt-2 w-56 bg-white rounded-lg shadow-lg border border-gray-200 py-1 z-10"
					>
						<div class="px-4 py-3 border-b border-gray-100">
							<p class="text-sm font-medium text-gray-900">
								{user.name || "User"}
							</p>
							<p class="text-sm text-gray-500">{user.email}</p>
							<p class="text-xs text-gray-400 mt-1">
								{user.role === "admin"
									? "Administrator"
									: "Member"}
							</p>
						</div>
						{#if user.role === "admin"}
							<a
								href="/admin"
								onclick={() => (showMenu = false)}
								class="block w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
							>
								Admin Dashboard
							</a>
						{/if}
						<button
							onclick={handleLogout}
							class="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
						>
							Sign out
						</button>
					</div>
				{/if}
			</div>
		</div>
	</div>
</header>
