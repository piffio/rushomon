<script lang="ts">
	import { onMount } from "svelte";
	import { adminApi } from "$lib/api/admin";
	import type { BlacklistEntry } from "$lib/types/api";

	let entries = $state<BlacklistEntry[]>([]);
	let loading = $state(false);
	let showAddModal = $state(false);
	let showRemoveModal = $state(false);
	let selectedEntry = $state<BlacklistEntry | null>(null);
	let newDestination = $state("");
	let newReason = $state("");
	let showToast = $state(false);
	let toastMessage = $state("");

	onMount(() => {
		loadBlacklist();
	});

	async function loadBlacklist() {
		try {
			loading = true;
			entries = await adminApi.getBlacklist();
		} catch (err) {
			console.error("Failed to load blacklist:", err);
		} finally {
			loading = false;
		}
	}

	async function handleAddEntry() {
		if (!newDestination || !newReason) return;

		try {
			await adminApi.blockDestination(newDestination, "exact", newReason);
			await loadBlacklist();
			showAddModal = false;
			newDestination = "";
			newReason = "";
			showToastMessage("Destination added to blacklist");
		} catch (err) {
			console.error("Failed to add entry:", err);
			showToastMessage("Failed to add destination");
		}
	}

	function confirmRemove(entry: BlacklistEntry) {
		selectedEntry = entry;
		showRemoveModal = true;
	}

	async function handleRemoveEntry() {
		if (!selectedEntry) return;

		try {
			await adminApi.removeBlacklistEntry(selectedEntry.id);
			await loadBlacklist();
			showRemoveModal = false;
			selectedEntry = null;
			showToastMessage("Destination removed from blacklist");
		} catch (err) {
			console.error("Failed to remove entry:", err);
			showToastMessage("Failed to remove destination");
		}
	}

	function showToastMessage(message: string) {
		toastMessage = message;
		showToast = true;
		setTimeout(() => {
			showToast = false;
		}, 3000);
	}

	function getMatchTypeLabel(type: string) {
		switch (type) {
			case "exact":
				return "Exact URL";
			case "domain":
				return "Domain";
			default:
				return type;
		}
	}
</script>

