<script lang="ts">
  import Logo from "./Logo.svelte";
  import UserMenu from "./UserMenu.svelte";
  import LoadingButton from "./LoadingButton.svelte";
  import Avatar from "./Avatar.svelte";
  import { authApi } from "$lib/api/auth";
  import { orgsApi } from "$lib/api/orgs";
  import { billingApi } from "$lib/api/billing";
  import { PUBLIC_VITE_DOCS_URL } from "$env/static/public";
  import type { User, OrgWithRole } from "$lib/types/api";

  const DOCS_URL =
    PUBLIC_VITE_DOCS_URL || "https://github.com/piffio/rushomon/";

  interface Props {
    user?: User | null;
    currentPage?: "landing" | "dashboard" | "analytics" | "admin" | "settings";
  }

  let { user, currentPage = "landing" }: Props = $props();
  let mobileMenuOpen = $state(false);
  let orgSwitcherOpen = $state(false);
  let showBilling = $state(false);
  let orgs = $state<OrgWithRole[]>([]);
  let currentOrgId = $state<string>("");
  let switchingOrg = $state(false);
  let orgsLoading = $state(false);
  let showCreateOrg = $state(false);
  let newOrgName = $state("");
  let creatingOrg = $state(false);
  let createOrgError = $state("");
  let navigatingToDashboard = $state(false);

  // Logo always links to landing page
  const logoHref = "/";

  async function handleNavigateToDashboard() {
    navigatingToDashboard = true;
    window.location.href = "/dashboard";
  }

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
      (currentPage === "dashboard" ||
        currentPage === "analytics" ||
        currentPage === "settings")
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

      billingApi
        .getStatus()
        .then((status) => {
          showBilling = status.is_billing_owner && status.tier !== "free";
        })
        .catch(() => {});
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

  const currentOrg = $derived(orgs.find((o) => o.id === currentOrgId));
  const canCreateOrg = $derived(() => {
    if (!currentOrg || !orgs) return false;
    // Only business organization owners can create new organizations
    const tier = currentOrg.tier;
    const role = currentOrg.role;
    if ((tier === "business" || tier === "unlimited") && role === "owner") {
      const ownedOrgs = orgs.filter((o) => o.role === "owner");
      return ownedOrgs.length < 3;
    }
    return false;
  });
  const ownedOrgCount = $derived(() => {
    if (!orgs) return 0;
    return orgs.filter((o) => o.role === "owner").length;
  });
  const isBusinessTier = $derived(() => {
    if (!currentOrg) return false;
    return currentOrg.tier === "business" || currentOrg.tier === "unlimited";
  });
  const hasReachedOrgLimit = $derived(() => {
    return (
      currentOrg &&
      isBusinessTier() &&
      currentOrg.role === "owner" &&
      ownedOrgCount() >= 3
    );
  });

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
            href={DOCS_URL}
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
            <LoadingButton
              onclick={handleNavigateToDashboard}
              loading={navigatingToDashboard}
              variant="primary"
              size="sm"
              class="hidden md:block bg-gradient-to-r from-orange-500 to-orange-600 hover:from-orange-600 hover:to-orange-700 shadow-sm hover:shadow-md"
            >
              Go to Dashboard →
            </LoadingButton>
          {/if}

          <!-- Org Switcher (dashboard & settings pages, always shown when authenticated) -->
          {#if currentPage === "dashboard" || currentPage === "analytics" || currentPage === "settings"}
            <div class="relative hidden md:block">
              <button
                onclick={() => (orgSwitcherOpen = !orgSwitcherOpen)}
                class="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-lg transition-colors max-w-[200px]"
                disabled={switchingOrg}
                aria-label="Organization switcher"
              >
                {#if orgsLoading}
                  <span
                    class="w-3 h-3 border border-gray-400 border-t-transparent rounded-full animate-spin flex-shrink-0"
                  ></span>
                  <span class="truncate text-gray-400">Loading…</span>
                {:else}
                  <span
                    class="w-5 h-5 rounded bg-gradient-to-br from-orange-400 to-orange-600 flex items-center justify-center text-white text-xs font-bold flex-shrink-0"
                  >
                    {(currentOrg?.name ?? "O").charAt(0).toUpperCase()}
                  </span>
                  <span class="truncate">{currentOrg?.name ?? "My Org"}</span>
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
                    e.key === "Escape" && (orgSwitcherOpen = false)}
                ></div>
                <div
                  class="absolute right-0 mt-1 w-72 bg-white rounded-xl border border-gray-200 shadow-lg z-50 overflow-hidden"
                >
                  <div class="px-3 py-2 border-b border-gray-100">
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
                          onclick={() => handleSwitchOrg(org.id)}
                          class="w-full flex items-center gap-3 px-3 py-2.5 text-sm text-left hover:bg-gray-50 transition-colors {org.id ===
                          currentOrgId
                            ? 'bg-orange-50'
                            : ''}"
                        >
                          <span
                            class="w-6 h-6 rounded-md bg-gradient-to-br from-orange-400 to-orange-600 flex items-center justify-center text-white text-xs font-bold flex-shrink-0"
                          >
                            {org.name.charAt(0).toUpperCase()}
                          </span>
                          <span class="flex-1 min-w-0">
                            <span
                              class="block truncate font-medium text-gray-900"
                              >{org.name}</span
                            >
                            <span
                              class="block text-xs text-gray-500 capitalize"
                            >
                              {org.role} · {org.tier}
                              {#if org.role === "owner"}
                                <span
                                  class="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800 ml-1"
                                >
                                  Personal
                                </span>
                              {/if}
                            </span>
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
                    {#if canCreateOrg()}
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
                        <span class="ml-auto text-xs text-gray-500"
                          >{ownedOrgCount()}/3</span
                        >
                      </button>
                    {:else if hasReachedOrgLimit()}
                      <div
                        class="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-400 cursor-default select-none"
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
                            d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                          />
                        </svg>
                        <span>{ownedOrgCount()}/3 organizations created</span>
                      </div>
                    {:else if !isBusinessTier()}
                      <div
                        class="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-400 cursor-default select-none"
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
                          >Business</span
                        >
                      </div>
                    {:else if currentOrg && currentOrg.role !== "owner"}
                      <div
                        class="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-400 cursor-default select-none"
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
                          class="ml-auto text-xs bg-gray-100 text-gray-600 px-1.5 py-0.5 rounded font-medium"
                          >Owner Only</span
                        >
                      </div>
                    {:else}
                      <button
                        onclick={() => {
                          orgSwitcherOpen = false;
                          window.location.href = "/pricing";
                        }}
                        class="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-400 hover:bg-gray-50 transition-colors"
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
                          >Business</span
                        >
                      </button>
                    {/if}
                  </div>
                </div>
              {/if}
            </div>
          {/if}

          <div class="user-menu-wrapper">
            <UserMenu {user} onLogout={handleLogout} {showBilling} />
          </div>
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
      <nav class="mobile-menu">
        {#if user}
          <!-- Authenticated Mobile Nav -->
          <!-- User Info Header -->
          <div class="mobile-user-info">
            <Avatar {user} size="lg" />
            <div class="mobile-user-details">
              <div class="mobile-user-name">
                {user.name || "User"}
              </div>
              <div class="mobile-user-email">{user.email}</div>
            </div>
          </div>

          <!-- User Menu Section -->
          <a
            href="/settings"
            class="mobile-nav-item"
            onclick={() => (mobileMenuOpen = false)}
          >
            <span class="mobile-nav-icon">⚙️</span>
            <span>Account Settings</span>
          </a>
          <a
            href={DOCS_URL}
            target="_blank"
            rel="noopener noreferrer"
            class="mobile-nav-item"
          >
            <span class="mobile-nav-icon">📖</span>
            <span>Documentation</span>
          </a>
          {#if user.role === "admin"}
            {#if currentPage === "dashboard" || currentPage === "analytics" || currentPage === "settings"}
              <a
                href="/admin/dashboard"
                class="mobile-nav-item"
                onclick={() => (mobileMenuOpen = false)}
              >
                <span class="mobile-nav-icon">👥</span>
                <span>Admin Dashboard</span>
              </a>
            {:else}
              <a
                href="/dashboard"
                class="mobile-nav-item"
                onclick={() => (mobileMenuOpen = false)}
              >
                <span class="mobile-nav-icon">📊</span>
                <span>Dashboard</span>
              </a>
            {/if}
          {/if}

          {#if currentPage === "landing"}
            <!-- Show Pricing on landing page -->
            <div class="mobile-nav-divider"></div>
            <a
              href="/pricing"
              class="mobile-nav-item"
              onclick={() => (mobileMenuOpen = false)}
            >
              <span class="mobile-nav-icon">💰</span>
              <span>Pricing</span>
            </a>
          {/if}

          <!-- Logout -->
          <div class="mobile-nav-divider"></div>
          <button
            class="mobile-nav-item mobile-logout"
            onclick={() => {
              handleLogout();
              mobileMenuOpen = false;
            }}
          >
            <span class="mobile-nav-icon">🚪</span>
            <span>Log out</span>
          </button>
        {:else}
          <!-- Unauthenticated Mobile Nav -->
          <a
            href="/#features"
            class="mobile-nav-item"
            onclick={() => (mobileMenuOpen = false)}
          >
            <span class="mobile-nav-icon">✨</span>
            <span>Features</span>
          </a>
          <a
            href="/pricing"
            class="mobile-nav-item"
            onclick={() => (mobileMenuOpen = false)}
          >
            <span class="mobile-nav-icon">💰</span>
            <span>Pricing</span>
          </a>
          <a
            href={DOCS_URL}
            target="_blank"
            rel="noopener noreferrer"
            class="mobile-nav-item"
          >
            <span class="mobile-nav-icon">📖</span>
            <span>Docs</span>
          </a>
          <div class="mobile-nav-divider"></div>
          <a
            href="/login"
            class="mobile-nav-item mobile-login"
            onclick={() => (mobileMenuOpen = false)}
          >
            <span class="mobile-nav-icon">🚪</span>
            <span>Sign In</span>
          </a>
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
    <div class="relative bg-white rounded-2xl shadow-xl w-full max-w-md p-6">
      <h2 class="text-lg font-semibold text-gray-900 mb-1">
        Create Organization
      </h2>
      <p class="text-sm text-gray-500 mb-5">
        Create a new workspace for your team. You'll be set as the owner.
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
        onkeydown={(e: KeyboardEvent) => e.key === "Enter" && handleCreateOrg()}
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

<style>
  /* Hide UserMenu on mobile */
  .user-menu-wrapper {
    display: block;
  }

  @media (max-width: 767px) {
    .user-menu-wrapper {
      display: none;
    }
  }

  /* Mobile Menu */
  .mobile-menu {
    display: block;
    margin-top: 1rem;
    padding-bottom: 1rem;
    border-top: 1px solid #e2e8f0;
    padding-top: 1rem;
  }

  @media (min-width: 768px) {
    .mobile-menu {
      display: none;
    }
  }

  /* Mobile User Info Header */
  .mobile-user-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 1rem;
    border-bottom: 1px solid #e2e8f0;
    margin-bottom: 0.5rem;
  }

  .mobile-user-details {
    flex: 1;
    min-width: 0;
  }

  .mobile-user-name {
    font-weight: 600;
    color: #1e293b;
    font-size: 0.95rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mobile-user-email {
    color: #64748b;
    font-size: 0.875rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .mobile-nav-divider {
    height: 1px;
    background: #e2e8f0;
    margin: 0.75rem 0;
  }

  .mobile-nav-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    color: #475569;
    text-decoration: none;
    border-radius: 0.375rem;
    transition: all 0.2s;
    font-size: 0.95rem;
  }

  .mobile-nav-item:hover {
    background: #f1f5f9;
    color: #1e293b;
  }

  .mobile-nav-icon {
    font-size: 1.25rem;
  }

  .mobile-logout {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    cursor: pointer;
    color: #dc2626;
  }

  .mobile-logout:hover {
    background: #fee2e2;
    color: #991b1b;
  }

  .mobile-login {
    color: #ea580c;
    font-weight: 600;
  }

  .mobile-login:hover {
    background: #ffedd5;
    color: #c2410c;
  }
</style>
