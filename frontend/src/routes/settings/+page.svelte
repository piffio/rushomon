<script lang="ts">
  import { backdropClose } from "$lib/actions/backdropClose";
  import type { BillingStatus } from "$lib/api/billing";
  import { billingApi } from "$lib/api/billing";
  import { apiClient } from "$lib/api/client";
  import { notificationsApi } from "$lib/api/notifications";
  import { orgsApi } from "$lib/api/orgs";
  import { apiKeysApi, type ApiKey } from "$lib/api/settings";
  import Header from "$lib/components/Header.svelte";
  import type { NotificationPreferences, OrgWithRole } from "$lib/types/api";
  import type { PageData } from "./$types";

  const { data }: { data: PageData } = $props();

  // Version info from API
  let versionInfo = $state({
    version: "Loading...",
    name: "Rushomon",
    build_timestamp: "unknown",
    git_commit: "unknown"
  });

  // User organizations
  let userOrgs = $state<OrgWithRole[]>([]);
  let orgsLoading = $state(true);

  // Billing status (needed to know if user is billing owner)
  let billingStatus = $state<BillingStatus | null>(null);

  // API Keys State
  let apiKeys = $state<ApiKey[]>([]);
  let keysLoading = $state(true);
  let isCreateModalOpen = $state(false);
  let keyToDelete = $state<string | null>(null);
  let isDeletingKey = $state(false);
  let newKeyName = $state("");
  let newKeyExpires = $state<number | null>(30); // Default 30 days
  let newKeySelectedOrgIds = $state<string[]>([]); // populated when modal opens
  let newlyGeneratedToken = $state<string | null>(null);
  let isCreatingKey = $state(false);
  let createError = $state<string | null>(null);
  let copySuccess = $state(false);

  // Edit-scope modal state
  let editScopeKey = $state<ApiKey | null>(null);
  let editScopeOrgIds = $state<string[]>([]);
  let isSavingScope = $state(false);
  let editScopeError = $state<string | null>(null);

  // Email notification feature flag (from public settings)
  let emailNotificationsEnabled = $state(false);

  // Notification preferences
  let notifPrefs = $state<NotificationPreferences>({
    email_monthly_stats: true
  });
  let notifPrefsLoading = $state(true);
  let notifSaving = $state(false);

  // Helper for version API URL
  const versionApiUrl = `${apiClient["baseUrl"]}/api/version`;

  $effect(() => {
    apiClient
      .get<{
        version: string;
        name: string;
        build_timestamp: string;
        git_commit: string;
      }>("/api/version")
      .then((d) => {
        versionInfo = d;
      })
      .catch(() => {
        versionInfo.version = "Error";
      });

    orgsApi
      .listMyOrgs()
      .then((res) => {
        userOrgs = res.orgs;
      })
      .catch(() => {})
      .finally(() => {
        orgsLoading = false;
      });

    billingApi
      .getStatus()
      .then((status) => {
        billingStatus = status;
      })
      .catch(() => {});

    apiKeysApi
      .list()
      .then((res) => {
        apiKeys = res;
      })
      .catch(() => {})
      .finally(() => {
        keysLoading = false;
      });

    // Fetch public settings to determine if email notifications feature is enabled
    apiClient
      .get<{ email_notifications_enabled?: boolean }>("/api/settings")
      .then((s) => {
        emailNotificationsEnabled = s.email_notifications_enabled ?? false;
      })
      .catch(() => {});

    // Fetch notification preferences (only meaningful when feature is enabled,
    // but we pre-load regardless so there's no flicker when the flag arrives)
    notificationsApi
      .getPreferences()
      .then((prefs) => {
        notifPrefs = prefs;
      })
      .catch(() => {})
      .finally(() => {
        notifPrefsLoading = false;
      });
  });

  function formatDate(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString();
  }

  const tierColors: Record<string, string> = {
    free: "bg-gray-100 text-gray-600",
    pro: "bg-blue-100 text-blue-700",
    business: "bg-amber-100 text-amber-700"
  };

  // Check if user can create API keys
  const canCreateApiKeys = $derived(() => {
    if (!billingStatus?.tier) return false;
    return billingStatus.tier !== "free";
  });

  function openCreateModal() {
    // Pre-select all orgs by default
    newKeySelectedOrgIds = userOrgs.map((o) => o.id);
    newKeyName = "";
    newKeyExpires = 30;
    newlyGeneratedToken = null;
    createError = null;
    copySuccess = false;
    isCreateModalOpen = true;
  }

  function toggleCreateOrgSelection(orgId: string) {
    if (newKeySelectedOrgIds.includes(orgId)) {
      newKeySelectedOrgIds = newKeySelectedOrgIds.filter((id) => id !== orgId);
    } else {
      newKeySelectedOrgIds = [...newKeySelectedOrgIds, orgId];
    }
  }

  async function handleCreateKey() {
    if (!newKeyName.trim()) return;
    isCreatingKey = true;
    createError = null;
    try {
      const res = await apiKeysApi.create(
        newKeyName,
        newKeyExpires,
        newKeySelectedOrgIds
      );
      newlyGeneratedToken = res.raw_token;
      // Refresh the list immediately
      apiKeys = await apiKeysApi.list();
    } catch (e: unknown) {
      const errorStatus =
        e && typeof e === "object" && "status" in e
          ? (e as { status: number }).status
          : undefined;
      if (errorStatus === 403) {
        createError =
          "API keys are available on Pro plans and higher. Upgrade your plan to access this feature.";
      } else {
        createError = "Failed to create API key. Please try again.";
      }
    } finally {
      isCreatingKey = false;
    }
  }

  function handleRevokeKey(id: string) {
    keyToDelete = id;
  }

  function openEditScope(key: ApiKey) {
    editScopeKey = key;
    // Pre-populate with current org IDs; fall back to all user orgs if empty (legacy key)
    editScopeOrgIds =
      key.org_ids.length > 0 ? [...key.org_ids] : userOrgs.map((o) => o.id);
    editScopeError = null;
  }

  function toggleEditScopeOrg(orgId: string) {
    if (editScopeOrgIds.includes(orgId)) {
      editScopeOrgIds = editScopeOrgIds.filter((id) => id !== orgId);
    } else {
      editScopeOrgIds = [...editScopeOrgIds, orgId];
    }
  }

  async function handleSaveScope() {
    if (!editScopeKey || editScopeOrgIds.length === 0) return;
    isSavingScope = true;
    editScopeError = null;
    try {
      await apiKeysApi.updateOrgs(editScopeKey.id, editScopeOrgIds);
      // Update the local state immediately
      apiKeys = apiKeys.map((k) =>
        k.id === editScopeKey!.id ? { ...k, org_ids: editScopeOrgIds } : k
      );
      editScopeKey = null;
    } catch {
      editScopeError = "Failed to update scope. Please try again.";
    } finally {
      isSavingScope = false;
    }
  }

  async function confirmRevokeKey() {
    if (!keyToDelete) return;

    isDeletingKey = true;
    try {
      await apiKeysApi.revoke(keyToDelete);
      apiKeys = apiKeys.filter((k) => k.id !== keyToDelete);
      keyToDelete = null;
    } catch {
      alert("Failed to revoke key");
    } finally {
      isDeletingKey = false;
    }
  }

  function copyToken() {
    if (newlyGeneratedToken) {
      navigator.clipboard.writeText(newlyGeneratedToken);
      copySuccess = true;
      // Reset success state after 3 seconds
      setTimeout(() => {
        copySuccess = false;
      }, 3000);
    }
  }

  async function handleToggleNotifPref(
    key: keyof NotificationPreferences,
    value: boolean
  ) {
    notifSaving = true;
    try {
      const updated = await notificationsApi.updatePreferences({
        [key]: value
      });
      notifPrefs = updated;
    } catch {
      // Revert optimistic update on failure
      notifPrefs = { ...notifPrefs, [key]: !value };
    } finally {
      notifSaving = false;
    }
  }
