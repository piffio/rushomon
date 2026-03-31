<script lang="ts">
  import { onMount } from "svelte";
  import { adminApi } from "$lib/api/admin";
  import { authApi } from "$lib/api/auth";
  import Pagination from "$lib/components/Pagination.svelte";
  import type { User } from "$lib/types/api";

  let users = $state<User[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let error = $state("");
  let currentPage = $state(1);
  let confirmingUserId = $state<string | null>(null);
  let confirmingRole = $state<"admin" | "member" | null>(null);
  let currentUser = $state<User | null>(null);
  let tierLoading = $state(false);
  let confirmingTierChange = $state<{
    userId: string;
    orgId: string;
    currentTier: string;
  } | null>(null);
  let orgTiers = $state<Record<string, string>>({});
  let activeDropdown = $state<string | null>(null);
  let confirmingSuspend = $state<string | null>(null);
  let confirmingDelete = $state<string | null>(null);
  let dropdownPosition = $state<{ top: number; right: number } | null>(null);
  let selectedNewTier = $state<string>("");

  onMount(async () => {
    await loadUsers();
    await loadCurrentUser();
  });

  async function loadUsers() {
    try {
      loading = true;
      const response = await adminApi.listUsers(currentPage, 20);
      users = response.users;
      total = response.total;
      if (response.org_tiers) {
        orgTiers = { ...orgTiers, ...response.org_tiers };
      }
    } catch (err) {
      error = "Failed to load users";
      console.error(err);
    } finally {
      loading = false;
    }
  }

  async function loadCurrentUser() {
    try {
      currentUser = await authApi.me();
    } catch (err) {
      console.error("Failed to load current user:", err);
    }
  }

  async function handleRoleChange(userId: string, newRole: "admin" | "member") {
    confirmingUserId = userId;
    confirmingRole = newRole;
  }

  async function confirmRoleChange() {
    if (!confirmingUserId || !confirmingRole) return;

    try {
      loading = true;
      const updatedUser = await adminApi.updateUserRole(
        confirmingUserId,
        confirmingRole
      );
      users = users.map((u) => (u.id === updatedUser.id ? updatedUser : u));
    } catch (err) {
      error = "Failed to update user role";
      console.error(err);
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

  function toggleDropdown(userId: string, event: MouseEvent) {
    const button = event.currentTarget as HTMLElement;
    const rect = button.getBoundingClientRect();

    if (activeDropdown === userId) {
      activeDropdown = null;
      dropdownPosition = null;
    } else {
      activeDropdown = userId;
      dropdownPosition = {
        top: rect.bottom + 4,
        right: window.innerWidth - rect.right
      };
    }
  }

  function closeDropdown() {
    activeDropdown = null;
    dropdownPosition = null;
  }

  async function handleSuspend(userId: string) {
    confirmingSuspend = userId;
  }

  async function confirmSuspend(userId?: string, currentlySuspended?: boolean) {
    if (userId) {
      // Called from mobile card - toggle suspend/unsuspend
      confirmingSuspend = userId;
      try {
        loading = true;
        if (currentlySuspended) {
          await adminApi.unsuspendUser(String(userId));
        } else {
          await adminApi.suspendUser(String(userId), "Suspended by admin");
        }
        await loadUsers();
      } catch (err) {
        error = currentlySuspended
          ? "Failed to unsuspend user"
          : "Failed to suspend user";
        console.error(err);
      } finally {
        loading = false;
        confirmingSuspend = null;
      }
    } else if (confirmingSuspend) {
      // Called from desktop table modal
      try {
        loading = true;
        await adminApi.suspendUser(
          String(confirmingSuspend),
          "Suspended by admin"
        );
        await loadUsers();
      } catch (err) {
        error = "Failed to suspend user";
        console.error(err);
      } finally {
        loading = false;
        confirmingSuspend = null;
      }
    }
  }

  function cancelSuspend() {
    confirmingSuspend = null;
  }

  async function handleActivate(userId: string) {
    try {
      loading = true;
      await adminApi.unsuspendUser(String(userId));
      await loadUsers(); // Reload to show updated status
    } catch (err) {
      error = "Failed to activate user";
      console.error(err);
    } finally {
      loading = false;
    }
  }

  async function handleDelete(userId: string) {
    confirmingDelete = userId;
  }

  async function confirmDelete(userId?: string) {
    if (userId) {
      // Called from mobile card
      try {
        loading = true;
        await adminApi.deleteUser(String(userId));
        await loadUsers();
      } catch (err) {
        error = "Failed to delete user";
        console.error(err);
      } finally {
        loading = false;
      }
    } else if (confirmingDelete) {
      // Called from desktop table modal
      try {
        loading = true;
        await adminApi.deleteUser(String(confirmingDelete));
        await loadUsers();
      } catch (err) {
        error = "Failed to delete user";
        console.error(err);
      } finally {
        loading = false;
        confirmingDelete = null;
      }
    }
  }

  function cancelDelete() {
    confirmingDelete = null;
  }

  function getOrgTier(orgId: string): string {
    return orgTiers[orgId] || "free";
  }

  async function handlePageChange(page: number) {
    if (page < 1) return;
    currentPage = page;
    await loadUsers();
  }

  function formatDate(ts: number | string): string {
    const ms = typeof ts === "number" ? ts * 1000 : Number(ts) * 1000;
    return new Date(ms).toLocaleDateString();
  }

  const totalPages = $derived(Math.ceil(total / 20));
</script>

<div class="users-page">
  <div class="page-header">
    <h1>User Management</h1>
    <p class="subtitle">Manage user accounts, roles, and permissions</p>
  </div>

  {#if loading && users.length === 0}
    <div class="loading">Loading users...</div>
  {:else if error}
    <div class="error">{error}</div>
  {:else}
    <div class="users-container">
      <!-- Mobile Card View -->
      <div class="mobile-cards">
        {#each users as user (user.id)}
          <div class="user-card">
            <div class="card-header">
              <div class="user-info">
                {#if user.avatar_url}
                  <img
                    src={user.avatar_url}
                    alt={user.name || user.email}
                    class="avatar"
                    referrerpolicy="no-referrer"
                  />
                {:else}
                  <div class="avatar-placeholder">
                    {(user.name || user.email).charAt(0).toUpperCase()}
                  </div>
                {/if}
                <div class="user-details">
                  <h3>{user.name || "Unknown"}</h3>
                  <p class="email">{user.email}</p>
                </div>
              </div>
              <div class="badges">
                {#if user.role === "admin"}
                  <span class="badge admin">Admin</span>
                {:else}
                  <span class="badge member">Member</span>
                {/if}
                {#if user.suspended_at}
                  <span class="badge suspended">Suspended</span>
                {:else}
                  <span class="badge active">Active</span>
                {/if}
              </div>
            </div>
            <div class="card-body">
              <div class="card-row">
                <span class="label">Provider:</span>
                <span>{user.oauth_provider || "Unknown"}</span>
              </div>
              <div class="card-row">
                <span class="label">Tier:</span>
                <span class="tier">{user.billing_account_tier || "N/A"}</span>
              </div>
              <div class="card-row">
                <span class="label">Billing:</span>
                {#if user.billing_account_id}
                  <a
                    href="/admin/billing#{user.billing_account_id}"
                    class="billing-link"
                  >
                    {user.billing_account_id.substring(0, 12)}...
                  </a>
                {:else}
                  <span>N/A</span>
                {/if}
              </div>
              <div class="card-row">
                <span class="label">Joined:</span>
                <span>{formatDate(user.created_at)}</span>
              </div>
            </div>
            <div class="card-actions">
              <button
                onclick={() =>
                  handleRoleChange(
                    user.id,
                    user.role === "admin" ? "member" : "admin"
                  )}
                class="btn btn-secondary"
              >
                {user.role === "admin" ? "Demote" : "Promote"}
              </button>
              <button
                onclick={() => confirmSuspend(user.id, !!user.suspended_at)}
                class="btn {user.suspended_at ? 'btn-success' : 'btn-danger'}"
              >
                {user.suspended_at ? "Unsuspend" : "Suspend"}
              </button>
              <button
                onclick={() => confirmDelete(user.id)}
                class="btn btn-danger"
              >
                Delete
              </button>
            </div>
          </div>
        {/each}
      </div>

      <!-- Desktop Table View -->
      <div class="users-table">
        <table>
          <thead>
            <tr>
              <th>User</th>
              <th>Email</th>
              <th>Provider</th>
              <th>Role</th>
              <th>Status</th>
              <th>Billing Account</th>
              <th>Tier</th>
              <th>Joined</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each users as user (user.id)}
              <tr>
                <td>
                  <div class="user-info">
                    {#if user.avatar_url}
                      <img
                        src={user.avatar_url}
                        alt={user.name || user.email}
                        class="avatar"
                        referrerpolicy="no-referrer"
                      />
                    {:else}
                      <div class="avatar-placeholder">
                        {(user.name || user.email).charAt(0).toUpperCase()}
                      </div>
                    {/if}
                    <span class="user-name">{user.name || "Unknown"}</span>
                  </div>
                </td>
                <td class="email">{user.email}</td>
                <td class="provider">{user.oauth_provider || "Unknown"}</td>
                <td>
                  {#if user.role === "admin"}
                    <span class="badge admin">Admin</span>
                  {:else}
                    <span class="badge member">Member</span>
                  {/if}
                </td>
                <td>
                  {#if user.suspended_at}
                    <span class="badge suspended">Suspended</span>
                  {:else}
                    <span class="badge active">Active</span>
                  {/if}
                </td>
                <td>
                  {#if user.billing_account_id}
                    <a
                      href="/admin/billing#{user.billing_account_id}"
                      class="billing-link"
                      title="View billing account"
                    >
                      {user.billing_account_id.substring(0, 12)}...
                    </a>
                  {:else}
                    <span class="no-billing">N/A</span>
                  {/if}
                </td>
                <td>
                  {#if user.billing_account_tier}
                    <a
                      href="/admin/billing#{user.billing_account_id}"
                      class="tier-badge-link {user.billing_account_tier ===
                      'unlimited'
                        ? 'unlimited'
                        : user.billing_account_tier === 'business'
                          ? 'business'
                          : user.billing_account_tier === 'pro'
                            ? 'pro'
                            : 'free'}"
                      title="Managed at billing account level"
                    >
                      {user.billing_account_tier.charAt(0).toUpperCase() +
                        user.billing_account_tier.slice(1)} 🔗
                    </a>
                  {:else}
                    <span
                      class="tier-badge {getOrgTier(user.org_id) === 'unlimited'
                        ? 'unlimited'
                        : 'free'}"
                    >
                      {getOrgTier(user.org_id) === "unlimited"
                        ? "Unlimited"
                        : "Free"}
                    </span>
                  {/if}
                </td>
                <td class="date">{formatDate(user.created_at)}</td>
                <td>
                  {#if currentUser && user.id === currentUser.id}
                    <span class="no-actions">Cannot edit self</span>
                  {:else}
                    <div class="dropdown-container">
                      <button
                        class="dropdown-toggle"
                        onclick={(e) => toggleDropdown(user.id, e)}
                        aria-label="Actions"
                        aria-expanded={activeDropdown === user.id}
                      >
                        ⋮
                      </button>
                      {#if activeDropdown === user.id}
                        <div
                          class="dropdown-menu"
                          style="top: {dropdownPosition?.top ||
                            0}px; right: {dropdownPosition?.right || 0}px;"
                        >
                          {#if user.role === "member"}
                            <button
                              class="dropdown-item promote"
                              onclick={() => {
                                handleRoleChange(user.id, "admin");
                                closeDropdown();
                              }}
                            >
                              Promote to Admin
                            </button>
                          {:else}
                            <button
                              class="dropdown-item demote"
                              onclick={() => {
                                handleRoleChange(user.id, "member");
                                closeDropdown();
                              }}
                            >
                              Demote to Member
                            </button>
                          {/if}
                          {#if user.suspended_at}
                            <button
                              class="dropdown-item success"
                              onclick={() => {
                                handleActivate(user.id);
                                closeDropdown();
                              }}
                            >
                              Activate User
                            </button>
                          {:else}
                            <button
                              class="dropdown-item suspend"
                              onclick={() => {
                                handleSuspend(user.id);
                                closeDropdown();
                              }}
                            >
                              Suspend User
                            </button>
                          {/if}
                          <button
                            class="dropdown-item delete"
                            onclick={() => {
                              handleDelete(user.id);
                              closeDropdown();
                            }}
                          >
                            Delete User
                          </button>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>

        <!-- Pagination -->
        {#if totalPages > 1}
          <div class="mt-6">
            <Pagination
              {currentPage}
              {totalPages}
              onPageChange={handlePageChange}
              {loading}
            />
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Role Change Confirmation Modal -->
{#if confirmingUserId && confirmingRole}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={cancelRoleChange}
    onkeydown={(e) => e.key === "Enter" && cancelRoleChange()}
  >
    <div
      class="modal"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="0"
      onkeydown={(e) => e.key === "Escape" && cancelRoleChange()}
    >
      <div class="modal-header">
        <h3>
          {confirmingRole === "admin"
            ? "Promote to Admin?"
            : "Demote to Member?"}
        </h3>
        <button class="modal-close" onclick={cancelRoleChange}>&times;</button>
      </div>
      <div class="modal-body">
        <p>
          {#if confirmingRole === "admin"}
            Are you sure you want to <strong>promote this user to admin</strong
            >? They will have full access to all admin features.
          {:else}
            Are you sure you want to <strong>demote this admin to member</strong
            >? They will lose access to admin features.
          {/if}
        </p>
      </div>
      <div class="modal-footer">
        <button
          class="btn btn-secondary"
          onclick={cancelRoleChange}
          disabled={loading}
        >
          Cancel
        </button>
        <button
          class="btn btn-primary"
          onclick={confirmRoleChange}
          disabled={loading}
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

<!-- Suspend Confirmation Modal -->
{#if confirmingSuspend}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={cancelSuspend}
    onkeydown={(e) => e.key === "Enter" && cancelSuspend()}
  >
    <div
      class="modal"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="0"
      onkeydown={(e) => e.key === "Escape" && cancelSuspend()}
    >
      <div class="modal-header">
        <h3>Suspend User?</h3>
        <button class="modal-close" onclick={cancelSuspend}>&times;</button>
      </div>
      <div class="modal-body">
        <p>
          Are you sure you want to <strong>suspend this user</strong>? They will
          lose access to the platform and all their links will be disabled.
        </p>
      </div>
      <div class="modal-footer">
        <button
          class="btn btn-secondary"
          onclick={cancelSuspend}
          disabled={loading}
        >
          Cancel
        </button>
        <button
          class="btn btn-danger"
          onclick={() => confirmSuspend()}
          disabled={loading}
        >
          {#if loading}
            Suspending...
          {:else}
            Suspend User
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Delete Confirmation Modal -->
{#if confirmingDelete}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={cancelDelete}
    onkeydown={(e) => e.key === "Enter" && cancelDelete()}
  >
    <div
      class="modal"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="0"
      onkeydown={(e) => e.key === "Escape" && cancelDelete()}
    >
      <div class="modal-header">
        <h3>Delete User?</h3>
        <button class="modal-close" onclick={cancelDelete}>&times;</button>
      </div>
      <div class="modal-body">
        <p>
          Are you sure you want to <strong>permanently delete this user</strong
          >?
        </p>
        <p class="warning">
          ⚠️ This action cannot be undone. The following will be permanently
          deleted:
        </p>
        <ul class="deletion-list">
          <li>User account and profile information</li>
          <li>All links created by this user</li>
          <li>All analytics data for their links</li>
          <li>Organization membership</li>
        </ul>
      </div>
      <div class="modal-footer">
        <button
          class="btn btn-secondary"
          onclick={cancelDelete}
          disabled={loading}
        >
          Cancel
        </button>
        <button
          class="btn btn-danger"
          onclick={() => confirmDelete()}
          disabled={loading}
        >
          {#if loading}
            Deleting...
          {:else}
            Delete Permanently
          {/if}
        </button>
      </div>
    </div>
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
  .users-page {
    max-width: 1200px;
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

  .loading,
  .error {
    text-align: center;
    padding: 3rem;
    color: #64748b;
  }

  .error {
    color: #dc2626;
  }

  .users-container {
    background: white;
    border-radius: 8px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    overflow: visible;
    position: relative;
  }

  /* Mobile Cards */
  .mobile-cards {
    display: none;
    gap: 1rem;
  }

  .user-card {
    background: white;
    border-radius: 8px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
    overflow: hidden;
    margin-bottom: 1rem;
  }

  .card-header {
    padding: 1rem;
    border-bottom: 1px solid #e2e8f0;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .card-header .user-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .card-header .avatar,
  .avatar-placeholder {
    width: 48px;
    height: 48px;
  }

  .card-header .user-details h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: #1e293b;
  }

  .card-header .user-details .email {
    margin: 0.25rem 0 0 0;
    font-size: 0.875rem;
    color: #64748b;
  }

  .card-header .badges {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .card-body {
    padding: 1rem;
  }

  .card-row {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 0;
    border-bottom: 1px solid #f1f5f9;
  }

  .card-row:last-child {
    border-bottom: none;
  }

  .card-row .label {
    font-weight: 500;
    color: #64748b;
  }

  .card-row .tier {
    text-transform: capitalize;
  }

  .card-row .billing-link {
    color: #3b82f6;
    text-decoration: none;
  }

  .card-row .billing-link:hover {
    text-decoration: underline;
  }

  .card-actions {
    padding: 1rem;
    background: #f8fafc;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .card-actions .btn {
    width: 100%;
    padding: 0.75rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .card-actions .btn-secondary {
    background: #e2e8f0;
    color: #475569;
  }

  .card-actions .btn-secondary:hover {
    background: #cbd5e1;
  }

  .card-actions .btn-danger {
    background: #fee2e2;
    color: #991b1b;
  }

  .card-actions .btn-danger:hover {
    background: #fecaca;
  }

  .card-actions .btn-success {
    background: #d1fae5;
    color: #065f46;
  }

  .card-actions .btn-success:hover {
    background: #a7f3d0;
  }

  .users-table {
    overflow-x: auto;
  }

  .users-table table {
    width: 100%;
    border-collapse: collapse;
  }

  .users-table th {
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

  .users-table td {
    padding: 1rem;
    border-bottom: 1px solid #f1f5f9;
  }

  .users-table tr:hover {
    background: #f8fafc;
  }

  .user-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    object-fit: cover;
  }

  .avatar-placeholder {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: #e2e8f0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    color: #64748b;
  }

  .user-name {
    font-weight: 500;
    color: #1e293b;
  }

  .email {
    color: #64748b;
    font-size: 0.875rem;
  }

  .provider {
    color: #64748b;
    font-size: 0.875rem;
    text-transform: capitalize;
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

  .badge.admin {
    background: #fef3c7;
    color: #92400e;
  }

  .badge.member {
    background: #f3f4f6;
    color: #6b7280;
  }

  .badge.active {
    background: #d1fae5;
    color: #065f46;
  }

  .badge.suspended {
    background: #fee2e2;
    color: #991b1b;
  }

  .tier-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: all 0.2s;
  }

  .tier-badge.free {
    background: #dbeafe;
    color: #1e40af;
  }

  .tier-badge.free:hover {
    background: #bfdbfe;
  }

  .tier-badge.unlimited {
    background: #d1fae5;
    color: #065f46;
  }

  .tier-badge.unlimited:hover {
    background: #a7f3d0;
  }

  .tier-badge:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .tier-badge-link {
    display: inline-flex;
    align-items: center;
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
    text-decoration: none;
    transition: all 0.2s;
  }

  .tier-badge-link.free {
    background: #dbeafe;
    color: #1e40af;
  }

  .tier-badge-link.free:hover {
    background: #bfdbfe;
  }

  .tier-badge-link.pro {
    background: #fef3c7;
    color: #92400e;
  }

  .tier-badge-link.pro:hover {
    background: #fde68a;
  }

  .tier-badge-link.business {
    background: #d1fae5;
    color: #065f46;
  }

  .tier-badge-link.business:hover {
    background: #a7f3d0;
  }

  .tier-badge-link.unlimited {
    background: #e0e7ff;
    color: #3730a3;
  }

  .tier-badge-link.unlimited:hover {
    background: #c7d2fe;
  }

  .billing-link {
    color: #3b82f6;
    text-decoration: none;
    font-size: 0.875rem;
    font-family: monospace;
  }

  .billing-link:hover {
    text-decoration: underline;
  }

  .no-billing {
    color: #9ca3af;
    font-style: italic;
    font-size: 0.875rem;
  }

  .date {
    color: #64748b;
    font-size: 0.875rem;
  }

  .no-actions {
    color: #9ca3af;
    font-size: 0.875rem;
    font-style: italic;
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

  .dropdown-item.promote {
    color: #059669;
  }

  .dropdown-item.demote {
    color: #dc2626;
  }

  .dropdown-item.suspend {
    color: #d97706;
  }

  .dropdown-item.delete {
    color: #dc2626;
    border-top: 1px solid #e5e7eb;
  }

  .dropdown-item.success {
    color: #059669;
  }

  /* Delete modal specific styles */
  .warning {
    color: #dc2626;
    font-weight: 600;
    margin: 1rem 0;
  }

  .deletion-list {
    margin: 1rem 0;
    padding-left: 1.5rem;
    color: #6b7280;
  }

  .deletion-list li {
    margin-bottom: 0.5rem;
  }

  .dropdown-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 40;
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

  .btn-primary {
    background: #3b82f6;
    color: white;
  }

  .btn-primary:hover:not(:disabled) {
    background: #2563eb;
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

  /* Responsive */
  @media (max-width: 768px) {
    .mobile-cards {
      display: block;
    }

    .users-table {
      display: none;
    }

    .users-container {
      padding-top: 3rem;
    }
  }
</style>
