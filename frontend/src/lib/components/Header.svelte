<script lang="ts">
	import Logo from "./Logo.svelte";
	import UserMenu from "./UserMenu.svelte";
	import { authApi } from "$lib/api/auth";
	import { orgsApi } from "$lib/api/orgs";
	import type { User, OrgWithRole } from "$lib/types/api";

	interface Props {
		user?: User | null;
		currentPage?: "landing" | "dashboard" | "admin" | "settings";
	}

	let { user, currentPage = "landing" }: Props = $props();
	let mobileMenuOpen = $state(false);
	let orgSwitcherOpen = $state(false);
	let orgs = $state<OrgWithRole[]>([]);
	let currentOrgId = $state<string>("");
	let switchingOrg = $state(false);
	let orgsLoading = $state(false);
	let showCreateOrg = $state(false);
	let newOrgName = $state("");
	let creatingOrg = $state(false);
	let createOrgError = $state("");

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

	$effect(() => {
		if (
			user &&
			(currentPage === "dashboard" || currentPage === "settings")
		) {
			orgsLoading = true;
			orgsApi
				.listMyOrgs()
				.then((res) => {
					orgs = res.orgs;
					currentOrgId = res.current_org_id;
				})
				.catch(() => {})
				.finally(() => {
					orgsLoading = false;
				});
		}
	});

	async function handleSwitchOrg(orgId: string) {
		if (orgId === currentOrgId || switchingOrg) return;
		switchingOrg = true;
		orgSwitcherOpen = false;
		try {
			await orgsApi.switchOrg(orgId);
			window.location.reload();
		} catch (error) {
			console.error("Failed to switch org:", error);
		} finally {
			switchingOrg = false;
		}
	}

	function handleOrgSettingsClick() {
		orgSwitcherOpen = false;
		window.location.href = "/dashboard/org";
	}

	const currentOrg = $derived(orgs.find((o) => o.id === currentOrgId));
	const canCreateOrg = $derived(currentOrg?.tier === "unlimited");

	function openCreateOrg() {
		showCreateOrg = true;
		newOrgName = "";
		createOrgError = "";
		orgSwitcherOpen = false;
	}

	async function handleCreateOrg() {
		const name = newOrgName.trim();
		if (!name) {
			createOrgError = "Name is required.";
			return;
		}
		if (name.length > 100) {
			createOrgError = "Max 100 characters.";
			return;
		}
		creatingOrg = true;
		createOrgError = "";
		try {
			await orgsApi.createOrg(name);
			window.location.reload();
		} catch (e: any) {
			createOrgError = e?.message ?? "Failed to create organization.";
		} finally {
			creatingOrg = false;
		}
	}

	function focusOnMount(node: HTMLElement) {
		node.focus();
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
					<!-- Unauthenticated Navigation: Features, Pricing & Docs -->
					<a
						href="/#features"
						class="text-sm font-medium text-gray-700 hover:text-orange-600 transition-colors"
					>
						Features
					</a>
					<a
						href="/pricing"
						class="text-sm font-medium text-gray-700 hover:text-orange-600 transition-colors"
					>
						Pricing
					</a>
					<a
						href="https://github.com/piffio/rushomon/"
						target="_blank"
						rel="noopener noreferrer"
						class="text-sm font-medium text-gray-700 hover:text-orange-600 transition-colors"
					>
						Docs
					</a>
				{:else}
					<!-- Authenticated Navigation: Show Pricing on public pages -->
					{#if currentPage === "landing"}
						<a
							href="/pricing"
							class="text-sm font-medium text-gray-700 hover:text-orange-600 transition-colors"
						>
							Pricing
						</a>
					{/if}
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

					<!-- Org Switcher (dashboard & settings pages, always shown when authenticated) -->
					{#if currentPage === "dashboard" || currentPage === "settings"}
						<div class="relative hidden md:block">
							<button
								onclick={() =>
									(orgSwitcherOpen = !orgSwitcherOpen)}
								class="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors max-w-[200px]"
								disabled={switchingOrg}
								aria-label="Organization switcher"
							>
								{#if orgsLoading}
									<span
										class="w-3 h-3 border border-gray-400 border-t-transparent rounded-full animate-spin flex-shrink-0"
									></span>
									<span class="truncate text-gray-400"
										>Loading…</span
									>
								{:else}
									<span
										class="w-5 h-5 rounded bg-gradient-to-br from-orange-400 to-orange-600 flex items-center justify-center text-white text-xs font-bold flex-shrink-0"
									>
										{(currentOrg?.name ?? "O")
											.charAt(0)
											.toUpperCase()}
									</span>
									<span class="truncate"
										>{currentOrg?.name ?? "My Org"}</span
									>
								{/if}
								<svg
									class="w-3.5 h-3.5 flex-shrink-0 text-gray-500"
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

							{#if orgSwitcherOpen}
								<div
									role="button"
									tabindex="-1"
									aria-label="Close org switcher"
									class="fixed inset-0 z-40"
									onclick={() => (orgSwitcherOpen = false)}
									onkeydown={(e) =>
										e.key === "Escape" &&
										(orgSwitcherOpen = false)}
								></div>
								<div
									class="absolute right-0 mt-1 w-72 bg-white rounded-xl border border-gray-200 shadow-lg z-50 overflow-hidden"
								>
									<div
										class="px-3 py-2 border-b border-gray-100"
									>
										<p
											class="text-xs font-semibold text-gray-500 uppercase tracking-wider"
										>
											Organizations
										</p>
									</div>
									<ul class="py-1 max-h-60 overflow-y-auto">
										{#each orgs as org}
											<li>
												<button
													onclick={() =>
														handleSwitchOrg(org.id)}
													class="w-full flex items-center gap-3 px-3 py-2.5 text-sm text-left hover:bg-gray-50 transition-colors {org.id ===
													currentOrgId
														? 'bg-orange-50'
														: ''}"
												>
													<span
														class="w-6 h-6 rounded-md bg-gradient-to-br from-orange-400 to-orange-600 flex items-center justify-center text-white text-xs font-bold flex-shrink-0"
													>
														{org.name
															.charAt(0)
															.toUpperCase()}
													</span>
													<span
														class="flex-1 min-w-0"
													>
														<span
															class="block truncate font-medium text-gray-900"
															>{org.name}</span
														>
														<span
															class="block text-xs text-gray-500 capitalize"
															>{org.role} · {org.tier}</span
														>
													</span>
													{#if org.id === currentOrgId}
														<svg
															class="w-4 h-4 text-orange-500 flex-shrink-0"
															fill="none"
															stroke="currentColor"
															viewBox="0 0 24 24"
														>
															<path
																stroke-linecap="round"
																stroke-linejoin="round"
																stroke-width="2"
																d="M5 13l4 4L19 7"
															/>
														</svg>
													{/if}
												</button>
											</li>
										{/each}
									</ul>
									<div class="border-t border-gray-100 py-1">
										<!-- Create Organization -->
										{#if canCreateOrg}
											<button
												onclick={openCreateOrg}
												class="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
											>
												<svg
													class="w-4 h-4 text-gray-400"
													fill="none"
													stroke="currentColor"
													viewBox="0 0 24 24"
												>
													<path
														stroke-linecap="round"
														stroke-linejoin="round"
														stroke-width="2"
														d="M12 4v16m8-8H4"
													/>
												</svg>
												Create Organization
											</button>
										{:else}
											<div
												class="flex items-center gap-2 px-3 py-2 text-sm text-gray-400 cursor-default select-none"
											>
												<svg
													class="w-4 h-4"
													fill="none"
													stroke="currentColor"
													viewBox="0 0 24 24"
												>
													<path
														stroke-linecap="round"
														stroke-linejoin="round"
														stroke-width="2"
														d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
													/>
												</svg>
												<span>Create Organization</span>
												<span
													class="ml-auto text-xs bg-amber-100 text-amber-700 px-1.5 py-0.5 rounded font-medium"
													>Unlimited</span
												>
											</div>
										{/if}
										<!-- Organization Settings -->
										<button
											onclick={handleOrgSettingsClick}
											class="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
										>
											<svg
												class="w-4 h-4 text-gray-400"
												fill="none"
												stroke="currentColor"
												viewBox="0 0 24 24"
											>
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
											Organization Settings
										</button>
									</div>
								</div>
							{/if}
						</div>
					{/if}

					<UserMenu {user} onLogout={handleLogout} />
				{:else}
					<!-- Unauthenticated: Show Sign In button -->
					<a
						href="/login"
						class="hidden md:block px-4 py-2 text-sm font-semibold text-gray-700 hover:text-orange-600 transition-colors"
					>
						Sign In
					</a>
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
					<!-- Authenticated Mobile Nav -->
					{#if currentPage === "landing"}
						<!-- Show Pricing on landing page -->
						<a
							href="/pricing"
							class="block py-2 text-gray-700 hover:text-orange-600 transition-colors"
							>💰 Pricing</a
						>
					{/if}
					<div class="border-t border-gray-100 pt-2">
						{#if currentPage === "dashboard" || currentPage === "settings"}
							<button
								onclick={() => {
									mobileMenuOpen = false;
									window.location.href = "/dashboard/org";
								}}
								class="block w-full text-left py-2 text-gray-700 hover:text-orange-600 transition-colors"
								>🏢 Organization Settings</button
							>
						{/if}
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
								href="/admin/dashboard"
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
						href="/pricing"
						class="block py-2 text-gray-700 hover:text-orange-600 transition-colors"
						>Pricing</a
					>
					<a
						href="https://github.com/piffio/rushomon/"
						target="_blank"
						rel="noopener noreferrer"
						class="block py-2 text-gray-700 hover:text-orange-600 transition-colors"
						>Docs</a
					>
					<div class="border-t border-gray-100 mt-2 pt-2">
						<a
							href="/login"
							class="block py-2 font-semibold text-orange-600 hover:text-orange-700 transition-colors"
							>Sign In</a
						>
					</div>
				{/if}
			</nav>
		{/if}
	</div>
</header>

<!-- Create Organization Modal -->
{#if showCreateOrg}
	<div
		role="dialog"
		aria-modal="true"
		aria-label="Create organization"
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
	>
		<!-- Backdrop -->
		<div
			role="button"
			tabindex="-1"
			aria-label="Close dialog"
			class="absolute inset-0 bg-black/40"
			onclick={() => (showCreateOrg = false)}
			onkeydown={(e) => e.key === "Escape" && (showCreateOrg = false)}
		></div>
		<!-- Dialog -->
		<div
			class="relative bg-white rounded-2xl shadow-xl w-full max-w-md p-6"
		>
			<h2 class="text-lg font-semibold text-gray-900 mb-1">
				Create Organization
			</h2>
			<p class="text-sm text-gray-500 mb-5">
				Create a new workspace for your team. You'll be set as the
				owner.
			</p>

			<label
				for="new-org-name"
				class="block text-sm font-medium text-gray-700 mb-1"
				>Organization name</label
			>
			<input
				id="new-org-name"
				type="text"
				bind:value={newOrgName}
				placeholder="Acme Corp"
				maxlength="100"
				use:focusOnMount
				class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-orange-500 outline-none"
				onkeydown={(e: KeyboardEvent) =>
					e.key === "Enter" && handleCreateOrg()}
			/>
			{#if createOrgError}
				<p class="mt-2 text-sm text-red-600">{createOrgError}</p>
			{/if}

			<div class="flex gap-3 mt-5">
				<button
					onclick={handleCreateOrg}
					disabled={creatingOrg}
					class="flex-1 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg text-sm font-semibold hover:from-orange-600 hover:to-orange-700 transition-all disabled:opacity-50"
				>
					{creatingOrg ? "Creating…" : "Create Organization"}
				</button>
				<button
					onclick={() => (showCreateOrg = false)}
					class="px-4 py-2 border border-gray-300 text-gray-700 rounded-lg text-sm hover:bg-gray-50 transition-colors"
				>
					Cancel
				</button>
			</div>
		</div>
	</div>
{/if}
