<script lang="ts">
	import Logo from "./Logo.svelte";
	import UserMenu from "./UserMenu.svelte";
	import { authApi } from "$lib/api/auth";
	import type { User } from "$lib/types/api";

	interface Props {
		user?: User | null;
		currentPage?: "landing" | "dashboard" | "admin" | "settings";
	}

	let { user, currentPage = "landing" }: Props = $props();
	let mobileMenuOpen = $state(false);

	// Logo always links to landing page
	const logoHref = "/";

	async function handleLogout() {
		try {
			await authApi.logout();
			window.location.href = "/";
		} catch (error) {
			console.error("Logout failed:", error);
			window.location.href = "/";
		}
	}
</script>

<header class="bg-white border-b border-gray-200 sticky top-0 z-40">
	<div class="container mx-auto px-4 py-4">
		<div class="flex justify-between items-center">
			<!-- Logo (Left) -->
			<Logo href={logoHref} />

			<!-- Desktop Navigation (Center-Left) -->
			<nav class="hidden md:flex items-center gap-6 ml-8">
				{#if !user}
					<!-- Unauthenticated Navigation: Features & Docs -->
					<a
						href="/#features"
						class="text-sm font-medium text-gray-700 hover:text-orange-600 transition-colors"
					>
						Features
					</a>
					<a
						href="https://github.com/piffio/rushomon/"
						target="_blank"
						rel="noopener noreferrer"
						class="text-sm font-medium text-gray-700 hover:text-orange-600 transition-colors"
					>
						Docs
					</a>
				{/if}
			</nav>

			<!-- Right Side Actions -->
			<div class="flex items-center gap-4">
				{#if user}
					<!-- Authenticated: Show "Go to Dashboard" CTA only on landing page -->
					{#if currentPage === "landing"}
						<a
							href="/dashboard"
							class="hidden md:block px-4 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm hover:shadow-md text-sm"
						>
							Go to Dashboard →
						</a>
					{/if}
					<UserMenu {user} onLogout={handleLogout} />
				{/if}

				<!-- Mobile Menu Button -->
				<button
					onclick={() => (mobileMenuOpen = !mobileMenuOpen)}
					class="md:hidden p-2 text-gray-700 hover:text-orange-600 transition-colors"
					aria-label="Toggle menu"
				>
					<svg
						class="w-6 h-6"
						fill="none"
						stroke="currentColor"
						viewBox="0 0 24 24"
					>
						{#if mobileMenuOpen}
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							/>
						{:else}
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M4 6h16M4 12h16M4 18h16"
							/>
						{/if}
					</svg>
				</button>
			</div>
		</div>

		<!-- Mobile Menu (Collapsible) -->
		{#if mobileMenuOpen}
			<nav class="md:hidden mt-4 pb-4 border-t border-gray-200 pt-4">
				{#if user}
					<!-- Authenticated Mobile Nav: Only settings and utility links -->
					<div class="border-t border-gray-100 pt-2">
						<a
							href="/settings"
							class="block py-2 text-gray-700 hover:text-orange-600 transition-colors"
							>⚙️ Settings</a
						>
						<a
							href="https://github.com/piffio/rushomon/"
							target="_blank"
							rel="noopener noreferrer"
							class="block py-2 text-gray-700 hover:text-orange-600 transition-colors"
							>📖 Docs</a
						>
						{#if user.role === "admin"}
							<a
								href="/admin"
								class="block py-2 text-gray-700 hover:text-orange-600 transition-colors"
								>👥 Admin</a
							>
						{/if}
					</div>
					<div class="border-t border-gray-100 mt-2 pt-2">
						<button
							onclick={handleLogout}
							class="w-full text-left py-2 text-gray-700 hover:text-orange-600 transition-colors"
						>
							🚪 Log out
						</button>
					</div>
				{:else}
					<!-- Unauthenticated Mobile Nav -->
					<a
						href="/#features"
						class="block py-2 text-gray-700 hover:text-orange-600 transition-colors"
						>Features</a
					>
					<a
						href="https://github.com/piffio/rushomon/"
						target="_blank"
						rel="noopener noreferrer"
						class="block py-2 text-gray-700 hover:text-orange-600 transition-colors"
						>Docs</a
					>
				{/if}
			</nav>
		{/if}
	</div>
</header>
