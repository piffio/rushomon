<script lang="ts">
  import { onMount } from "svelte";
  import { orgsApi } from "$lib/api/orgs";
  import type { OrgDomain } from "$lib/types/api";
  import LoadingButton from "$lib/components/LoadingButton.svelte";

  let { orgId }: { orgId: string } = $props();

  let domains = $state<OrgDomain[]>([]);
  let newDomain = $state("");
  let isLoading = $state(true);
  let isSubmitting = $state(false);
  let verifyingDomain = $state<string | null>(null);
  let error = $state<string | null>(null);
  let successMessage = $state<string | null>(null);

  // Track which domain's token was just copied for the UI checkmark
  let copiedDomain = $state<string | null>(null);

  // State for the delete confirmation modal
  let confirmingDeleteDomain = $state<string | null>(null);

  onMount(async () => {
    await loadDomains();
    autoVerifyPending();
  });

  async function loadDomains() {
    try {
      isLoading = true;
      const res = await orgsApi.getOrgDomains(orgId);
      domains = res.domains;
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to load domains";
    } finally {
      isLoading = false;
    }
  }

  async function handleAdd() {
    if (!newDomain.trim()) return;
    try {
      error = null;
      isSubmitting = true;
      const domainStr = newDomain.trim();

      const res = await orgsApi.addOrgDomain(orgId, domainStr);
      newDomain = "";
      domains = [...domains, res.domain];

      successMessage = "Domain added. Please add the TXT record to verify.";
      setTimeout(() => (successMessage = null), 5000);
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to add domain";
    } finally {
      isSubmitting = false;
    }
  }

  async function handleVerify(domain: string) {
    try {
      error = null;
      verifyingDomain = domain;
      await orgsApi.verifyOrgDomain(orgId, domain);
      await loadDomains();
      successMessage = `${domain} verified successfully!`;
      setTimeout(() => (successMessage = null), 5000);
    } catch (e) {
      error =
        e instanceof Error
          ? e.message
          : `Failed to verify ${domain}. Make sure the TXT record is propagated.`;
    } finally {
      verifyingDomain = null;
    }
  }

  function startDeleteDomain(domain: string) {
    confirmingDeleteDomain = domain;
  }

  function cancelDeleteDomain() {
    confirmingDeleteDomain = null;
  }

  async function confirmDeleteDomain() {
    if (!confirmingDeleteDomain) return;
    try {
      error = null;
      await orgsApi.deleteOrgDomain(orgId, confirmingDeleteDomain);
      await loadDomains();
      successMessage = `${confirmingDeleteDomain} removed successfully.`;
      setTimeout(() => (successMessage = null), 5000);
    } catch (e) {
      error = e instanceof Error ? e.message : "Failed to delete domain";
    } finally {
      confirmingDeleteDomain = null;
    }
  }

  async function copyToClipboard(text: string, domain: string) {
    try {
      if (navigator?.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const textArea = document.createElement("textarea");
        textArea.value = text;
        document.body.appendChild(textArea);
        textArea.select();
        document.execCommand("copy");
        document.body.removeChild(textArea);
      }
      copiedDomain = domain;
      setTimeout(() => {
        if (copiedDomain === domain) copiedDomain = null;
      }, 2000);
    } catch (err) {
      console.error("Failed to copy text", err);
    }
  }

  // Silently attempt to verify pending domains in the background — the DNS
  // record may already be in place (e.g. after a page refresh).
  async function autoVerifyPending() {
    const pendingDomains = domains.filter((d) => !d.is_verified);
    if (pendingDomains.length === 0) return;

    let newlyVerified = false;
    for (const d of pendingDomains) {
      try {
        await orgsApi.verifyOrgDomain(orgId, d.domain);
        newlyVerified = true;
      } catch {
        // Silently ignore — DNS simply hasn't propagated yet
      }
    }

    if (newlyVerified) {
      const res = await orgsApi.getOrgDomains(orgId);
      domains = res.domains;
    }
  }
</script>

<div
  class="bg-white rounded-xl border border-gray-200 overflow-hidden shadow-sm mb-6"
