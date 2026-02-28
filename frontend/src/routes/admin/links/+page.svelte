<script lang="ts">
	import { onMount } from "svelte";
	import { adminApi } from "$lib/api/admin";
	import type { AdminLink, ApiError } from "$lib/types/api";

	let links = $state<AdminLink[]>([]);
	let total = $state(0);
	let loading = $state(false);
	let error = $state("");
	let currentPage = $state(1);
	let orgFilter = $state("");
	let emailFilter = $state("");
	let domainFilter = $state("");
	let confirmingLinkId = $state<string | null>(null);
	let confirmingAction = $state<"delete" | "block" | null>(null);
	let blockDestination = $state("");
	let blockMatchType = $state<"exact" | "domain">("exact");
	let blockReason = $state("");
	let activeDropdown = $state<string | null>(null);
	let dropdownPosition = $state<{ top: number; right: number } | null>(null);
	let toast = $state<{
		message: string;
		type: "success" | "error";
		visible: boolean;
	}>({
		message: "",
		type: "success",
		visible: false,
	});

	const totalPages = $derived(Math.ceil(total / 50));

	onMount(() => {
		loadLinks();
	});

	async function loadLinks() {
		try {
			loading = true;
			const response = await adminApi.listLinks(
				currentPage,
				50,
				orgFilter,
				emailFilter,
				domainFilter,
			);
			links = response.links;
			total = response.total;
		} catch (err) {
			error = "Failed to load links";
			console.error(err);
		} finally {
			loading = false;
		}
	}

	async function handleUpdateStatus(
		linkId: string,
		status: "active" | "disabled" | "blocked",
	) {
		try {
			await adminApi.updateLinkStatus(linkId, status);
			await loadLinks();
			showToast(`Link ${status} successfully`, "success");
		} catch (err) {
			showToast(
				(err as ApiError).message || "Failed to update link status",
				"error",
			);
		}
	}

	async function handleDeleteLink(linkId: string) {
		try {
			await adminApi.deleteLink(linkId);
			await loadLinks();
			showToast("Link deleted successfully", "success");
		} catch (err) {
			showToast(
				(err as ApiError).message || "Failed to delete link",
				"error",
			);
		}
		closeConfirm();
	}

	async function handleBlockDestination() {
		try {
			const response = await adminApi.blockDestination(
				blockDestination,
				blockMatchType,
				blockReason,
			);
			await loadLinks();
			const action = blockMatchType === "domain" ? "domain" : "URL";
			showToast(
				`Blocked ${action} - ${response.blocked_links} links affected`,
				"success",
			);
		} catch (err) {
			showToast(
				(err as ApiError).message || "Failed to block destination",
				"error",
			);
		}
		closeConfirm();
	}

	function toggleDropdown(linkId: string, event: MouseEvent) {
		const button = event.currentTarget as HTMLElement;
		const rect = button.getBoundingClientRect();

		if (activeDropdown === linkId) {
			activeDropdown = null;
			dropdownPosition = null;
		} else {
			activeDropdown = linkId;
			dropdownPosition = {
				top: rect.bottom + 4,
				right: window.innerWidth - rect.right,
			};
		}
	}

	function closeDropdown() {
		activeDropdown = null;
		dropdownPosition = null;
	}

	function confirmDelete(linkId: string) {
		confirmingLinkId = linkId;
		confirmingAction = "delete";
	}

	function confirmBlock(
		linkId: string,
		destination: string,
		matchType: "exact" | "domain" = "exact",
	) {
		confirmingLinkId = linkId;
		confirmingAction = "block";
		blockDestination = destination;
		blockMatchType = matchType;
		blockReason = "";
	}

	function confirmBlockDomain(linkId: string, destination: string) {
		confirmBlock(linkId, destination, "domain");
	}

	function closeConfirm() {
		confirmingLinkId = null;
		confirmingAction = null;
		blockDestination = "";
		blockMatchType = "exact";
		blockReason = "";
	}

	async function handleSearch() {
		currentPage = 1;
		await loadLinks();
	}

	async function handlePageChange(page: number) {
		if (page < 1 || page > totalPages) return;
		currentPage = page;
		await loadLinks();
	}

	function showToast(message: string, type: "success" | "error") {
		toast.message = message;
		toast.type = type;
		toast.visible = true;
		setTimeout(() => {
			toast.visible = false;
		}, 3000);
	}

	function getStatusBadge(status: string): string {
		switch (status) {
			case "active":
				return "success";
			case "disabled":
				return "warning";
			case "blocked":
				return "danger";
			default:
				return "secondary";
		}
	}

	function formatDate(timestamp: number): string {
		return new Date(timestamp * 1000).toLocaleDateString();
	}
