<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import { adminApi } from "$lib/api/admin";
	import { apiClient } from "$lib/api/client";
	import { goto } from "$app/navigation";
	import type { PageData } from "./$types";
	import type { User, ApiError, PaginationMeta } from "$lib/types/api";

	let { data }: { data: PageData } = $props();

	let users = $state<User[]>([]);
	let total = $state(0);
	let loading = $state(false);
	let error = $state("");
	let currentPage = $state(1);
	let confirmingUserId = $state<string | null>(null);
	let confirmingRole = $state<"admin" | "member" | null>(null);
	let signupsEnabled = $state(true);
	let defaultUserTier = $state("free");
	let settingsLoading = $state(false);
	let confirmingSignupToggle = $state(false);
	let toast = $state<{
		message: string;
		type: "success" | "error";
		visible: boolean;
	}>({
		message: "",
		type: "success",
		visible: false,
	});
	let confirmingTierChange = $state<{
		userId: string;
		orgId: string;
		currentTier: string;
	} | null>(null);
	let tierLoading = $state(false);

	$effect(() => {
		users = [...(data.users as User[])];
		total = data.total as number;
		const d = data as Record<string, any>;
		const settings = (d.settings ?? {}) as Record<string, string>;
		signupsEnabled = settings.signups_enabled !== "false";
		defaultUserTier = settings.default_user_tier || "free";
		orgTiers = { ...(d.orgTiers ?? {}) } as Record<string, string>;
	});

	async function handleRoleChange(
		userId: string,
		newRole: "admin" | "member",
	) {
		// Show confirmation dialog
		confirmingUserId = userId;
		confirmingRole = newRole;
	}

	async function confirmRoleChange() {
		if (!confirmingUserId || !confirmingRole) return;

		error = "";
		loading = true;

		try {
			const updatedUser = await adminApi.updateUserRole(
				confirmingUserId,
				confirmingRole,
			);
			users = users.map((u) =>
				u.id === updatedUser.id ? updatedUser : u,
			);
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to update user role";
		} finally {
			loading = false;
			confirmingUserId = null;
			confirmingRole = null;
		}
	}

	function cancelRoleChange() {
		confirmingUserId = null;
		confirmingRole = null;
	}

	async function handlePageChange(page: number) {
		if (page < 1) return;

		loading = true;
		error = "";

		try {
			const response = await adminApi.listUsers(page, 50);
			users = response.users;
			total = response.total;
			currentPage = page;
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to load users";
		} finally {
			loading = false;
		}
	}

	function handleSignupToggle() {
		confirmingSignupToggle = true;
	}

	async function confirmSignupToggle() {
		settingsLoading = true;
		error = "";

		try {
			const newValue = signupsEnabled ? "false" : "true";
			const updatedSettings = await adminApi.updateSetting(
				"signups_enabled",
				newValue,
			);
			signupsEnabled = updatedSettings.signups_enabled !== "false";
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to update setting";
		} finally {
			settingsLoading = false;
			confirmingSignupToggle = false;
		}
	}

	function cancelSignupToggle() {
		confirmingSignupToggle = false;
	}

	function showToast(
		message: string,
		type: "success" | "error",
		refreshable: boolean = false,
	) {
		toast.message = message;
		toast.type = type;
		toast.visible = true;

		// Auto-hide after 3 seconds
		setTimeout(() => {
			toast.visible = false;
		}, 3000);

		// Add click handler for refreshable errors
		if (refreshable) {
			setTimeout(() => {
				const toastElement = document.querySelector(
					'[data-toast-refresh="true"]',
				);
				if (toastElement) {
					toastElement.addEventListener(
						"click",
						refreshSessionAndReload,
					);
					(toastElement as HTMLElement).style.cursor = "pointer";
				}
			}, 100);
		}
	}

	async function refreshSessionAndReload() {
		try {
			await apiClient.post("/api/auth/refresh");
			// Reload the page to get fresh user data
			window.location.reload();
		} catch (err) {
			console.error("Failed to refresh session:", err);
			showToast(
				"Session refresh failed. Please log out and log back in.",
				"error",
			);
		}
	}

	async function handleDefaultTierChange() {
		settingsLoading = true;
		error = "";

		try {
			const updatedSettings = await adminApi.updateSetting(
				"default_user_tier",
				defaultUserTier,
			);
			defaultUserTier = updatedSettings.default_user_tier || "free";
			showToast(`Default tier updated to ${defaultUserTier}`, "success");
		} catch (err) {
			const apiError = err as ApiError;
			const errorMessage = apiError.message || "Failed to update setting";
			if (apiError.status === 403) {
				error =
					"Access denied. Your session may have stale permissions.";
				showToast(
					"Access denied. Click here to refresh your session.",
					"error",
					true,
				);
			} else {
				error = errorMessage;
				showToast(errorMessage, "error");
			}
		} finally {
			settingsLoading = false;
		}
	}

	function handleTierChange(
		userId: string,
		orgId: string,
		currentTier: string,
	) {
		confirmingTierChange = { userId, orgId, currentTier };
	}

	async function confirmTierChange() {
		if (!confirmingTierChange) return;

		tierLoading = true;
		error = "";

		try {
			const newTier =
				confirmingTierChange.currentTier === "free"
					? "unlimited"
					: "free";
			await adminApi.updateOrgTier(
				confirmingTierChange.orgId,
				newTier as "free" | "unlimited",
			);
			// Update the local user's tier display
			// Since tier is on the org, update all users with the same org_id
			const orgId = confirmingTierChange.orgId;
			users = users.map((u) => {
				if (u.org_id === orgId) {
					return { ...u, _tier: newTier } as any;
				}
				return u;
			});
			// Store tier in a map for display
			orgTiers[orgId] = newTier;
		} catch (err) {
			const apiError = err as ApiError;
			error = apiError.message || "Failed to update tier";
		} finally {
			tierLoading = false;
			confirmingTierChange = null;
		}
	}

	function cancelTierChange() {
		confirmingTierChange = null;
	}

	// Track org tiers locally (since User doesn't have tier field)
	let orgTiers = $state<Record<string, string>>({});

	function getOrgTier(orgId: string): string {
		return orgTiers[orgId] || "free";
	}

	function formatDate(timestamp: number): string {
		return new Date(timestamp * 1000).toLocaleDateString("en-US", {
			year: "numeric",
			month: "short",
			day: "numeric",
		});
	}

	let totalPages = $derived(Math.ceil(total / 50));
	let currentUser = $derived(data.user as User);