>
  <div class="p-6 border-b border-gray-200">
    <h2 class="text-lg font-semibold text-gray-900 mb-1">
      Organization Domains
    </h2>
    <p class="text-sm text-gray-500 mb-4">
      Verify your domain(s) to automatically provision accounts and add new
      users to this organization when they sign in.
    </p>

    {#if error}
      <div
        class="mb-4 bg-red-50 p-4 rounded-lg text-sm text-red-700 border border-red-200"
      >
        {error}
      </div>
    {/if}
    {#if successMessage}
      <div
        class="mb-4 bg-green-50 p-4 rounded-lg text-sm text-green-700 border border-green-200"
      >
        {successMessage}
      </div>
    {/if}

    <form
      onsubmit={(e) => {
        e.preventDefault();
        handleAdd();
      }}
      class="flex items-start gap-3"
    >
      <div class="flex-1">
        <input
          type="text"
          bind:value={newDomain}
          placeholder="example.com"
          required
          class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-orange-500 focus:border-orange-500 outline-none"
          disabled={isSubmitting}
        />
      </div>
      <LoadingButton
        type="submit"
        loading={isSubmitting}
        disabled={isSubmitting || !newDomain.trim()}
        variant="primary"
      >
        {isSubmitting ? "Adding…" : "Add Domain"}
      </LoadingButton>
    </form>
    <p class="text-xs text-gray-500 mt-0.5">
      Enter the part of your organizational email address after the @ sign.
    </p>
  </div>

  <div>
    {#if isLoading}
      <div class="flex justify-center p-6">
        <span class="text-gray-500 text-sm">Loading domains...</span>
      </div>
    {:else if domains.length === 0}
      <div class="text-center p-8 bg-gray-50">
        <p class="text-sm text-gray-500">No domains added yet.</p>
      </div>
    {:else}
      <ul class="divide-y divide-gray-200">
        {#each domains as d (d.id)}
          <li class="p-6 hover:bg-gray-50 transition-colors relative">
            <button
              type="button"
              onclick={() => startDeleteDomain(d.domain)}
              class="absolute top-6 right-6 text-xs text-red-500 hover:text-red-700 transition-colors"
            >
              Remove
            </button>

            <div class="w-full">
              <div class="flex items-center gap-3 pr-16">
                <p class="text-base font-semibold text-gray-900 truncate">
                  {d.domain}
                </p>
                {#if d.is_verified}
                  <span
                    class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800 border border-green-200"
                  >
                    Verified
                  </span>
                {:else}
                  <span
                    class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800 border border-yellow-200"
                  >
                    Pending Verification
                  </span>
                {/if}
              </div>

              {#if !d.is_verified && d.verification_token}
                <div class="mt-4 w-full">
                  <div class="flex items-center gap-2 mb-3">
                    <p class="font-medium text-sm text-gray-900">
                      Add this TXT record to your DNS settings:
                    </p>

                    {#if d.is_cloudflare}
                      <a
                        href={`https://dash.cloudflare.com/?to=/:account/${d.domain}/dns`}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="inline-flex items-center gap-1 text-xs font-medium text-orange-600 hover:text-orange-700 transition-colors"
                      >
                        Open Cloudflare
                        <svg
                          class="w-3.5 h-3.5"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                          ><path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                          ></path></svg
                        >
                      </a>
                    {/if}
                  </div>

                  <div class="flex flex-col sm:flex-row sm:gap-6 gap-2 mb-3">
                    <div
                      class="text-gray-500 text-xs uppercase tracking-wider font-semibold"
                    >
                      Type: <span class="font-mono text-gray-900 ml-1">TXT</span
                      >
                    </div>
                    <div
                      class="text-gray-500 text-xs uppercase tracking-wider font-semibold"
                    >
                      Name: <span class="font-mono text-gray-900 ml-1 lowercase"
                        >{d.domain}</span
                      >
                      <span
                        class="normal-case tracking-normal font-normal text-gray-400 ml-1"
                        >(use @ if this is your root domain)</span
                      >
                    </div>
                  </div>

                  <div class="flex items-start gap-3 w-full mt-1">
                    <div class="flex flex-1 shadow-sm">
                      <input
                        type="text"
                        readonly
                        value={`rushomon-verification=${d.verification_token}`}
                        class="flex-1 w-full font-mono text-sm bg-gray-50 border border-gray-300 rounded-l-lg px-3 py-2 text-gray-800 focus:outline-none"
                      />
                      <button
                        type="button"
                        onclick={() =>
                          copyToClipboard(
                            `rushomon-verification=${d.verification_token}`,
                            d.domain
                          )}
                        class="px-4 py-2 border border-l-0 border-gray-300 rounded-r-lg text-gray-600 bg-gray-100 hover:bg-gray-200 transition-colors focus:outline-none flex items-center justify-center min-w-[48px]"
                        title="Copy to clipboard"
                      >
                        {#if copiedDomain === d.domain}
                          <svg
                            class="w-5 h-5 text-green-600"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                            ><path
                              stroke-linecap="round"
                              stroke-linejoin="round"
                              stroke-width="2"
                              d="M5 13l4 4L19 7"
                            ></path></svg
                          >
                        {:else}
                          <svg
                            class="w-5 h-5"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                            ><path
                              stroke-linecap="round"
                              stroke-linejoin="round"
                              stroke-width="2"
                              d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
                            ></path></svg
                          >
                        {/if}
                      </button>
                    </div>

                    <button
                      onclick={() => handleVerify(d.domain)}
                      disabled={verifyingDomain === d.domain}
                      class="px-4 py-2 bg-orange-500 hover:bg-orange-600 text-white rounded-lg text-sm font-medium transition-colors disabled:opacity-50 whitespace-nowrap min-w-[100px] flex items-center justify-center shadow-sm"
                    >
                      {#if verifyingDomain === d.domain}
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
                        Verifying...
                      {:else}
                        Verify Now
                      {/if}
                    </button>
                  </div>

                  <p class="mt-3 text-xs text-gray-500">
                    It may take several minutes for DNS changes to propagate
                    before verification succeeds.
                  </p>
                </div>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

{#if confirmingDeleteDomain}
  <div
    class="modal-backdrop"
    role="button"
    tabindex="0"
    onclick={cancelDeleteDomain}
    onkeydown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        cancelDeleteDomain();
      }
    }}
  >
    <div
      class="modal"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="modal-header">
        <h3>Remove Domain?</h3>
        <button class="modal-close" onclick={cancelDeleteDomain}>&times;</button
        >
      </div>
      <div class="modal-body">
        <p>
          Are you sure you want to remove <strong
            >{confirmingDeleteDomain}</strong
          >? Users logging in with this domain will no longer automatically join
          this organization.
        </p>
      </div>
      <div class="modal-footer">
        <button class="btn btn-secondary" onclick={cancelDeleteDomain}>
          Cancel
        </button>
        <button class="btn btn-danger" onclick={confirmDeleteDomain}>
          Remove Domain
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
