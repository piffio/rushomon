<script lang="ts">
  import { onMount } from "svelte";
  import { adminApi } from "$lib/api/admin";
  import type { AdminApiKey } from "$lib/api/admin";
  import Pagination from "$lib/components/Pagination.svelte";

  let keys = $state<AdminApiKey[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let error = $state("");
  let currentPage = $state(1);
  let searchQuery = $state("");
  let searchInput = $state("");
  let statusFilter = $state<"all" | "active" | "revoked" | "deleted">("all");
  let confirmingKeyId = $state<string | null>(null);
  let confirmingKeyName = $state<string>("");
  let confirmingAction = $state<
    "revoke" | "reactivate" | "delete" | "restore" | null
  >(null);
  let toast = $state<{
    message: string;
    type: "success" | "error";
    visible: boolean;
  }>({ message: "", type: "success", visible: false });

  const totalPages = $derived(Math.ceil(total / 20));

  onMount(() => {
    loadKeys();
  });

  async function loadKeys() {
    try {
      loading = true;
      error = "";
      const response = await adminApi.listApiKeys(
        currentPage,
        20,
        searchQuery || undefined,
        statusFilter
      );
      keys = response.keys;
      total = response.total;
    } catch (err) {
      error = "Failed to load API keys";
      console.error(err);
    } finally {
      loading = false;
    }
  }

  function handleSearch() {
    searchQuery = searchInput;
    currentPage = 1;
    loadKeys();
  }

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") handleSearch();
  }

  function handleStatusFilterChange() {
    currentPage = 1;
    loadKeys();
  }

  async function handlePageChange(page: number) {
    if (page < 1) return;
    currentPage = page;
    await loadKeys();
  }

  function confirmRevoke(key: AdminApiKey) {
    confirmingKeyId = key.id;
    confirmingKeyName = key.name;
    confirmingAction = "revoke";
  }

  function confirmReactivate(key: AdminApiKey) {
    confirmingKeyId = key.id;
    confirmingKeyName = key.name;
    confirmingAction = "reactivate";
  }

  function confirmDelete(key: AdminApiKey) {
    confirmingKeyId = key.id;
    confirmingKeyName = key.name;
    confirmingAction = "delete";
  }

  function confirmRestore(key: AdminApiKey) {
    confirmingKeyId = key.id;
    confirmingKeyName = key.name;
    confirmingAction = "restore";
  }

  function cancelAction() {
    confirmingKeyId = null;
    confirmingKeyName = "";
    confirmingAction = null;
  }

  async function doAction() {
    if (!confirmingKeyId || !confirmingAction) return;
    try {
      if (confirmingAction === "revoke") {
        await adminApi.revokeApiKey(confirmingKeyId);
        showToast("API key revoked successfully", "success");
      } else if (confirmingAction === "reactivate") {
        await adminApi.reactivateApiKey(confirmingKeyId);
        showToast("API key reactivated successfully", "success");
      } else if (confirmingAction === "delete") {
        await adminApi.deleteApiKey(confirmingKeyId);
        showToast("API key deleted successfully", "success");
      } else if (confirmingAction === "restore") {
        await adminApi.restoreApiKey(confirmingKeyId);
        showToast("API key restored successfully", "success");
      }
      await loadKeys();
    } catch (err) {
      showToast(`Failed to ${confirmingAction} API key`, "error");
      console.error(err);
    } finally {
      confirmingKeyId = null;
      confirmingKeyName = "";
      confirmingAction = null;
    }
  }

  function showToast(message: string, type: "success" | "error") {
    toast = { message, type, visible: true };
    setTimeout(() => {
      toast = { ...toast, visible: false };
    }, 3500);
  }

  function formatDate(ts: number | null): string {
    if (!ts) return "—";
    return new Date(ts * 1000).toLocaleString();
  }

  function getKeyStatus(
    key: AdminApiKey
  ): "active" | "revoked" | "deleted" | "expired" {
    if (key.status === "deleted") return "deleted";
    if (key.status === "revoked") return "revoked";
    if (key.expires_at && key.expires_at < Date.now() / 1000) return "expired";
    return "active";
  }

  function getTierBadgeClass(tier: string | null): string {
    switch (tier) {
      case "pro":
        return "tier-badge tier-pro";
      case "business":
        return "tier-badge tier-business";
      case "unlimited":
        return "tier-badge tier-unlimited";
      default:
        return "tier-badge tier-free";
    }
  }