</script>

<svelte:head>
	<title>Admin Dashboard - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50">
	{#if data.user}
		<Header user={data.user} currentPage="admin" />

		<main class="container mx-auto px-4 py-8">
			<div class="max-w-6xl mx-auto">
				<div class="mb-8">
					<h1 class="text-3xl font-bold text-gray-900">
						Admin Dashboard
					</h1>
					<p class="text-gray-500 mt-1">
						Manage instance users and roles
					</p>
				</div>

				<!-- Error Message -->
				{#if error}
					<div
						class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-6"
					>
						{error}
					</div>
				{/if}

				<!-- Instance Settings -->
				<div
					class="bg-white rounded-lg shadow border border-gray-200 overflow-hidden mb-8"
				>
					<div class="px-6 py-4 border-b border-gray-200">
						<h2 class="text-lg font-semibold text-gray-900">
							Instance Settings
						</h2>
					</div>
					<div class="px-6 py-4 space-y-6">
						<div class="flex items-center justify-between">
							<div>
								<h3 class="text-sm font-medium text-gray-900">
									Allow new signups
								</h3>
								<p class="text-sm text-gray-500 mt-0.5">
									When disabled, only existing users can log
									in. New users will be blocked from creating
									accounts.
								</p>
							</div>
							<button
								onclick={handleSignupToggle}
								disabled={settingsLoading}
								class="relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed {signupsEnabled
									? 'bg-orange-500'
									: 'bg-gray-200'}"
								role="switch"
								aria-checked={signupsEnabled}
								aria-label="Toggle new signups"
							>
								<span
									class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {signupsEnabled
										? 'translate-x-5'
										: 'translate-x-0'}"
								></span>
							</button>
						</div>

						<div class="border-t border-gray-100 pt-6">
							<div class="flex items-center justify-between">
								<div>
									<h3
										class="text-sm font-medium text-gray-900"
									>
										Default tier for new users
									</h3>
									<p class="text-sm text-gray-500 mt-0.5">
										New signups will be assigned this tier.
									</p>
								</div>
								<div class="flex items-center gap-3">
									<select
										bind:value={defaultUserTier}
										onchange={handleDefaultTierChange}
										disabled={settingsLoading}
										class="block px-3 py-2 pr-8 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-orange-500 focus:border-orange-500 disabled:opacity-50 disabled:cursor-not-allowed appearance-none bg-white"
									>
										<option value="free">Free</option>
										<option value="unlimited"
											>Unlimited</option
										>
									</select>
								</div>
							</div>
						</div>
					</div>
				</div>

				<!-- Users Table -->
				<div
					class="bg-white rounded-lg shadow border border-gray-200 overflow-hidden"
				>
					<div class="px-6 py-4 border-b border-gray-200">
						<div class="flex items-center justify-between">
							<h2 class="text-lg font-semibold text-gray-900">
								Users
							</h2>
							<span
								class="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-orange-100 text-orange-800"
							>
								{total} user{total !== 1 ? "s" : ""}
							</span>
						</div>
					</div>

					{#if loading && users.length === 0}
						<div class="px-6 py-12 text-center text-gray-500">
							Loading users...
						</div>
					{:else if users.length === 0}
						<div class="px-6 py-12 text-center text-gray-500">
							No users found.
						</div>
					{:else}
						<div class="overflow-x-auto">
							<table class="w-full">
								<thead class="bg-gray-50">
									<tr>
										<th
											class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
											>User</th
										>
										<th
											class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
											>Email</th
										>
										<th
											class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
											>Provider</th
										>
										<th
											class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
											>Role</th
										>
										<th
											class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
											>Tier</th
										>
										<th
											class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider"
											>Joined</th
										>
										<th
											class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider"
											>Actions</th
										>
									</tr>
								</thead>
								<tbody class="divide-y divide-gray-200">
									{#each users as user (user.id)}
										<tr
											class="hover:bg-gray-50 transition-colors"
										>
											<td
												class="px-6 py-4 whitespace-nowrap"
											>
												<div
													class="flex items-center gap-3"
												>
													{#if user.avatar_url}
														<img
															src={user.avatar_url}
															alt={user.name ||
																user.email}
															class="w-8 h-8 rounded-full"
														/>
													{:else}
														<div
															class="w-8 h-8 rounded-full bg-gray-300 flex items-center justify-center"
														>
															<span
																class="text-gray-600 font-medium text-sm"
															>
																{(
																	user.name ||
																	user.email
																)
																	.charAt(0)
																	.toUpperCase()}
															</span>
														</div>
													{/if}
													<div>
														<div
															class="text-sm font-medium text-gray-900"
														>
															{user.name ||
																"No name"}
														</div>
														{#if user.id === currentUser.id}
															<span
																class="text-xs text-gray-400"
																>(you)</span
															>
														{/if}
													</div>
												</div>
											</td>
											<td
												class="px-6 py-4 whitespace-nowrap text-sm text-gray-600"
											>
												{user.email}
											</td>
											<td
												class="px-6 py-4 whitespace-nowrap"
											>
												<span
													class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800 capitalize"
												>
													{user.oauth_provider}
												</span>
											</td>
											<td
												class="px-6 py-4 whitespace-nowrap"
											>
												{#if user.role === "admin"}
													<span
														class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-orange-100 text-orange-800"
													>
														Admin
													</span>
												{:else}
													<span
														class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-600"
													>
														Member
													</span>
												{/if}
											</td>
											<td
												class="px-6 py-4 whitespace-nowrap"
											>
												<button
													onclick={() =>
														handleTierChange(
															user.id,
															user.org_id,
															getOrgTier(
																user.org_id,
															),
														)}
													disabled={tierLoading}
													class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium cursor-pointer transition-colors disabled:opacity-50 disabled:cursor-not-allowed {getOrgTier(
														user.org_id,
													) === 'unlimited'
														? 'bg-green-100 text-green-800 hover:bg-green-200'
														: 'bg-blue-100 text-blue-800 hover:bg-blue-200'}"
													title="Click to change tier"
												>
													{getOrgTier(user.org_id) ===
													"unlimited"
														? "Unlimited"
														: "Free"}
												</button>
											</td>
											<td
												class="px-6 py-4 whitespace-nowrap text-sm text-gray-500"
											>
												{formatDate(user.created_at)}
											</td>
											<td
												class="px-6 py-4 whitespace-nowrap text-right"
											>
												{#if user.id === currentUser.id}
													<span
														class="text-xs text-gray-400 italic"
														>Cannot edit self</span
													>
												{:else if user.role === "member"}
													<button
														onclick={() =>
															handleRoleChange(
																user.id,
																"admin",
															)}
														disabled={loading}
														class="text-sm text-orange-600 hover:text-orange-800 font-medium disabled:opacity-50 disabled:cursor-not-allowed"
													>
														Promote to Admin
													</button>
												{:else}
													<button
														onclick={() =>
															handleRoleChange(
																user.id,
																"member",
															)}
														disabled={loading}
														class="text-sm text-red-600 hover:text-red-800 font-medium disabled:opacity-50 disabled:cursor-not-allowed"
													>
														Demote to Member
													</button>
												{/if}
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}

					<!-- Pagination -->
					{#if totalPages > 1}
						<div
							class="px-6 py-4 border-t border-gray-200 flex items-center justify-between"
						>
							<p class="text-sm text-gray-500">
								Page {currentPage} of {totalPages} ({total} total
								users)
							</p>
							<div class="flex gap-2">
								<button
									onclick={() =>
										handlePageChange(currentPage - 1)}
									disabled={currentPage <= 1 || loading}
									class="px-3 py-1 text-sm border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
								>
									Previous
								</button>
								<button
									onclick={() =>
										handlePageChange(currentPage + 1)}
									disabled={currentPage >= totalPages ||
										loading}
									class="px-3 py-1 text-sm border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50 disabled:cursor-not-allowed"
								>
									Next
								</button>
							</div>
						</div>
					{/if}
				</div>
			</div>
		</main>
	{/if}

	<!-- Confirmation Modal -->
	{#if confirmingUserId}
		{@const targetUser = users.find((u) => u.id === confirmingUserId)}
		<div
			class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		>
			<div class="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
				<h3 class="text-lg font-semibold text-gray-900 mb-2">
					Confirm Role Change
				</h3>
				<p class="text-gray-600 mb-6">
					{#if confirmingRole === "admin"}
						Are you sure you want to promote <strong
							>{targetUser?.name || targetUser?.email}</strong
						>
						to <strong>Admin</strong>? They will have full control
						over this Rushomon instance.
					{:else}
						Are you sure you want to demote <strong
							>{targetUser?.name || targetUser?.email}</strong
						>
						to <strong>Member</strong>? They will lose admin
						privileges.
					{/if}
				</p>
				<div class="flex justify-end gap-3">
					<button
						onclick={cancelRoleChange}
						class="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
					>
						Cancel
					</button>
					<button
						onclick={confirmRoleChange}
						disabled={loading}
						class="px-4 py-2 text-sm font-medium text-white rounded-lg transition-colors disabled:opacity-50
							{confirmingRole === 'admin'
							? 'bg-orange-600 hover:bg-orange-700'
							: 'bg-red-600 hover:bg-red-700'}"
					>
						{#if loading}
							Updating...
						{:else if confirmingRole === "admin"}
							Promote to Admin
						{:else}
							Demote to Member
						{/if}
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Signup Toggle Confirmation Modal -->
	{#if confirmingSignupToggle}
		<div
			class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		>
			<div class="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
				<h3 class="text-lg font-semibold text-gray-900 mb-2">
					{signupsEnabled
						? "Disable New Signups?"
						: "Enable New Signups?"}
				</h3>
				<p class="text-gray-600 mb-6">
					{#if signupsEnabled}
						Are you sure you want to <strong
							>disable new signups</strong
						>? New users will no longer be able to create accounts.
						Existing users can still log in.
					{:else}
						Are you sure you want to <strong
							>enable new signups</strong
						>? Anyone with access to this instance will be able to
						create an account.
					{/if}
				</p>
				<div class="flex justify-end gap-3">
					<button
						onclick={cancelSignupToggle}
						class="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
					>
						Cancel
					</button>
					<button
						onclick={confirmSignupToggle}
						disabled={settingsLoading}
						class="px-4 py-2 text-sm font-medium text-white rounded-lg transition-colors disabled:opacity-50
							{signupsEnabled
							? 'bg-red-600 hover:bg-red-700'
							: 'bg-orange-600 hover:bg-orange-700'}"
					>
						{#if settingsLoading}
							Updating...
						{:else if signupsEnabled}
							Disable Signups
						{:else}
							Enable Signups
						{/if}
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Org Tier Change Confirmation Modal -->
	{#if confirmingTierChange}
		{@const targetUser = users.find(
			(u) => u.id === confirmingTierChange?.userId,
		)}
		{@const newTier =
			confirmingTierChange.currentTier === "free" ? "unlimited" : "free"}
		<div
			class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
		>
			<div class="bg-white rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
				<h3 class="text-lg font-semibold text-gray-900 mb-2">
					Change Organization Tier?
				</h3>
				<p class="text-gray-600 mb-6">
					{#if newTier === "unlimited"}
						Are you sure you want to upgrade <strong
							>{targetUser?.name || targetUser?.email}</strong
						>'s organization to <strong>Unlimited</strong>? They
						will have no usage limits.
					{:else}
						Are you sure you want to downgrade <strong
							>{targetUser?.name || targetUser?.email}</strong
						>'s organization to <strong>Free</strong>? They will be
						subject to free tier limits (25 links/month, 1,000
						clicks/month).
					{/if}
				</p>
				<div class="flex justify-end gap-3">
					<button
						onclick={cancelTierChange}
						class="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-lg hover:bg-gray-200 transition-colors"
					>
						Cancel
					</button>
					<button
						onclick={confirmTierChange}
						disabled={tierLoading}
						class="px-4 py-2 text-sm font-medium text-white rounded-lg transition-colors disabled:opacity-50
							{newTier === 'unlimited'
							? 'bg-green-600 hover:bg-green-700'
							: 'bg-orange-600 hover:bg-orange-700'}"
					>
						{#if tierLoading}
							Updating...
						{:else if newTier === "unlimited"}
							Upgrade to Unlimited
						{:else}
							Downgrade to Free
						{/if}
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Toast Notification -->
	{#if toast.visible}
		<div
			class="fixed top-4 right-4 z-50 transform transition-all duration-300 ease-in-out {toast.visible
				? 'translate-x-0 opacity-100'
				: 'translate-x-full opacity-0'}"
		>
			<div
				class="px-4 py-3 rounded-lg shadow-lg border {toast.type ===
				'success'
					? 'bg-green-50 border-green-200 text-green-700'
					: 'bg-red-50 border-red-200 text-red-700'}"
			>
				<div class="flex items-center gap-2">
					{#if toast.type === "success"}
						<svg
							class="w-5 h-5"
							fill="currentColor"
							viewBox="0 0 20 20"
						>
							<path
								fill-rule="evenodd"
								d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
								clip-rule="evenodd"
							/>
						</svg>
					{:else}
						<svg
							class="w-5 h-5"
							fill="currentColor"
							viewBox="0 0 20 20"
						>
							<path
								fill-rule="evenodd"
								d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
								clip-rule="evenodd"
							/>
						</svg>
					{/if}
					<span class="text-sm font-medium">{toast.message}</span>
				</div>
			</div>
		</div>
	{/if}
</div>
