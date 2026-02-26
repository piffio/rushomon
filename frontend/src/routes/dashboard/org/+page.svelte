<script lang="ts">
	import Header from "$lib/components/Header.svelte";
	import { orgsApi } from "$lib/api/orgs";
	import type { PageData } from "./$types";
	import type {
		OrgDetails,
		OrgMember,
		OrgInvitation,
		OrgWithRole,
	} from "$lib/types/api";

	let { data }: { data: PageData } = $props();

	let orgDetails = $state<OrgDetails | null>(null);
	let loading = $state(true);
	let error = $state("");

	// Rename org
	let editingName = $state(false);
	let newOrgName = $state("");
	let savingName = $state(false);
	let nameError = $state("");

	// Invite member
	let inviteEmail = $state("");
	let inviting = $state(false);
	let inviteError = $state("");
	let inviteSuccess = $state("");

	// General feedback
	let actionError = $state("");
	let actionSuccess = $state("");

	async function loadOrg() {
		loading = true;
		error = "";
		try {
			const orgsRes = await orgsApi.listMyOrgs();
			const currentOrgId = orgsRes.current_org_id;
			if (!currentOrgId) {
				error = "No active organization found.";
				return;
			}
			orgDetails = await orgsApi.getOrg(currentOrgId);
		} catch (e: any) {
			error = e?.message ?? "Failed to load organization details.";
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		loadOrg();
	});

	function startEditName() {
		if (!orgDetails) return;
		newOrgName = orgDetails.org.name;
		editingName = true;
		nameError = "";
	}

	async function saveOrgName() {
		if (!orgDetails) return;
		const trimmed = newOrgName.trim();
		if (!trimmed) {
			nameError = "Name cannot be empty.";
			return;
		}
		if (trimmed.length > 100) {
			nameError = "Name must be 100 characters or less.";
			return;
		}
		savingName = true;
		nameError = "";
		try {
			await orgsApi.updateOrgName(orgDetails.org.id, trimmed);
			orgDetails.org.name = trimmed;
			editingName = false;
			actionSuccess = "Organization name updated.";
			setTimeout(() => (actionSuccess = ""), 3000);
		} catch (e: any) {
			nameError = e?.message ?? "Failed to save name.";
		} finally {
			savingName = false;
		}
	}

	async function handleInvite() {
		if (!orgDetails) return;
		const email = inviteEmail.trim().toLowerCase();
		if (!email || !email.includes("@")) {
			inviteError = "Please enter a valid email address.";
			return;
		}
		inviting = true;
		inviteError = "";
		inviteSuccess = "";
		try {
			await orgsApi.inviteMember(orgDetails.org.id, email);
			inviteSuccess = `Invitation sent to ${email}.`;
			inviteEmail = "";
			await loadOrg();
		} catch (e: any) {
			inviteError = e?.message ?? "Failed to send invitation.";
		} finally {
			inviting = false;
		}
	}

	async function handleRevokeInvitation(inv: OrgInvitation) {
		if (!orgDetails) return;
		actionError = "";
		actionSuccess = "";
		try {
			await orgsApi.revokeInvitation(orgDetails.org.id, inv.id);
			actionSuccess = `Invitation to ${inv.email} revoked.`;
			await loadOrg();
			setTimeout(() => (actionSuccess = ""), 3000);
		} catch (e: any) {
			actionError = e?.message ?? "Failed to revoke invitation.";
		}
	}

	async function handleResendInvitation(inv: OrgInvitation) {
		if (!orgDetails) return;
		actionError = "";
		actionSuccess = "";
		try {
			await orgsApi.resendInvitation(orgDetails.org.id, inv.id);
			actionSuccess = `Invitation resent to ${inv.email}.`;
			setTimeout(() => (actionSuccess = ""), 3000);
		} catch (e: any) {
			actionError = e?.message ?? "Failed to resend invitation.";
		}
	}

	async function handleRemoveMember(member: OrgMember) {
		if (!orgDetails) return;
		confirmingRemoveMember = member;
	}

	async function confirmRemoveMember() {
		if (!confirmingRemoveMember) return;
		actionError = "";
		actionSuccess = "";
		const memberEmail = confirmingRemoveMember.email;
		try {
			await orgsApi.removeMember(
				orgDetails!.org.id,
				confirmingRemoveMember.user_id,
			);
			actionSuccess = `${memberEmail} removed from the organization.`;
			await loadOrg();
			setTimeout(() => (actionSuccess = ""), 3000);
		} catch (e: any) {
			actionError = e?.message ?? "Failed to remove member.";
		} finally {
			confirmingRemoveMember = null;
		}
	}

	function cancelRemoveMember() {
		confirmingRemoveMember = null;
	}

	function formatDate(ts: number): string {
		return new Date(ts * 1000).toLocaleDateString(undefined, {
			year: "numeric",
			month: "short",
			day: "numeric",
		});
	}

	const isOwner = $derived(orgDetails?.org.role === "owner");
	const isUnlimited = $derived(orgDetails?.org.tier === "unlimited");

	// Delete organization
	let showDeleteModal = $state(false);
	let deleteAction = $state<"delete" | "migrate">("delete");
	let targetOrgId = $state("");
	let deleting = $state(false);
	let deleteError = $state("");
	let confirmingTierChange = $state<{
		userId: string;
		orgId: string;
		currentTier: string;
	} | null>(null);
	let confirmingRemoveMember = $state<OrgMember | null>(null);
	let orgTiers = $state<Record<string, string>>({});
	let linkCount = $state(0);
	let userOrgs = $state<OrgWithRole[]>([]);
	let canDelete = $state(false);

	async function checkCanDelete() {
		if (!orgDetails) return;
		try {
			const orgsRes = await orgsApi.listMyOrgs();
			const ownedOrgs = orgsRes.orgs.filter((o) => o.role === "owner");
			userOrgs = ownedOrgs.filter((o) => o.id !== orgDetails!.org.id);
			canDelete = ownedOrgs.length > 1;

			// Get link count from usage API
			const usageRes = await fetch("http://localhost:8787/api/usage", {
				credentials: "include",
			});
			if (usageRes.ok) {
				const usage = await usageRes.json();
				linkCount = usage.links_created_this_month || 0;
			}
		} catch (e) {
			canDelete = false;
		}
	}

	async function openDeleteModal() {
		await checkCanDelete();
		if (!canDelete) {
			actionError = "Cannot delete your only organization.";
			setTimeout(() => (actionError = ""), 3000);
			return;
		}
		showDeleteModal = true;
		deleteError = "";
		deleteAction = "delete";
		targetOrgId = userOrgs.length > 0 ? userOrgs[0].id : "";
	}

	async function handleDeleteOrg() {
		if (!orgDetails) return;
		deleting = true;
		deleteError = "";
		try {
			const result = await orgsApi.deleteOrg(
				orgDetails.org.id,
				deleteAction,
				deleteAction === "migrate" ? targetOrgId : undefined,
			);
			// Redirect to dashboard after successful deletion
			window.location.href = "/dashboard";
		} catch (e: any) {
			deleteError = e?.message ?? "Failed to delete organization.";
		} finally {
			deleting = false;
		}
	}
