<script lang="ts">
  import { adminApi } from "$lib/api/admin";
  import type { CustomDomain } from "$lib/api/domains";
  import { onMount } from "svelte";

  let domains = $state<CustomDomain[]>([]);
  let loading = $state(true);
  let error = $state("");
  let polling = $state(false);
  let pollResult = $state("");

  onMount(async () => {
    await loadDomains();
  });

  async function loadDomains() {
    try {
      loading = true;
      error = "";
      domains = await adminApi.listDomains();
    } catch (err) {
      error = "Failed to load custom domains";
      console.error("Domains load error:", err);
    } finally {
      loading = false;
    }
  }

  async function handlePollDomains() {
    try {
      polling = true;
      pollResult = "";
      const result = await adminApi.pollDomains();
      pollResult = result.message;
      // Refresh domain list after polling
      await loadDomains();
    } catch (err) {
      pollResult = "Failed to poll domains";
      console.error("Poll domains error:", err);
    } finally {
      polling = false;
    }
  }

  function getStatusBadge(status: string): string {
    switch (status) {
      case "active":
        return "status-badge status-active";
      case "pending":
        return "status-badge status-pending";
      case "failed":
        return "status-badge status-failed";
      default:
        return "status-badge";
    }
  }
</script>

<div class="domains-page">
  <div class="page-header">
    <h1>Custom Domains</h1>
    <p class="subtitle">
      Manage custom domains and poll their verification status
    </p>
  </div>

  {#if pollResult}
    <div class="alert alert-info">
      {pollResult}
    </div>
  {/if}

  <div class="action-bar">
    <button
      class="btn btn-primary"
      onclick={handlePollDomains}
      disabled={polling}
    >
      {#if polling}
        <span class="spinner"></span>
        Polling...
      {:else}
        🔄 Poll Pending Domains
      {/if}
    </button>
  </div>

  {#if loading}
    <div class="loading">Loading domains...</div>
  {:else if error}
    <div class="alert alert-error">{error}</div>
  {:else if domains.length === 0}
    <div class="empty-state">
      <p>No custom domains have been added yet.</p>
    </div>
  {:else}
    <div class="domains-table-container">
      <table class="domains-table">
        <thead>
          <tr>
            <th>Hostname</th>
            <th>Organization</th>
            <th>Status</th>
            <th>Created</th>
          </tr>
        </thead>
        <tbody>
          {#each domains as domain (domain.id)}
            <tr>
              <td class="hostname">{domain.hostname}</td>
              <td>{domain.org_id}</td>
              <td>
                <span class={getStatusBadge(domain.status)}>
                  {domain.status}
                </span>
              </td>
              <td>{new Date(domain.created_at * 1000).toLocaleDateString()}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .domains-page {
    padding: 2rem;
  }

  .page-header {
    margin-bottom: 2rem;
  }

  .page-header h1 {
    font-size: 1.75rem;
    font-weight: 600;
    color: #1f2937;
    margin: 0 0 0.5rem 0;
  }

  .subtitle {
    color: #6b7280;
    margin: 0;
  }

  .action-bar {
    margin-bottom: 1.5rem;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1rem;
    border-radius: 0.375rem;
    font-size: 0.875rem;
    font-weight: 500;
    border: none;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary {
    background: #4f46e5;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: #4338ca;
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spinner {
    display: inline-block;
    width: 1rem;
    height: 1rem;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .alert {
    padding: 0.75rem 1rem;
    border-radius: 0.375rem;
    margin-bottom: 1.5rem;
    font-size: 0.875rem;
  }

  .alert-info {
    background: #eff6ff;
    color: #1e40af;
    border: 1px solid #bfdbfe;
  }

  .alert-error {
    background: #fef2f2;
    color: #991b1b;
    border: 1px solid #fecaca;
  }

  .loading,
  .empty-state {
    text-align: center;
    padding: 3rem;
    color: #6b7280;
  }

  .domains-table-container {
    overflow-x: auto;
    border: 1px solid #e5e7eb;
    border-radius: 0.5rem;
  }

  .domains-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  .domains-table thead {
    background: #f9fafb;
  }

  .domains-table th {
    padding: 0.75rem 1rem;
    text-align: left;
    font-weight: 600;
    color: #374151;
    border-bottom: 1px solid #e5e7eb;
  }

  .domains-table td {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #f3f4f6;
    color: #4b5563;
  }

  .domains-table tbody tr:hover {
    background: #f9fafb;
  }

  .hostname {
    font-family: monospace;
    font-weight: 500;
    color: #1f2937;
  }

  .status-badge {
    display: inline-block;
    padding: 0.25rem 0.625rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
    text-transform: capitalize;
  }

  .status-active {
    background: #d1fae5;
    color: #065f46;
  }

  .status-pending {
    background: #fef3c7;
    color: #92400e;
  }

  .status-failed {
    background: #fee2e2;
    color: #991b1b;
  }
</style>