</script>

<div class="page">
  <div class="page-header">
    <h1>API Keys</h1>
    <p class="subtitle">
      View and manage all API keys across the instance. Revoke keys in case of
      abuse.
    </p>
  </div>

  <div class="controls">
    <div class="search-row">
      <input
        type="text"
        placeholder="Search by user email, key name, or org..."
        bind:value={searchInput}
        onkeydown={handleSearchKeydown}
        class="search-input"
      />
      <button class="btn btn-primary" onclick={handleSearch}>Search</button>
    </div>
    <div class="filter-row">
      <label class="filter-label" for="status-filter">Status:</label>
      <select
        id="status-filter"
        bind:value={statusFilter}
        onchange={handleStatusFilterChange}
        class="status-filter"
      >
        <option value="all">All Keys</option>
        <option value="active">Active Only</option>
        <option value="revoked">Revoked Only</option>
        <option value="deleted">Deleted Only</option>
      </select>
    </div>
  </div>

  {#if error}
    <div class="error-banner">{error}</div>
  {/if}

  {#if loading}
    <div class="loading">Loading API keys…</div>
  {:else if keys.length === 0}
    <div class="empty-state">
      {#if searchQuery}
        No API keys found matching "<strong>{searchQuery}</strong>".
      {:else}
        No {statusFilter === "all" ? "" : statusFilter} API keys found.
      {/if}
    </div>
  {:else}
    <div class="table-wrapper">
      <table class="data-table">
        <thead>
          <tr>
            <th>Key name / hint</th>
            <th>User</th>
            <th>Organization</th>
            <th>Tier</th>
            <th>Created</th>
            <th>Last used</th>
            <th>Expires</th>
            <th>Status</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each keys as key (key.id)}
            {@const status = getKeyStatus(key)}
            <tr class:row-revoked={status === "revoked"}>
              <td>
                <div class="key-name">{key.name}</div>
                <div class="key-hint">{key.hint}</div>
              </td>
              <td>
                <div class="user-email">
                  {key.user_email ?? "—"}
                </div>
                {#if key.user_name}
                  <div class="user-name">{key.user_name}</div>
                {/if}
              </td>
              <td>{key.org_name ?? key.org_id}</td>
              <td>
                <span class={getTierBadgeClass(key.tier)}>
                  {key.tier ?? "free"}
                </span>
              </td>
              <td class="date-cell">{formatDate(key.created_at)}</td>
              <td class="date-cell">{formatDate(key.last_used_at)}</td>
              <td class="date-cell">{formatDate(key.expires_at)}</td>
              <td>
                {#if status === "deleted"}
                  <span class="status-badge status-deleted">Deleted</span>
                {:else if status === "revoked"}
                  <span class="status-badge status-revoked">Revoked</span>
                {:else if status === "expired"}
                  <span class="status-badge status-expired">Expired</span>
                {:else}
                  <span class="status-badge status-active">Active</span>
                {/if}
              </td>
              <td>
                {#if status === "active"}
                  <button
                    class="btn btn-danger btn-sm"
                    onclick={() => confirmRevoke(key)}
                  >
                    Revoke
                  </button>
                  <button
                    class="btn btn-secondary btn-sm"
                    onclick={() => confirmDelete(key)}
                  >
                    Delete
                  </button>
                {:else if status === "revoked"}
                  <button
                    class="btn btn-success btn-sm"
                    onclick={() => confirmReactivate(key)}
                  >
                    Reactivate
                  </button>
                  <button
                    class="btn btn-secondary btn-sm"
                    onclick={() => confirmDelete(key)}
                  >
                    Delete
                  </button>
                  <span class="muted status-info">
                    Updated {formatDate(key.updated_at)}
                  </span>
                {:else if status === "deleted"}
                  <button
                    class="btn btn-primary btn-sm"
                    onclick={() => confirmRestore(key)}
                  >
                    Restore
                  </button>
                  <span class="muted status-info">
                    Deleted {formatDate(key.updated_at)}
                  </span>
                {:else}
                  <span class="muted">—</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if totalPages > 1}
      <Pagination {currentPage} {totalPages} onPageChange={handlePageChange} />
    {/if}
  {/if}

  {#if total > 0}
    <div class="results-count">
      Showing {keys.length} of {total} key{total !== 1 ? "s" : ""}
    </div>
  {/if}
</div>

<!-- Confirm action modal -->
{#if confirmingKeyId && confirmingAction}
  <div
    class="modal-overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="action-modal-title"
    tabindex="-1"
    onkeydown={(e) => e.key === "Escape" && cancelAction()}
  >
    <div class="modal">
      <h2 id="action-modal-title">
        {#if confirmingAction === "revoke"}
          Revoke API Key
        {:else if confirmingAction === "reactivate"}
          Reactivate API Key
        {:else if confirmingAction === "delete"}
          Delete API Key
        {:else if confirmingAction === "restore"}
          Restore API Key
        {/if}
      </h2>
      <p>
        {#if confirmingAction === "revoke"}
          Are you sure you want to revoke <strong>{confirmingKeyName}</strong>?
          This will immediately invalidate the key and prevent it from being
          used.
        {:else if confirmingAction === "reactivate"}
          Are you sure you want to reactivate <strong
            >{confirmingKeyName}</strong
          >? This will allow the key to be used again.
        {:else if confirmingAction === "delete"}
          Are you sure you want to delete <strong>{confirmingKeyName}</strong>?
          This will soft-delete the key and hide it from normal views.
        {:else if confirmingAction === "restore"}
          Are you sure you want to restore <strong>{confirmingKeyName}</strong>?
          This will restore the deleted key to active status.
        {/if}
      </p>
      <div class="modal-actions">
        <button class="btn btn-secondary" onclick={cancelAction}>Cancel</button>
        <button
          class="btn {confirmingAction === 'revoke'
            ? 'btn-danger'
            : confirmingAction === 'delete'
              ? 'btn-danger'
              : 'btn-success'}"
          onclick={doAction}
        >
          {#if confirmingAction === "revoke"}
            Revoke Key
          {:else if confirmingAction === "reactivate"}
            Reactivate Key
          {:else if confirmingAction === "delete"}
            Delete Key
          {:else if confirmingAction === "restore"}
            Restore Key
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Toast notification -->
{#if toast.visible}
  <div class="toast toast-{toast.type}" role="alert">
    {toast.message}
  </div>
{/if}

<style>
  .page {
    max-width: 1200px;
  }

  .page-header {
    margin-bottom: 1.5rem;
  }

  .page-header h1 {
    font-size: 1.75rem;
    font-weight: 700;
    color: #1e293b;
    margin: 0 0 0.25rem;
  }

  .subtitle {
    color: #64748b;
    margin: 0;
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin-bottom: 1.5rem;
  }

  .search-row {
    display: flex;
    gap: 0.5rem;
  }

  .filter-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .filter-label {
    font-size: 0.9rem;
    color: #475569;
    font-weight: 500;
  }

  .status-filter {
    padding: 0.4rem 0.75rem;
    border: 1px solid #cbd5e1;
    border-radius: 0.375rem;
    font-size: 0.9rem;
    background: white;
    color: #1e293b;
    min-width: 140px;
  }

  .status-filter:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
  }

  .search-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid #cbd5e1;
    border-radius: 0.375rem;
    font-size: 0.9rem;
  }

  .search-input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
    color: #475569;
    cursor: pointer;
    user-select: none;
  }

  .error-banner {
    background: #fee2e2;
    color: #991b1b;
    padding: 0.75rem 1rem;
    border-radius: 0.375rem;
    margin-bottom: 1rem;
  }

  .loading,
  .empty-state {
    text-align: center;
    padding: 3rem 1rem;
    color: #64748b;
  }

  .table-wrapper {
    overflow-x: auto;
    border: 1px solid #e2e8f0;
    border-radius: 0.5rem;
    margin-bottom: 1rem;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  .data-table th {
    padding: 0.75rem 1rem;
    text-align: left;
    background: #f8fafc;
    color: #475569;
    font-weight: 600;
    border-bottom: 1px solid #e2e8f0;
    white-space: nowrap;
  }

  .data-table td {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #f1f5f9;
    vertical-align: middle;
  }

  .data-table tr:last-child td {
    border-bottom: none;
  }

  .row-revoked td {
    opacity: 0.6;
  }

  .key-name {
    font-weight: 500;
    color: #1e293b;
  }

  .key-hint {
    font-family: monospace;
    font-size: 0.8rem;
    color: #94a3b8;
  }

  .user-email {
    color: #1e293b;
  }

  .user-name {
    font-size: 0.8rem;
    color: #94a3b8;
  }

  .date-cell {
    white-space: nowrap;
    color: #475569;
    font-size: 0.8rem;
  }

  .muted {
    color: #94a3b8;
    font-size: 0.8rem;
  }

  /* Tier badges */
  .tier-badge {
    display: inline-block;
    padding: 0.2rem 0.5rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .tier-free {
    background: #f1f5f9;
    color: #64748b;
  }
  .tier-pro {
    background: #dbeafe;
    color: #1d4ed8;
  }
  .tier-business {
    background: #ede9fe;
    color: #6d28d9;
  }
  .tier-unlimited {
    background: #d1fae5;
    color: #065f46;
  }

  /* Status badges */
  .status-badge {
    display: inline-block;
    padding: 0.2rem 0.6rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .status-active {
    background: #d1fae5;
    color: #065f46;
  }
  .status-expired {
    background: #fef9c3;
    color: #854d0e;
  }
  .status-revoked {
    background: #fee2e2;
    color: #991b1b;
  }
  .status-deleted {
    background: #f3f4f6;
    color: #6b7280;
    text-decoration: line-through;
  }

  /* Buttons */
  .btn {
    padding: 0.5rem 1rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: all 0.15s;
  }

  .btn-primary {
    background: #3b82f6;
    color: white;
  }

  .btn-primary:hover {
    background: #2563eb;
  }

  .btn-secondary {
    background: #f1f5f9;
    color: #475569;
  }

  .btn-secondary:hover {
    background: #e2e8f0;
  }

  .btn-danger {
    background: #ef4444;
    color: white;
  }

  .btn-danger:hover {
    background: #dc2626;
  }

  .btn-success {
    background: #10b981;
    color: white;
  }

  .btn-success:hover {
    background: #059669;
  }

  .btn-sm {
    padding: 0.35rem 0.75rem;
    font-size: 0.8rem;
  }

  .status-info {
    display: block;
    font-size: 0.75rem;
    margin-top: 0.25rem;
  }

  .results-count {
    font-size: 0.85rem;
    color: #94a3b8;
    text-align: right;
    margin-top: 0.5rem;
  }

  /* Modal */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .modal {
    background: white;
    border-radius: 0.75rem;
    padding: 2rem;
    max-width: 460px;
    width: 90%;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
  }

  .modal h2 {
    margin: 0 0 0.75rem;
    font-size: 1.25rem;
    color: #1e293b;
  }

  .modal p {
    color: #475569;
    margin: 0 0 1.5rem;
    line-height: 1.6;
  }

  .modal-actions {
    display: flex;
    gap: 0.75rem;
    justify-content: flex-end;
  }

  /* Toast */
  .toast {
    position: fixed;
    bottom: 1.5rem;
    right: 1.5rem;
    padding: 0.875rem 1.25rem;
    border-radius: 0.5rem;
    font-size: 0.9rem;
    font-weight: 500;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 300;
    animation: slide-in 0.2s ease;
  }

  .toast-success {
    background: #d1fae5;
    color: #065f46;
  }
  .toast-error {
    background: #fee2e2;
    color: #991b1b;
  }

  @keyframes slide-in {
    from {
      transform: translateY(1rem);
      opacity: 0;
    }
    to {
      transform: translateY(0);
      opacity: 1;
    }
  }

  @media (max-width: 768px) {
    .data-table th:nth-child(4),
    .data-table td:nth-child(4),
    .data-table th:nth-child(7),
    .data-table td:nth-child(7) {
      display: none;
    }
  }
</style>