</script>

<div class="min-h-screen bg-gray-50">
	<Header user={data.user} currentPage="settings" />

	<main class="container mx-auto px-4 py-8 max-w-3xl">
		<div class="mb-6">
			<a
				href="/dashboard"
				class="text-sm text-gray-500 hover:text-orange-600 transition-colors flex items-center gap-1"
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
						d="M15 19l-7-7 7-7"
					/>
				</svg>
				Back to Dashboard
			</a>
			<h1 class="text-2xl font-bold text-gray-900 mt-2">
				Organization Settings
			</h1>
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-16">
				<div
					class="w-8 h-8 border-2 border-orange-500 border-t-transparent rounded-full animate-spin"
				></div>
			</div>
		{:else if error}
			<div
				class="bg-red-50 border border-red-200 rounded-xl p-4 text-red-700"
			>
				{error}
			</div>
		{:else if orgDetails}
			<!-- Global feedback -->
			{#if actionSuccess}
				<div
					class="mb-4 bg-green-50 border border-green-200 rounded-xl p-3 text-green-700 text-sm"
				>
					{actionSuccess}
				</div>
			{/if}
			{#if actionError}
				<div
					class="mb-4 bg-red-50 border border-red-200 rounded-xl p-3 text-red-700 text-sm"
				>
					{actionError}
				</div>
			{/if}

			<!-- Org Name Card -->
			<div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
				<h2 class="text-lg font-semibold text-gray-900 mb-4">
					Organization Name
				</h2>

				{#if editingName}
					<div class="flex items-center gap-3">
						<input
							type="text"
							bind:value={newOrgName}
							maxlength="100"
							class="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-orange-500 outline-none"
							onkeydown={(e) =>
								e.key === "Enter" && saveOrgName()}
						/>
						<button
							onclick={saveOrgName}
							disabled={savingName}
							class="px-4 py-2 bg-orange-500 hover:bg-orange-600 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
						>
							{savingName ? "Saving…" : "Save"}
						</button>
						<button
							onclick={() => (editingName = false)}
							class="px-4 py-2 text-gray-600 hover:text-gray-900 border border-gray-300 rounded-lg text-sm transition-colors"
						>
							Cancel
						</button>
					</div>
					{#if nameError}
						<p class="mt-2 text-sm text-red-600">{nameError}</p>
					{/if}
				{:else}
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-3">
							<span
								class="w-10 h-10 rounded-lg bg-gradient-to-br from-orange-400 to-orange-600 flex items-center justify-center text-white text-lg font-bold"
							>
								{orgDetails.org.name.charAt(0).toUpperCase()}
							</span>
							<div>
								<p class="font-semibold text-gray-900">
									{orgDetails.org.name}
								</p>
								<p class="text-xs text-gray-500 capitalize">
									{orgDetails.org.tier} plan · Created {formatDate(
										orgDetails.org.created_at,
									)}
								</p>
							</div>
						</div>
						{#if isOwner}
							<button
								onclick={startEditName}
								class="text-sm text-orange-600 hover:text-orange-700 font-medium transition-colors"
							>
								Rename
							</button>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Members Card -->
			<div class="bg-white rounded-xl border border-gray-200 p-6 mb-6">
				<h2 class="text-lg font-semibold text-gray-900 mb-4">
					Members
					<span class="text-sm font-normal text-gray-500 ml-1"
						>({orgDetails.members.length})</span
					>
				</h2>

				<ul class="divide-y divide-gray-100">
					{#each orgDetails.members as member}
						<li class="flex items-center justify-between py-3">
							<div class="flex items-center gap-3">
								{#if member.avatar_url}
									<img
										src={member.avatar_url}
										alt={member.name ?? member.email}
										class="w-8 h-8 rounded-full"
									/>
								{:else}
									<div
										class="w-8 h-8 rounded-full bg-gray-200 flex items-center justify-center text-gray-500 text-sm font-medium"
									>
										{(member.name ?? member.email)
											.charAt(0)
											.toUpperCase()}
									</div>
								{/if}
								<div>
									<p
										class="text-sm font-medium text-gray-900"
									>
										{member.name ?? member.email}
									</p>
									{#if member.name}
										<p class="text-xs text-gray-500">
											{member.email}
										</p>
									{/if}
								</div>
							</div>
							<div class="flex items-center gap-3">
								<span
									class="text-xs font-medium capitalize px-2 py-0.5 rounded-full {member.role ===
									'owner'
										? 'bg-orange-100 text-orange-700'
										: 'bg-gray-100 text-gray-600'}"
								>
									{member.role}
								</span>
								{#if isOwner && member.user_id !== data.user?.id}
									<button
										onclick={() =>
											handleRemoveMember(member)}
										class="text-xs text-red-500 hover:text-red-700 transition-colors"
										aria-label="Remove member"
									>
										Remove
									</button>
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			</div>

			<!-- Invite Card (owner + unlimited only) -->
			{#if isOwner}
				<div
					class="bg-white rounded-xl border border-gray-200 p-6 mb-6"
				>
					<h2 class="text-lg font-semibold text-gray-900 mb-1">
						Invite Members
					</h2>

					{#if !isUnlimited}
						<div
							class="bg-amber-50 border border-amber-200 rounded-lg p-3 mt-2 text-sm text-amber-800"
						>
							Inviting members requires the <strong
								>Unlimited plan</strong
							>. Upgrade to collaborate with your team.
						</div>
					{:else}
						<p class="text-sm text-gray-500 mb-4">
							Send an invitation by email. The invitee will have 7
							days to accept.
						</p>
						<div class="flex items-start gap-3">
							<div class="flex-1">
								<input
									type="email"
									bind:value={inviteEmail}
									placeholder="colleague@example.com"
									class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-orange-500 outline-none"
									onkeydown={(e) =>
										e.key === "Enter" && handleInvite()}
								/>
								{#if inviteError}
									<p class="mt-1 text-sm text-red-600">
										{inviteError}
									</p>
								{/if}
								{#if inviteSuccess}
									<p class="mt-1 text-sm text-green-600">
										{inviteSuccess}
									</p>
								{/if}
							</div>
							<button
								onclick={handleInvite}
								disabled={inviting}
								class="px-4 py-2 bg-orange-500 hover:bg-orange-600 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50 whitespace-nowrap"
							>
								{inviting ? "Sending…" : "Send Invite"}
							</button>
						</div>

						<!-- Pending Invitations -->
						{#if orgDetails.pending_invitations.length > 0}
							<div class="mt-5">
								<h3
									class="text-sm font-medium text-gray-700 mb-2"
								>
									Pending Invitations
								</h3>
								<ul class="divide-y divide-gray-100">
									{#each orgDetails.pending_invitations as inv}
										<li
											class="flex items-center justify-between py-2.5"
										>
											<div>
												<p
													class="text-sm text-gray-900"
												>
													{inv.email}
												</p>
												<p
													class="text-xs text-gray-500"
												>
													Expires {formatDate(
														inv.expires_at,
													)}
												</p>
											</div>
											<div
												class="flex items-center gap-3"
											>
												<button
													onclick={() =>
														handleResendInvitation(
															inv,
														)}
													class="text-xs text-orange-500 hover:text-orange-700 transition-colors"
												>
													Resend
												</button>
												<button
													onclick={() =>
														handleRevokeInvitation(
															inv,
														)}
													class="text-xs text-red-500 hover:text-red-700 transition-colors"
												>
													Revoke
												</button>
											</div>
										</li>
									{/each}
								</ul>
							</div>
						{/if}
					{/if}
				</div>
			{/if}

			<!-- Danger Zone (owner only, multiple orgs) -->
			{#if isOwner}
				<div class="bg-white rounded-xl border border-red-200 p-6">
					<h2 class="text-lg font-semibold text-red-700 mb-2">
						Danger Zone
					</h2>
					<p class="text-sm text-gray-600 mb-4">
						Deleting an organization is permanent and cannot be
						undone.
					</p>
					<button
						onclick={openDeleteModal}
						class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm font-medium transition-colors"
					>
						Delete Organization
					</button>
				</div>
			{/if}
		{/if}
	</main>
</div>

<!-- Delete Organization Modal -->
{#if showDeleteModal && orgDetails}
	<div
		class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
	>
		<div class="bg-white rounded-xl max-w-lg w-full p-6 shadow-2xl">
			<h2 class="text-xl font-bold text-gray-900 mb-4">
				Delete Organization
			</h2>

			<div
				class="mb-4 bg-red-50 border border-red-200 rounded-lg p-3 text-sm text-red-800"
			>
				<strong>Warning:</strong> This action cannot be undone. You are
				about to delete <strong>{orgDetails.org.name}</strong>.
			</div>

			<p class="text-sm text-gray-700 mb-4">
				What would you like to do with the <strong
					>{linkCount} link{linkCount === 1 ? "" : "s"}</strong
				> in this organization?
			</p>

			<div class="space-y-3 mb-6">
				<!-- Delete All Option -->
				<label
					class="flex items-start gap-3 p-3 border-2 rounded-lg cursor-pointer transition-colors {deleteAction ===
					'delete'
						? 'border-orange-500 bg-orange-50'
						: 'border-gray-200 hover:border-gray-300'}"
				>
					<input
						type="radio"
						name="deleteAction"
						value="delete"
						bind:group={deleteAction}
						class="mt-0.5"
					/>
					<div class="flex-1">
						<div class="font-medium text-gray-900">
							Delete all links
						</div>
						<div class="text-sm text-gray-600 mt-1">
							All links and analytics data will be permanently
							removed.
						</div>
					</div>
				</label>

				<!-- Migrate Option -->
				{#if userOrgs.length > 0}
					<label
						class="flex items-start gap-3 p-3 border-2 rounded-lg cursor-pointer transition-colors {deleteAction ===
						'migrate'
							? 'border-orange-500 bg-orange-50'
							: 'border-gray-200 hover:border-gray-300'}"
					>
						<input
							type="radio"
							name="deleteAction"
							value="migrate"
							bind:group={deleteAction}
							class="mt-0.5"
						/>
						<div class="flex-1">
							<div class="font-medium text-gray-900">
								Migrate links to another organization
							</div>
							<div class="text-sm text-gray-600 mt-1 mb-2">
								Transfer all links to one of your other
								organizations.
							</div>
							{#if deleteAction === "migrate"}
								<select
									bind:value={targetOrgId}
									class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-orange-500 outline-none"
								>
									{#each userOrgs as org}
										<option value={org.id}
											>{org.name} ({org.tier} plan)</option
										>
									{/each}
								</select>
								<p class="text-xs text-gray-500 mt-1">
									Make sure the target organization has enough
									available slots.
								</p>
							{/if}
						</div>
					</label>
				{:else}
					<div
						class="p-3 bg-gray-50 border border-gray-200 rounded-lg text-sm text-gray-600"
					>
						You don't have other organizations to migrate links to.
					</div>
				{/if}
			</div>

			{#if deleteError}
				<div
					class="mb-4 bg-red-50 border border-red-200 rounded-lg p-3 text-sm text-red-700"
				>
					{deleteError}
				</div>
			{/if}

			<div class="flex gap-3 justify-end">
				<button
					onclick={() => {
						showDeleteModal = false;
						deleteError = "";
					}}
					disabled={deleting}
					class="px-4 py-2 text-gray-700 hover:text-gray-900 border border-gray-300 rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
				>
					Cancel
				</button>
				<button
					onclick={handleDeleteOrg}
					disabled={deleting ||
						(deleteAction === "migrate" && !targetOrgId)}
					class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50"
				>
					{deleting ? "Deleting…" : "Delete Organization"}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Remove Member Confirmation Modal -->
{#if confirmingRemoveMember}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={cancelRemoveMember}
		onkeydown={(e) => {
			if (e.key === "Enter" || e.key === " ") {
				e.preventDefault();
				cancelRemoveMember();
			}
		}}
	>
		<div class="modal" role="dialog" aria-modal="true" tabindex="-1">
			<div class="modal-header">
				<h3>Remove Member?</h3>
				<button class="modal-close" onclick={cancelRemoveMember}
					>&times;</button
				>
			</div>
			<div class="modal-body">
				<p>
					Are you sure you want to <strong
						>remove {confirmingRemoveMember.email}</strong
					>
					from this organization? They will lose access to all links and
					resources.
				</p>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" onclick={cancelRemoveMember}>
					Cancel
				</button>
				<button class="btn btn-danger" onclick={confirmRemoveMember}>
					Remove Member
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	/* Modal Styles */
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		border-radius: 8px;
		width: 90%;
		max-width: 400px;
		max-height: 90vh;
		overflow-y: auto;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid #e2e8f0;
	}

	.modal-header h3 {
		margin: 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: #1e293b;
	}

	.modal-close {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: #64748b;
		padding: 0;
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal-body {
		padding: 1.5rem;
	}

	.modal-body p {
		margin: 0;
		color: #475569;
		line-height: 1.5;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e2e8f0;
	}

	.btn {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn-secondary {
		background: #64748b;
		color: white;
	}

	.btn-secondary:hover:not(:disabled) {
		background: #475569;
	}

	.btn-danger {
		background: #dc2626;
		color: white;
	}

	.btn-danger:hover:not(:disabled) {
		background: #b91c1c;
	}

	.btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