<div class="blacklist-page">
	<div class="page-header">
		<h1>Destination Blacklist</h1>
		<button class="btn btn-primary" onclick={() => (showAddModal = true)}>
			Add Destination
		</button>
	</div>

	{#if loading}
		<div class="loading">Loading...</div>
	{:else if entries.length === 0}
		<div class="empty-state">
			<p>No destinations are blacklisted.</p>
		</div>
	{:else}
		<div class="table-container">
			<table class="data-table">
				<thead>
					<tr>
						<th>Destination</th>
						<th>Match Type</th>
						<th>Reason</th>
						<th>Added By</th>
						<th>Added At</th>
						<th>Actions</th>
					</tr>
				</thead>
				<tbody>
					{#each entries as entry}
						<tr>
							<td class="destination-cell">
								<code>{entry.destination}</code>
							</td>
							<td>
								<span class="badge badge-{entry.match_type}">
									{getMatchTypeLabel(entry.match_type)}
								</span>
							</td>
							<td>{entry.reason}</td>
							<td>{entry.created_by}</td>
							<td
								>{new Date(
									entry.created_at,
								).toLocaleDateString()}</td
							>
							<td>
								<button
									class="btn btn-danger btn-sm"
									onclick={() => confirmRemove(entry)}
								>
									Remove
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<!-- Add Entry Modal -->
{#if showAddModal}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={() => (showAddModal = false)}
		onkeydown={(e) => e.key === "Enter" && (showAddModal = false)}
	>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<div class="modal-header">
				<h2>Add Destination to Blacklist</h2>
				<button
					class="modal-close"
					onclick={() => (showAddModal = false)}>&times;</button
				>
			</div>
			<div class="modal-body">
				<div class="form-group">
					<label for="destination">Destination URL or Domain</label>
					<input
						id="destination"
						type="text"
						bind:value={newDestination}
						placeholder="https://example.com or example.com"
					/>
				</div>
				<div class="form-group">
					<label for="reason">Reason</label>
					<textarea
						id="reason"
						bind:value={newReason}
						placeholder="Why should this destination be blocked?"
						rows="3"
					></textarea>
				</div>
			</div>
			<div class="modal-footer">
				<button
					class="btn btn-secondary"
					onclick={() => (showAddModal = false)}
				>
					Cancel
				</button>
				<button class="btn btn-primary" onclick={handleAddEntry}
					>Add</button
				>
			</div>
		</div>
	</div>
{/if}

<!-- Remove Entry Modal -->
{#if showRemoveModal && selectedEntry}
	<div
		class="modal-backdrop"
		role="button"
		tabindex="0"
		onclick={() => (showRemoveModal = false)}
		onkeydown={(e) => e.key === "Enter" && (showRemoveModal = false)}
	>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<div class="modal-header">
				<h2>Remove Destination</h2>
				<button
					class="modal-close"
					onclick={() => (showRemoveModal = false)}>&times;</button
				>
			</div>
			<div class="modal-body">
				<p>
					Are you sure you want to remove <code
						>{selectedEntry.destination}</code
					> from the blacklist?
				</p>
				<p class="warning">
					This will allow users to create links to this destination
					again.
				</p>
			</div>
			<div class="modal-footer">
				<button
					class="btn btn-secondary"
					onclick={() => (showRemoveModal = false)}
				>
					Cancel
				</button>
				<button class="btn btn-danger" onclick={handleRemoveEntry}
					>Remove</button
				>
			</div>
		</div>
	</div>
{/if}

<!-- Toast -->
{#if showToast}
	<div class="toast">{toastMessage}</div>
{/if}

<style>
	.blacklist-page {
		max-width: 1200px;
		margin: 0 auto;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
	}

	.page-header h1 {
		margin: 0;
		font-size: 1.75rem;
		font-weight: 600;
	}

	.loading,
	.empty-state {
		text-align: center;
		padding: 3rem;
		color: #64748b;
	}

	.table-container {
		background: white;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		overflow: hidden;
	}

	.data-table {
		width: 100%;
		border-collapse: collapse;
	}

	.data-table th {
		background: #f1f5f9;
		padding: 1rem;
		text-align: left;
		font-weight: 600;
		font-size: 0.875rem;
		color: #475569;
		border-bottom: 1px solid #e2e8f0;
	}

	.data-table td {
		padding: 1rem;
		border-bottom: 1px solid #e2e8f0;
	}

	.destination-cell code {
		background: #f1f5f9;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.875rem;
		word-break: break-all;
	}

	.badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		font-size: 0.75rem;
		font-weight: 600;
	}

	.badge-exact {
		background: #dbeafe;
		color: #1e40af;
	}

	.badge-domain {
		background: #fce7f3;
		color: #9d174d;
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

	.btn-primary:hover {
		background: #2563eb;
	}

	.btn-secondary {
		background: #64748b;
		color: white;
	}

	.btn-secondary:hover {
		background: #475569;
	}

	.btn-danger {
		background: #ef4444;
		color: white;
	}

	.btn-danger:hover {
		background: #dc2626;
	}

	.btn-sm {
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
	}

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

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.modal-close {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: #64748b;
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
		font-size: 0.875rem;
		color: #475569;
	}

	.form-group input,
	.form-group textarea {
		width: 100%;
		padding: 0.625rem;
		border: 1px solid #e2e8f0;
		border-radius: 6px;
		font-size: 0.875rem;
		font-family: inherit;
	}

	.form-group textarea {
		resize: vertical;
	}

	.warning {
		color: #dc2626;
		font-size: 0.875rem;
		margin-top: 0.5rem;
	}

	.toast {
		position: fixed;
		bottom: 2rem;
		right: 2rem;
		background: #1e293b;
		color: white;
		padding: 1rem 1.5rem;
		border-radius: 6px;
		box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
		z-index: 1001;
		animation: slideIn 0.3s ease;
	}

	@keyframes slideIn {
		from {
			transform: translateY(100%);
			opacity: 0;
		}
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}
</style>
