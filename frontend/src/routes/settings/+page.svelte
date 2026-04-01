<script lang="ts">
  import Header from "$lib/components/Header.svelte";
  import { orgsApi } from "$lib/api/orgs";
  import { billingApi } from "$lib/api/billing";
  import { apiClient } from "$lib/api/client";
  import { apiKeysApi, type ApiKey } from "$lib/api/settings";
  import type { PageData } from "./$types";
  import type { OrgWithRole } from "$lib/types/api";
  import type { BillingStatus } from "$lib/api/billing";

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
  let newlyGeneratedToken = $state<string | null>(null);
  let isCreatingKey = $state(false);
  let createError = $state<string | null>(null);
  let copySuccess = $state(false);

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

  async function handleCreateKey() {
    if (!newKeyName.trim()) return;
    isCreatingKey = true;
    createError = null;
    try {
      const res = await apiKeysApi.create(newKeyName, newKeyExpires);
      newlyGeneratedToken = res.raw_token;
      // Refresh the list immediately
      apiKeys = await apiKeysApi.list();
    } catch (e: any) {
      if (e?.status === 403) {
        createError =
          "API keys are available on Pro plans and higher. Upgrade your plan to access this feature.";
      } else {
        createError = "Failed to create API key. Please try again.";
      }
    } finally {
      isCreatingKey = false;
    }
  }

  async function handleRevokeKey(id: string) {
    keyToDelete = id;
  }

  async function confirmRevokeKey() {
    if (!keyToDelete) return;

    isDeletingKey = true;
    try {
      await apiKeysApi.revoke(keyToDelete);
      apiKeys = apiKeys.filter((k) => k.id !== keyToDelete);
      keyToDelete = null;
    } catch (e) {
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
              {#each userOrgs as org}
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
                    {#if billingStatus?.is_billing_owner && org.role === "owner"}
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
                onclick={() => {
                  isCreateModalOpen = true;
                  newlyGeneratedToken = null;
                  newKeyName = "";
                  createError = null;
                  copySuccess = false;
                }}
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
              {#each apiKeys as key}
                <li
                  class="flex items-center justify-between py-4 first:pt-4 last:pb-0"
                >
                  <div>
                    <p class="text-sm font-medium text-gray-900">
                      {key.name}
                    </p>
                    <p class="text-xs text-gray-500 font-mono mt-1">
                      {key.hint}
                    </p>
                    <p class="text-xs text-gray-400 mt-1.5">
                      Created {formatDate(key.created_at)}
                      {#if key.last_used_at}
                        • Last used {formatDate(key.last_used_at)}{/if}
                    </p>
                  </div>
                  <button
                    onclick={() => handleRevokeKey(key.id)}
                    class="text-xs text-red-600 hover:text-red-800 font-medium px-3 py-1.5 rounded-md bg-red-50 hover:bg-red-100 transition-colors"
                  >
                    Revoke
                  </button>
                </li>
              {/each}
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
            disabled={!newKeyName.trim() || isCreatingKey}
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
    role="button"
    tabindex="0"
    onclick={() => (keyToDelete = null)}
    onkeydown={(e) => e.key === "Escape" && (keyToDelete = null)}
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