</script>

<svelte:head>
  <title>Account Settings - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50 flex flex-col">
  <Header user={data.user} currentPage="settings" />

  <main class="flex-1 container mx-auto px-4 py-12">
    <div class="max-w-3xl mx-auto">
      <h1 class="text-3xl font-bold text-gray-900 mb-2">Account Settings</h1>
      <p class="text-gray-600 mb-8">
        Manage your personal account preferences and configuration.
      </p>

      <!-- User Info Preview -->
      {#if data.user}
        <div class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6">
          <h3 class="font-semibold text-gray-900 mb-4">Your Account</h3>
          <div class="space-y-3 text-sm">
            <div class="flex justify-between">
              <span class="text-gray-600">Email</span>
              <span class="text-gray-900 font-medium">{data.user.email}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-600">Name</span>
              <span class="text-gray-900 font-medium"
                >{data.user.name || "Not set"}</span
              >
            </div>
            <div class="flex justify-between">
              <span class="text-gray-600">Joined</span>
              <span class="text-gray-900 font-medium"
                >{formatDate(data.user.created_at)}</span
              >
            </div>
          </div>
        </div>

        <!-- Organizations Section -->
        <div class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6">
          <h3 class="font-semibold text-gray-900 mb-4">Your Organizations</h3>
          {#if orgsLoading}
            <div class="flex items-center gap-2 text-sm text-gray-500">
              <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                ></circle>
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
              Loading organizations...
            </div>
          {:else if userOrgs.length === 0}
            <p class="text-sm text-gray-500">
              You don't belong to any organizations yet.
            </p>
          {:else}
            <ul class="divide-y divide-gray-100">
              {#each userOrgs as org (org.id)}
                <li
                  class="flex items-center justify-between py-3 first:pt-0 last:pb-0"
                >
                  <div class="flex items-center gap-3">
                    <span
                      class="w-8 h-8 rounded-md bg-gradient-to-br from-orange-400 to-orange-600 flex items-center justify-center text-white text-xs font-bold flex-shrink-0"
                    >
                      {org.name.charAt(0).toUpperCase()}
                    </span>
                    <div>
                      <p class="text-sm font-medium text-gray-900">
                        {org.name}
                      </p>
                      <p class="text-xs text-gray-500 capitalize">
                        {org.role}
                      </p>
                    </div>
                  </div>
                  <div class="flex items-center gap-3">
                    <span
                      class="text-xs px-2 py-0.5 rounded-full capitalize font-medium {tierColors[
                        org.tier
                      ] ?? 'bg-gray-100 text-gray-600'}"
                    >
                      {org.tier}
                    </span>
                    {#if billingStatus?.billing_account_id && org.billing_account_id && billingStatus.billing_account_id === org.billing_account_id}
                      <a
                        href="/billing"
                        class="text-xs text-orange-600 hover:text-orange-700 font-medium transition-colors"
                        >Billing →</a
                      >
                    {/if}
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        </div>

        <div class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6">
          <div class="flex justify-between items-center mb-4">
            <div>
              <h3 class="font-semibold text-gray-900">Developer API Keys</h3>
              <p class="text-xs text-gray-500 mt-1">
                Manage Personal Access Tokens for programmatic access to the
                API.
              </p>
            </div>
            {#if canCreateApiKeys()}
              <button
                onclick={openCreateModal}
                class="px-4 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white text-sm font-medium rounded-lg hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm"
              >
                Generate New Key
              </button>
            {:else}
              <div class="text-right">
                <p class="text-sm text-gray-500 mb-2">
                  API keys require a <span class="font-semibold text-orange-600"
                    >Pro plan</span
                  > or higher
                </p>
                <a
                  href="/pricing"
                  class="inline-flex items-center px-4 py-2 bg-orange-500 text-white text-sm font-medium rounded-lg hover:bg-orange-600 transition-colors shadow-sm"
                >
                  Upgrade to Pro →
                </a>
              </div>
            {/if}
          </div>

          {#if keysLoading}
            <div class="flex items-center gap-2 text-sm text-gray-500 py-2">
              <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                ></circle>
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
              Loading keys...
            </div>
          {:else if apiKeys.length === 0}
            <p class="text-sm text-gray-500 py-2 border-t border-gray-100">
              You don't have any active API keys.
            </p>
          {:else}
            <ul class="divide-y divide-gray-100 border-t border-gray-100">
              {#each apiKeys as key (key.id)}
                <li class="py-4 first:pt-4 last:pb-0">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <p class="text-sm font-medium text-gray-900">
                        {key.name}
                      </p>
                      <p class="text-xs text-gray-500 font-mono mt-1">
                        {key.hint}
                      </p>
                      {#if userOrgs.length > 1}
                        <p class="text-xs text-gray-400 mt-1">
                          {#if key.org_ids.length === 0}
                            All organizations
                          {:else}
                            {key.org_ids
                              .map(
                                (id) =>
                                  userOrgs.find((o) => o.id === id)?.name ?? id
                              )
                              .join(", ")}
                          {/if}
                        </p>
                      {/if}
                      <p class="text-xs text-gray-400 mt-1">
                        Created {formatDate(key.created_at)}
                        {#if key.last_used_at}
                          • Last used {formatDate(key.last_used_at)}{/if}
                      </p>
                    </div>
                    <div class="flex items-center gap-2 flex-shrink-0">
                      {#if userOrgs.length > 1}
                        <button
                          onclick={() => openEditScope(key)}
                          class="text-xs text-gray-500 hover:text-gray-800 font-medium px-3 py-1.5 rounded-md bg-gray-50 hover:bg-gray-100 transition-colors"
                        >
                          Edit scope
                        </button>
                      {/if}
                      <button
                        onclick={() => handleRevokeKey(key.id)}
                        class="text-xs text-red-600 hover:text-red-800 font-medium px-3 py-1.5 rounded-md bg-red-50 hover:bg-red-100 transition-colors"
                      >
                        Revoke
                      </button>
                    </div>
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}

      <!-- Email Notifications Section -->
      {#if emailNotificationsEnabled}
        <div class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6">
          <h3 class="font-semibold text-gray-900 mb-1">Email Notifications</h3>
          <p class="text-xs text-gray-500 mb-4">
            Choose which emails you'd like to receive from Rushomon.
          </p>

          {#if notifPrefsLoading}
            <div class="flex items-center gap-2 text-sm text-gray-500 py-2">
              <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                ></circle>
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
              Loading preferences...
            </div>
          {:else}
            <ul class="divide-y divide-gray-100">
              <!-- Monthly statistics -->
              <li
                class="flex items-start justify-between py-4 first:pt-0 last:pb-0"
              >
                <div class="flex-1 pr-4">
                  <p class="text-sm font-medium text-gray-900">
                    Monthly statistics summary
                  </p>
                  <p class="text-xs text-gray-500 mt-0.5">
                    Receive a monthly recap of your link performance at the
                    start of each month.
                  </p>
                </div>
                <!-- Toggle switch -->
                <button
                  role="switch"
                  aria-checked={notifPrefs.email_monthly_stats}
                  disabled={notifSaving}
                  onclick={() =>
                    handleToggleNotifPref(
                      "email_monthly_stats",
                      !notifPrefs.email_monthly_stats
                    )}
                  class="relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed {notifPrefs.email_monthly_stats
                    ? 'bg-orange-500'
                    : 'bg-gray-200'}"
                >
                  <span class="sr-only">Monthly statistics email</span>
                  <span
                    class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {notifPrefs.email_monthly_stats
                      ? 'translate-x-5'
                      : 'translate-x-0'}"
                  ></span>
                </button>
              </li>
            </ul>
          {/if}
        </div>
      {/if}

      <!-- Version Information -->
      <div class="mt-6 bg-white rounded-2xl border-2 border-gray-200 p-6">
        <h3 class="font-semibold text-gray-900 mb-4">Application Version</h3>
        <div class="space-y-3 text-sm">
          <div class="flex justify-between">
            <span class="text-gray-600">Version</span>
            <span class="text-gray-900 font-medium">{versionInfo.version}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-600">Application</span>
            <span class="text-gray-900 font-medium">{versionInfo.name}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-600">Build Timestamp</span>
            <span class="text-gray-900 font-medium"
              >{versionInfo.build_timestamp}</span
            >
          </div>
          <div class="flex justify-between">
            <span class="text-gray-600">Git Commit</span>
            <span class="text-gray-900 font-medium text-xs font-mono">
              {versionInfo.git_commit}
            </span>
          </div>
        </div>

        <div class="mt-4 pt-4 border-t border-gray-200">
          <p class="text-xs text-gray-500">
            For detailed version information, visit the
            <a
              href={versionApiUrl}
              target="_blank"
              class="text-blue-600 hover:text-blue-800 underline">version API</a
            >
            or check the
            <a
              href="https://github.com/piffio/rushomon/releases"
              target="_blank"
              class="text-blue-600 hover:text-blue-800 underline"
              >GitHub releases</a
            >.
          </p>
        </div>
      </div>
    </div>
  </main>
</div>

{#if isCreateModalOpen}
  <div
    class="fixed inset-0 bg-gray-900/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
  >
    <div class="bg-white rounded-2xl shadow-xl w-full max-w-md p-6">
      <h2 class="text-xl font-bold text-gray-900 mb-4">Generate API Key</h2>

      {#if newlyGeneratedToken}
        <div
          class="bg-amber-50 border border-amber-200 rounded-xl p-5 mb-5 shadow-sm"
        >
          <p
            class="text-sm text-amber-800 font-bold mb-2 flex items-center gap-2"
          >
            <svg
              class="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
              ><path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              ></path></svg
            >
            Copy this key now!
          </p>
          <p class="text-xs text-amber-700 mb-4">
            For your security, we only show this token once. If you lose it, you
            will need to revoke it and generate a new one.
          </p>
          <div class="flex gap-2">
            <input
              type="text"
              readonly
              value={newlyGeneratedToken}
              class="flex-1 text-sm font-mono bg-white border border-amber-300 rounded-lg px-3 py-2 text-gray-900 outline-none"
            />
            <button
              onclick={copyToken}
              class="px-4 py-2 bg-amber-600 text-white text-sm font-medium rounded-lg hover:bg-amber-700 transition-colors shadow-sm flex items-center gap-2 min-w-[80px] justify-center"
            >
              {#if copySuccess}
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
                    d="M5 13l4 4L19 7"
                  ></path>
                </svg>
                Copied!
              {:else}
                Copy
              {/if}
            </button>
          </div>
        </div>
        <button
          onclick={() => (isCreateModalOpen = false)}
          class="w-full py-2.5 bg-gray-100 text-gray-900 font-medium rounded-lg hover:bg-gray-200 transition-colors"
        >
          I have copied it safely
        </button>
      {:else}
        <!-- Error Display -->
        {#if createError}
          <div class="bg-red-50 border border-red-200 rounded-xl p-4 mb-5">
            <div class="flex items-start gap-3">
              <svg
                class="w-5 h-5 text-red-500 mt-0.5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                ></path>
              </svg>
              <div>
                <p class="text-sm text-red-800 font-medium">
                  Unable to create API key
                </p>
                <p class="text-sm text-red-700 mt-1">
                  {createError}
                </p>
              </div>
            </div>
          </div>
        {/if}

        <div class="space-y-5 mb-8">
          <div>
            <label
              for="keyName"
              class="block text-sm font-medium text-gray-700 mb-1.5"
              >Key Name</label
            >
            <input
              id="keyName"
              type="text"
              bind:value={newKeyName}
              placeholder="e.g., Zapier Integration"
              class="w-full border border-gray-300 rounded-lg px-3 py-2.5 text-sm focus:ring-2 focus:ring-gray-900 focus:border-gray-900 outline-none transition-shadow"
            />
          </div>
          <div>
            <label
              for="keyExpiration"
              class="block text-sm font-medium text-gray-700 mb-1.5"
              >Expiration</label
            >
            <select
              id="keyExpiration"
              bind:value={newKeyExpires}
              class="w-full border border-gray-300 rounded-lg px-3 py-2.5 text-sm focus:ring-2 focus:ring-gray-900 focus:border-gray-900 outline-none bg-white"
            >
              <option value={7}>7 days</option>
              <option value={30}>30 days</option>
              <option value={90}>90 days</option>
              <option value={null}>Never expire</option>
            </select>
          </div>
          {#if userOrgs.length > 1}
            <div>
              <p class="block text-sm font-medium text-gray-700 mb-2">
                Organization scope
              </p>
              <p class="text-xs text-gray-500 mb-3">
                Choose which organizations this key can act on behalf of.
              </p>
              <ul class="space-y-2">
                {#each userOrgs as org (org.id)}
                  <li>
                    <label class="flex items-center gap-3 cursor-pointer group">
                      <input
                        type="checkbox"
                        checked={newKeySelectedOrgIds.includes(org.id)}
                        onchange={() => toggleCreateOrgSelection(org.id)}
                        class="h-4 w-4 rounded border-gray-300 text-orange-500 focus:ring-orange-400"
                      />
                      <span
                        class="text-sm text-gray-800 group-hover:text-gray-900"
                        >{org.name}</span
                      >
                      <span
                        class="text-xs px-1.5 py-0.5 rounded-full capitalize font-medium ml-auto {tierColors[
                          org.tier
                        ] ?? 'bg-gray-100 text-gray-600'}">{org.tier}</span
                      >
                    </label>
                  </li>
                {/each}
              </ul>
              {#if newKeySelectedOrgIds.length === 0}
                <p class="text-xs text-red-600 mt-2">
                  Select at least one organization.
                </p>
              {/if}
            </div>
          {/if}
        </div>
        <div class="flex justify-end gap-3 pt-2 border-t border-gray-100">
          <button
            onclick={() => (isCreateModalOpen = false)}
            class="px-4 py-2 text-sm font-medium text-gray-600 hover:text-gray-900 hover:bg-gray-100 rounded-lg transition-colors"
          >
            Cancel
          </button>
          <button
            onclick={handleCreateKey}
            disabled={!newKeyName.trim() ||
              isCreatingKey ||
              newKeySelectedOrgIds.length === 0}
            class="px-5 py-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white text-sm font-medium rounded-lg hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm flex items-center justify-center min-w-[120px] disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {#if isCreatingKey}
              <svg
                class="animate-spin -ml-1 mr-2 h-4 w-4 text-white"
                fill="none"
                viewBox="0 0 24 24"
              >
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                ></circle>
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                ></path>
              </svg>
              Generating
            {:else}
              Generate Key
            {/if}
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<!-- Delete API Key Confirmation Modal -->
{#if keyToDelete}
  <div
    class="fixed inset-0 bg-gray-900/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
    use:backdropClose={() => (keyToDelete = null)}
  >
    <div
      class="bg-white rounded-2xl shadow-2xl max-w-md w-full p-6"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="0"
      onkeydown={(e) => e.key === "Escape" && (keyToDelete = null)}
    >
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-lg font-semibold text-gray-900">Revoke API Key?</h3>
        <button
          onclick={() => (keyToDelete = null)}
          class="text-gray-400 hover:text-gray-600 transition-colors"
          aria-label="Close dialog"
        >
          <svg
            class="w-6 h-6"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            ></path>
          </svg>
        </button>
      </div>

      <div class="mb-6">
        <p class="text-gray-600 mb-3">
          Are you sure you want to revoke this API key? Any integrations using
          it will immediately break.
        </p>
        <p class="text-sm text-orange-600 font-medium">
          This action cannot be undone.
        </p>
      </div>

      <div class="flex gap-3">
        <button
          onclick={() => (keyToDelete = null)}
          disabled={isDeletingKey}
          class="flex-1 px-4 py-2.5 bg-gray-100 text-gray-900 font-medium rounded-lg hover:bg-gray-200 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Cancel
        </button>
        <button
          onclick={confirmRevokeKey}
          disabled={isDeletingKey}
          class="flex-1 px-4 py-2.5 bg-red-600 text-white font-medium rounded-lg hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center"
        >
          {#if isDeletingKey}
            <svg
              class="animate-spin -ml-1 mr-2 h-4 w-4 text-white"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle
                class="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                stroke-width="4"
              ></circle>
              <path
                class="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              ></path>
            </svg>
            Revoking...
          {:else}
            Revoke Key
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Edit Scope Modal -->
{#if editScopeKey}
  <div
    class="fixed inset-0 bg-gray-900/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
    use:backdropClose={() => (editScopeKey = null)}
  >
    <div
      class="bg-white rounded-2xl shadow-2xl max-w-md w-full p-6"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="0"
      onkeydown={(e) => e.key === "Escape" && (editScopeKey = null)}
    >
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-lg font-semibold text-gray-900">Edit Key Scope</h3>
        <button
          onclick={() => (editScopeKey = null)}
          class="text-gray-400 hover:text-gray-600 transition-colors"
          aria-label="Close dialog"
        >
          <svg
            class="w-6 h-6"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            ></path>
          </svg>
        </button>
      </div>

      <p class="text-sm text-gray-500 mb-4">
        Choose which organizations <span class="font-medium text-gray-800"
          >{editScopeKey.name}</span
        > is allowed to act on behalf of.
      </p>

      {#if editScopeError}
        <div class="bg-red-50 border border-red-200 rounded-lg p-3 mb-4">
          <p class="text-sm text-red-700">{editScopeError}</p>
        </div>
      {/if}

      <ul class="space-y-2 mb-6">
        {#each userOrgs as org (org.id)}
          <li>
            <label class="flex items-center gap-3 cursor-pointer group">
              <input
                type="checkbox"
                checked={editScopeOrgIds.includes(org.id)}
                onchange={() => toggleEditScopeOrg(org.id)}
                class="h-4 w-4 rounded border-gray-300 text-orange-500 focus:ring-orange-400"
              />
              <span class="text-sm text-gray-800 group-hover:text-gray-900"
                >{org.name}</span
              >
              <span
                class="text-xs px-1.5 py-0.5 rounded-full capitalize font-medium ml-auto {tierColors[
                  org.tier
                ] ?? 'bg-gray-100 text-gray-600'}">{org.tier}</span
              >
            </label>
          </li>
        {/each}
      </ul>

      {#if editScopeOrgIds.length === 0}
        <p class="text-xs text-red-600 mb-4">
          Select at least one organization.
        </p>
      {/if}

      <div class="flex gap-3">
        <button
          onclick={() => (editScopeKey = null)}
          disabled={isSavingScope}
          class="flex-1 px-4 py-2.5 bg-gray-100 text-gray-900 font-medium rounded-lg hover:bg-gray-200 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Cancel
        </button>
        <button
          onclick={handleSaveScope}
          disabled={editScopeOrgIds.length === 0 || isSavingScope}
          class="flex-1 px-4 py-2.5 bg-gradient-to-r from-orange-500 to-orange-600 text-white font-medium rounded-lg hover:from-orange-600 hover:to-orange-700 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center"
        >
          {#if isSavingScope}
            <svg
              class="animate-spin -ml-1 mr-2 h-4 w-4 text-white"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle
                class="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                stroke-width="4"
              ></circle>
              <path
                class="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              ></path>
            </svg>
            Saving...
          {:else}
            Save Scope
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}