</script>

<div class="links-page">
	<div class="page-header">
		<h1>Links Management</h1>
		<p class="subtitle">Monitor and manage all links on the platform</p>
	</div>

	<!-- Filters -->
	<div class="filters">
		<div class="filter-group">
			<input
				type="text"
				placeholder="Filter by organization..."
				bind:value={orgFilter}
				onkeyup={(e) => e.key === "Enter" && handleSearch()}
				class="filter-input"
			/>
		</div>
		<div class="filter-group">
			<input
				type="text"
				placeholder="Filter by email..."
				bind:value={emailFilter}
				onkeyup={(e) => e.key === "Enter" && handleSearch()}
				class="filter-input"
			/>
		</div>
		<div class="filter-group">
			<input
				type="text"
				placeholder="Filter by domain..."
				bind:value={domainFilter}
				onkeyup={(e) => e.key === "Enter" && handleSearch()}
				class="filter-input"
			/>
		</div>
		<div class="filter-group">
			<button onclick={handleSearch} class="btn btn-primary"
				>Search</button
			>
		</div>
	</div>

	{#if loading && links.length === 0}
		<div class="loading">Loading links...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else}
		<div class="links-table">
			<table>
				<thead>
					<tr>
						<th>Short Code</th>
						<th>Destination</th>
						<th>Creator</th>
						<th>Organization</th>
						<th>Status</th>
						<th>Clicks</th>
						<th>Created</th>
						<th>Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each links as link (link.id)}
						<tr>
							<td>
								<div class="short-code">
									<code>{link.short_code}</code>
								</div>
							</td>
							<td>
								<div class="destination">
									{link.destination_url}
								</div>
							</td>
							<td>
								<div class="creator">{link.creator_email}</div>
							</td>
							<td>
								<div class="org">{link.org_name}</div>
							</td>
							<td>
								<span
									class="badge {getStatusBadge(link.status)}"
									>{link.status}</span
								>
							</td>
							<td>{link.click_count}</td>
							<td>{formatDate(link.created_at)}</td>
							<td>
								<div class="dropdown-container">
									<button
										class="dropdown-toggle"
										onclick={(e) =>
											toggleDropdown(link.id, e)}
										aria-label="Actions"
										aria-expanded={activeDropdown ===
											link.id}
									>
										⋮
									</button>
									{#if activeDropdown === link.id}
										<div
											class="dropdown-menu"
											style="top: {dropdownPosition?.top ||
												0}px; right: {dropdownPosition?.right ||
												0}px;"
										>
											{#if link.status === "active"}
												<button
													class="dropdown-item"
													onclick={() => {
														handleUpdateStatus(
															link.id,
															"disabled",
														);
														closeDropdown();
													}}
												>
													Disable
												</button>
											{:else if link.status === "disabled"}
												<button
													class="dropdown-item success"
													onclick={() => {
														handleUpdateStatus(
															link.id,
															"active",
														);
														closeDropdown();
													}}
												>
													Enable
												</button>
											{/if}
											<button
												class="dropdown-item danger"
												onclick={() => {
													confirmDelete(link.id);
													closeDropdown();
												}}
											>
												Delete
											</button>
											<button
												class="dropdown-item danger"
												onclick={() => {
													confirmBlock(
														link.id,
														link.destination_url,
														"exact",
													);
													closeDropdown();
												}}
											>
												Block Destination URL
											</button>
											<button
												class="dropdown-item danger"
												onclick={() => {
													confirmBlockDomain(
														link.id,
														link.destination_url,
													);
													closeDropdown();
												}}
											>
												Block Destination Domain
											</button>
										</div>
									{/if}
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>

			{#if links.length === 0}
				<div class="empty-state">No links found</div>
			{/if}
		</div>

		<!-- Pagination -->
		{#if totalPages > 1}
			<div class="pagination">
				<p class="pagination-info">
					Page {currentPage} of {totalPages} ({total} total links)
				</p>
				<div class="pagination-controls">
					<button
						onclick={() => handlePageChange(currentPage - 1)}
						disabled={currentPage <= 1 || loading}
						class="pagination-btn"
					>
						Previous
					</button>
					<button
						onclick={() => handlePageChange(currentPage + 1)}
						disabled={currentPage >= totalPages || loading}
						class="pagination-btn"
					>
						Next
					</button>
				</div>
			</div>
		{/if}
	{/if}
</div>

<!-- Delete Confirmation Modal -->
{#if confirmingAction === "delete"}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={closeConfirm}
		onkeydown={(e) => e.key === "Enter" && closeConfirm()}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			tabindex="0"
			onkeydown={(e) => e.key === "Escape" && closeConfirm()}
		>
			<div class="modal-header">
				<h3>Delete Link?</h3>
				<button class="modal-close" onclick={closeConfirm}
					>&times;</button
				>
			</div>
			<div class="modal-body">
				<p>
					Are you sure you want to delete this link? This action
					cannot be undone.
				</p>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" onclick={closeConfirm}>
					Cancel
				</button>
				<button
					class="btn btn-danger"
					onclick={() =>
						confirmingLinkId && handleDeleteLink(confirmingLinkId)}
				>
					Delete
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Block Destination Modal -->
{#if confirmingAction === "block"}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={closeConfirm}
		onkeydown={(e) => e.key === "Enter" && closeConfirm()}
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			tabindex="0"
			onkeydown={(e) => e.key === "Escape" && closeConfirm()}
		>
			<div class="modal-header">
				<h3>
					{blockMatchType === "domain"
						? "Block Destination Domain"
						: "Block Destination URL"}
				</h3>
				<button class="modal-close" onclick={closeConfirm}
					>&times;</button
				>
			</div>
			<div class="modal-body">
				<div class="form-group">
					<label for="block-destination"
						>Destination {blockMatchType === "domain"
							? "Domain"
							: "URL"}</label
					>
					<input
						id="block-destination"
						type="text"
						bind:value={blockDestination}
						readonly
						class="form-input"
					/>
				</div>
				<div class="form-group">
					<label for="block-match-type">Match Type</label>
					<input
						id="block-match-type"
						type="text"
						value={blockMatchType === "domain"
							? "Domain (blocks all URLs from this domain)"
							: "Exact URL (blocks only this specific URL)"}
						readonly
						class="form-input"
					/>
				</div>
				<div class="form-group">
					<label for="block-reason">Reason</label>
					<textarea
						id="block-reason"
						bind:value={blockReason}
						placeholder="Reason for blocking..."
						class="form-textarea"
					></textarea>
				</div>
			</div>
			<div class="modal-footer">
				<button class="btn btn-secondary" onclick={closeConfirm}>
					Cancel
				</button>
				<button class="btn btn-danger" onclick={handleBlockDestination}>
					{blockMatchType === "domain" ? "Block Domain" : "Block URL"}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Toast Notification -->
{#if toast.visible}
	<div class="toast {toast.type}" class:visible={toast.visible}>
		{toast.message}
	</div>
{/if}

<!-- Click outside to close dropdown -->
{#if activeDropdown}
	<div
		class="dropdown-overlay"
		role="button"
		tabindex="0"
		onclick={closeDropdown}
		onkeydown={(e) => e.key === "Enter" && closeDropdown()}
	></div>
{/if}

<style>
	.links-page {
		max-width: 1400px;
		margin: 0 auto;
	}

	.page-header {
		margin-bottom: 2rem;
	}

	.page-header h1 {
		margin: 0 0 0.5rem 0;
		font-size: 1.75rem;
		font-weight: 600;
		color: #1e293b;
	}

	.subtitle {
		margin: 0;
		color: #64748b;
		font-size: 1rem;
	}

	.filters {
		display: flex;
		gap: 1rem;
		margin-bottom: 2rem;
		flex-wrap: wrap;
	}

	.filter-group {
		flex: 1;
		min-width: 200px;
	}

	.filter-input {
		width: 100%;
		padding: 0.5rem 1rem;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 0.875rem;
	}

	.loading,
	.error {
		text-align: center;
		padding: 3rem;
		color: #64748b;
	}

	.error {
		color: #dc2626;
	}

	.links-table {
		background: white;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		overflow: hidden;
	}

	.links-table table {
		width: 100%;
		border-collapse: collapse;
	}

	.links-table th {
		text-align: left;
		padding: 1rem;
		background: #f8fafc;
		border-bottom: 1px solid #e2e8f0;
		font-weight: 600;
		color: #374151;
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.links-table td {
		padding: 1rem;
		border-bottom: 1px solid #f1f5f9;
	}

	.links-table tr:hover {
		background: #f8fafc;
	}

	.short-code {
		font-family: monospace;
		font-weight: 500;
	}

	.destination {
		max-width: 300px;
		word-break: break-all;
	}

	.creator,
	.org {
		color: #64748b;
		font-size: 0.875rem;
	}

	.badge {
		display: inline-flex;
		align-items: center;
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.badge.success {
		background: #d1fae5;
		color: #065f46;
	}

	.badge.warning {
		background: #fef3c7;
		color: #92400e;
	}

	.badge.danger {
		background: #fee2e2;
		color: #991b1b;
	}

	.badge.secondary {
		background: #f3f4f6;
		color: #6b7280;
	}

	.empty-state {
		text-align: center;
		padding: 3rem;
		color: #64748b;
	}

	.pagination {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		background: #f8fafc;
		border-top: 1px solid #e2e8f0;
	}

	.pagination-info {
		color: #64748b;
		font-size: 0.875rem;
		margin: 0;
	}

	.pagination-controls {
		display: flex;
		gap: 0.5rem;
	}

	.pagination-btn {
		padding: 0.5rem 1rem;
		border: 1px solid #d1d5db;
		background: white;
		color: #374151;
		border-radius: 6px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.pagination-btn:hover:not(:disabled) {
		background: #f9fafb;
		border-color: #9ca3af;
	}

	.pagination-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
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

	.btn-primary {
		background: #3b82f6;
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		background: #2563eb;
	}

	.btn-danger {
		background: #dc2626;
		color: white;
	}

	.btn-danger:hover:not(:disabled) {
		background: #b91c1c;
	}

	.btn-secondary {
		background: #64748b;
		color: white;
	}

	.btn-secondary:hover:not(:disabled) {
		background: #475569;
	}

	/* Modal */
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
		max-width: 500px;
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

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid #e2e8f0;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: #374151;
	}

	.form-input,
	.form-textarea {
		width: 100%;
		padding: 0.5rem 1rem;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 0.875rem;
	}

	.form-textarea {
		resize: vertical;
		min-height: 80px;
	}

	/* Toast */
	.toast {
		position: fixed;
		top: 1rem;
		right: 1rem;
		padding: 1rem 1.5rem;
		border-radius: 6px;
		color: white;
		font-weight: 500;
		z-index: 1001;
		opacity: 0;
		transform: translateY(-10px);
		transition: all 0.2s;
	}

	.toast.visible {
		opacity: 1;
		transform: translateY(0);
	}

	.toast.success {
		background: #059669;
	}

	.toast.error {
		background: #dc2626;
	}

	/* Dropdown Styles */
	.dropdown-container {
		position: relative;
	}

	.dropdown-toggle {
		background: none;
		border: none;
		font-size: 1.25rem;
		cursor: pointer;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		color: #6b7280;
		transition: all 0.2s;
	}

	.dropdown-toggle:hover {
		background: #f3f4f6;
		color: #374151;
	}

	.dropdown-menu {
		position: fixed;
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 6px;
		box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
		min-width: 160px;
		z-index: 1000;
		overflow: visible;
	}

	.dropdown-item {
		display: block;
		width: 100%;
		padding: 0.5rem 1rem;
		border: none;
		background: none;
		text-align: left;
		font-size: 0.875rem;
		color: #374151;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.dropdown-item:hover {
		background: #f9fafb;
	}

	.dropdown-item.danger {
		color: #dc2626;
	}

	.dropdown-item.warning {
		color: #d97706;
	}

	.dropdown-item.secondary {
		color: #6b7280;
	}

	.dropdown-item.success {
		color: #059669;
	}

	.dropdown-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 40;
	}

	/* Responsive */
	@media (max-width: 768px) {
		.filters {
			flex-direction: column;
		}

		.filter-group {
			min-width: auto;
		}

		.links-table {
			font-size: 0.875rem;
		}

		.links-table th,
		.links-table td {
			padding: 0.75rem 0.5rem;
		}

		.pagination {
			flex-direction: column;
			gap: 1rem;
			align-items: stretch;
		}

		.pagination-controls {
			justify-content: center;
		}
	}
</style>
