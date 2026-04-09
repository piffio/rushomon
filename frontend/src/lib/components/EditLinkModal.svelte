<script lang="ts">
  import { linksApi } from "$lib/api/links";
  import type {
    Link,
    LinkStatus,
    UpdateLinkRequest,
    UtmParams
  } from "$lib/types/api";
  import { createEventDispatcher } from "svelte";
  import LoadingButton from "./LoadingButton.svelte";

  let {
    link,
    isOpen = $bindable(false),
    isPro = false
  }: {
    link: Link;
    isOpen?: boolean;
    isPro?: boolean;
  } = $props();

  const dispatch = createEventDispatcher<{ close: void; updated: Link }>();

  let destinationUrl = $state("");
  let title = $state("");
  let expiresAt = $state("");
  let status = $state<LinkStatus>("active");

  // Pro features
  let showUtmBuilder = $state(false);
  let utmSource = $state("");
  let utmMedium = $state("");
  let utmCampaign = $state("");
  let utmTerm = $state("");
  let utmContent = $state("");
  let forwardQueryParams = $state<boolean | null>(null);

  let isSubmitting = $state(false);
  let error = $state("");

  function hasUtmParams(): boolean {
    return !!(
      utmSource.trim() ||
      utmMedium.trim() ||
      utmCampaign.trim() ||
      utmTerm.trim() ||
      utmContent.trim()
    );
  }

  // Reset form when link prop changes
  $effect(() => {
    destinationUrl = link.destination_url;
    title = link.title || "";
    expiresAt = link.expires_at
      ? new Date(link.expires_at * 1000).toISOString().slice(0, 16)
      : "";
    status = link.status;
    // UTM fields
    utmSource = link.utm_params?.utm_source || "";
    utmMedium = link.utm_params?.utm_medium || "";
    utmCampaign = link.utm_params?.utm_campaign || "";
    utmTerm = link.utm_params?.utm_term || "";
    utmContent = link.utm_params?.utm_content || "";
    showUtmBuilder = !!(
      link.utm_params &&
      (link.utm_params.utm_source ||
        link.utm_params.utm_medium ||
        link.utm_params.utm_campaign ||
        link.utm_params.utm_term ||
        link.utm_params.utm_content)
    );
    forwardQueryParams = link.forward_query_params ?? null;
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    error = "";
    isSubmitting = true;

    try {
      const updateData: UpdateLinkRequest = {};

      // Only send changed fields
      if (destinationUrl !== link.destination_url) {
        updateData.destination_url = destinationUrl;
      }
      if (title !== (link.title || "")) {
        updateData.title = title || undefined;
      }
      if (status !== link.status) {
        updateData.status = status;
      }
      if (expiresAt) {
        updateData.expires_at = Math.floor(
          new Date(expiresAt).getTime() / 1000
        );
      } else if (link.expires_at) {
        updateData.expires_at = undefined;
      }

      // UTM params: send if Pro and changed
      if (isPro) {
        const newUtm: UtmParams = {
          utm_source: utmSource.trim() || undefined,
          utm_medium: utmMedium.trim() || undefined,
          utm_campaign: utmCampaign.trim() || undefined,
          utm_term: utmTerm.trim() || undefined,
          utm_content: utmContent.trim() || undefined
        };
        const origUtm = link.utm_params;
        const utmChanged =
          (newUtm.utm_source || "") !== (origUtm?.utm_source || "") ||
          (newUtm.utm_medium || "") !== (origUtm?.utm_medium || "") ||
          (newUtm.utm_campaign || "") !== (origUtm?.utm_campaign || "") ||
          (newUtm.utm_term || "") !== (origUtm?.utm_term || "") ||
          (newUtm.utm_content || "") !== (origUtm?.utm_content || "");
        if (utmChanged) {
          updateData.utm_params = newUtm;
        }

        if (forwardQueryParams !== (link.forward_query_params ?? null)) {
          updateData.forward_query_params = forwardQueryParams ?? undefined;
        }
      }

      const updatedLink = await linksApi.update(link.id, updateData);
      dispatch("updated", updatedLink);
      dispatch("close");
      isOpen = false;
    } catch (err: unknown) {
      error = err instanceof Error ? err.message : "Failed to update link";
    } finally {
      isSubmitting = false;
    }
  }

  function handleClose() {
    if (!isSubmitting) {
      dispatch("close");
      isOpen = false;
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4"
  >
    <div class="bg-white rounded-lg shadow-xl max-w-md w-full">
      <div class="p-6">
        <h3 class="text-xl font-bold text-gray-900 mb-4">Edit Link</h3>

        <form onsubmit={handleSubmit}>
          <!-- Short Code (read-only) -->
          <div class="mb-4">
            <label
              for="short-code-readonly"
              class="block text-sm font-medium text-gray-700 mb-1"
            >
              Short Code
            </label>
            <input
              type="text"
              id="short-code-readonly"
              value={link.short_code}
              disabled
              class="w-full px-4 py-2 border border-gray-300 rounded-lg bg-gray-100 text-gray-500 cursor-not-allowed"
            />
            <p class="text-xs text-gray-500 mt-1">
              Short codes cannot be changed
            </p>
          </div>

          <!-- Destination URL -->
          <div class="mb-4">
            <label
              for="destination-url-edit"
              class="block text-sm font-medium text-gray-700 mb-1"
            >
              Destination URL <span class="text-red-500">*</span>
            </label>
            <input
              type="url"
              id="destination-url-edit"
              bind:value={destinationUrl}
              required
              placeholder="https://example.com"
              class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
            />
          </div>

          <!-- Title -->
          <div class="mb-4">
            <label
              for="title-edit"
              class="block text-sm font-medium text-gray-700 mb-1"
            >
              Title <span class="text-gray-500 text-xs font-normal"
                >(Optional)</span
              >
            </label>
            <input
              type="text"
              id="title-edit"
              bind:value={title}
              placeholder="My Link"
              maxlength="200"
              class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
            />
          </div>

          <!-- Status -->
          <div class="mb-4">
            <label
              for="status-edit"
              class="block text-sm font-medium text-gray-700 mb-1"
            >
              Status
            </label>
            <select
              id="status-edit"
              bind:value={status}
              class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
            >
              <option value="active">Active - Link redirects normally</option>
              <option value="disabled">Disabled - Link returns 404</option>
            </select>
            <p class="text-xs text-gray-500 mt-1">
              Disabled links don't redirect but keep the short code reserved
            </p>
          </div>

          <!-- Expiration Date -->
          <div class="mb-4">
            <label
              for="expires-at-edit"
              class="block text-sm font-medium text-gray-700 mb-1"
            >
              Expiration Date <span class="text-gray-500 text-xs font-normal"
                >(Optional)</span
              >
            </label>
            <input
              type="datetime-local"
              id="expires-at-edit"
              bind:value={expiresAt}
              class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-gray-900 focus:border-transparent"
            />
            <p class="text-xs text-gray-500 mt-1">
              Leave empty for no expiration
            </p>
          </div>

          <!-- Pro Features: UTM Builder + Query Forwarding -->
          {#if isPro}
            <!-- UTM Builder -->
            <div class="mb-4 border border-gray-200 rounded-lg overflow-hidden">
              <button
                type="button"
                class="w-full flex items-center justify-between px-4 py-3 bg-gray-50 hover:bg-gray-100 transition-colors text-sm font-medium text-gray-700"
                onclick={() => (showUtmBuilder = !showUtmBuilder)}
              >
                <span class="flex items-center gap-2">
                  UTM Parameters
                  {#if hasUtmParams()}
                    <span
                      class="bg-indigo-100 text-indigo-700 text-xs px-2 py-0.5 rounded-full"
                      >active</span
                    >
                  {/if}
                </span>
                <svg
                  class="w-4 h-4 text-gray-400 transition-transform {showUtmBuilder
                    ? 'rotate-180'
                    : ''}"
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
              {#if showUtmBuilder}
                <div class="p-4 space-y-3 border-t border-gray-200">
                  <p class="text-xs text-gray-500">
                    Appended to the destination URL on every redirect.
                  </p>
                  <div class="grid grid-cols-2 gap-3">
                    <div>
                      <label
                        for="edit-utm-source"
                        class="block text-xs font-medium text-gray-600 mb-1"
                        >Source</label
                      >
                      <input
                        type="text"
                        id="edit-utm-source"
                        bind:value={utmSource}
                        placeholder="e.g. newsletter"
                        class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
                      />
                    </div>
                    <div>
                      <label
                        for="edit-utm-medium"
                        class="block text-xs font-medium text-gray-600 mb-1"
                        >Medium</label
                      >
                      <input
                        type="text"
                        id="edit-utm-medium"
                        bind:value={utmMedium}
                        placeholder="e.g. email"
                        class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
                      />
                    </div>
                    <div>
                      <label
                        for="edit-utm-campaign"
                        class="block text-xs font-medium text-gray-600 mb-1"
                        >Campaign</label
                      >
                      <input
                        type="text"
                        id="edit-utm-campaign"
                        bind:value={utmCampaign}
                        placeholder="e.g. spring_sale"
                        class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
                      />
                    </div>
                    <div>
                      <label
                        for="edit-utm-term"
                        class="block text-xs font-medium text-gray-600 mb-1"
                        >Term</label
                      >
                      <input
                        type="text"
                        id="edit-utm-term"
                        bind:value={utmTerm}
                        placeholder="e.g. running+shoes"
                        class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
                      />
                    </div>
                    <div class="col-span-2">
                      <label
                        for="edit-utm-content"
                        class="block text-xs font-medium text-gray-600 mb-1"
                        >Content</label
                      >
                      <input
                        type="text"
                        id="edit-utm-content"
                        bind:value={utmContent}
                        placeholder="e.g. banner_top"
                        class="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-md focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
                      />
                    </div>
                  </div>
                </div>
              {/if}
            </div>

            <!-- Forward Query Params Toggle -->
            <div
              class="mb-4 flex items-start gap-3 p-4 border border-gray-200 rounded-lg"
            >
              <div class="flex-1">
                <label
                  for="edit-forward-query-params"
                  class="block text-sm font-medium text-gray-700"
                >
                  Forward visitor query parameters
                </label>
                <p class="text-xs text-gray-500 mt-0.5">
                  Append visitor query params to the destination. Overrides UTM
                  on conflict.
                </p>
              </div>
              <input
                type="checkbox"
                id="edit-forward-query-params"
                checked={forwardQueryParams === true}
                onchange={(e) =>
                  (forwardQueryParams = (e.target as HTMLInputElement).checked)}
                class="mt-0.5 h-4 w-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500"
              />
            </div>
          {/if}

          {#if error}
            <div
              class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm mb-4"
            >
              {error}
            </div>
          {/if}

          <div class="flex gap-3 justify-end">
            <button
              type="button"
              class="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded-lg transition-colors"
              onclick={handleClose}
              disabled={isSubmitting}
            >
              Cancel
            </button>
            <LoadingButton
              type="submit"
              loading={isSubmitting}
              variant="primary"
            >
              {isSubmitting ? "Saving..." : "Save Changes"}
            </LoadingButton>
          </div>
        </form>
      </div>
    </div>
  </div>
{/if}
