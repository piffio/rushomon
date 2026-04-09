<script lang="ts">
  import {
    PUBLIC_VITE_API_BASE_URL,
    PUBLIC_VITE_SHORT_LINK_BASE_URL
  } from "$env/static/public";
  import { orgsApi } from "$lib/api/orgs";
  import type { Link } from "$lib/types/api";
  import QRCodeStyling from "qr-code-styling";

  interface Props {
    link: Link | null;
    isOpen: boolean;
    onClose: () => void;
    tier?: string;
    orgLogoUrl?: string | null;
    orgId?: string;
  }

  const {
    link,
    isOpen,
    onClose,
    tier = "free",
    orgLogoUrl = null,
    orgId = ""
  }: Props = $props();

  const SHORT_LINK_BASE =
    PUBLIC_VITE_SHORT_LINK_BASE_URL ||
    PUBLIC_VITE_API_BASE_URL ||
    "http://localhost:8787";

  const tierLevels: Record<string, number> = {
    free: 0,
    pro: 1,
    business: 2,
    unlimited: 3
  };

  function hasTierAccess(required: string): boolean {
    return (tierLevels[tier] ?? 0) >= (tierLevels[required] ?? 0);
  }

  const isPro = $derived(hasTierAccess("pro"));

  // ── State ────────────────────────────────────────────────────────────────
  let container: HTMLDivElement = $state() as HTMLDivElement;
  let shortUrl = $state("");
  let selectedSize = $state(256);
  let useLogoInQR = $state(false);
  let lockedPopoverOpen = $state(false);
  let qrInstance: QRCodeStyling | null = null;

  // Upload state
  let isUploading = $state(false);
  let uploadError = $state("");
  let uploadedLogoUrl = $state<string | null>(null);

  const sizes = [256, 512, 1024];

  // ── Helpers ──────────────────────────────────────────────────────────────
  function buildAbsoluteLogoUrl(url: string | null): string | undefined {
    if (!url) return undefined;
    if (url.startsWith("http")) return url;
    const base =
      PUBLIC_VITE_API_BASE_URL ||
      (typeof window !== "undefined" ? window.location.origin : "");
    return `${base}${url}`;
  }

  function getQROptions(size: number, withLogo: boolean) {
    const currentLogoUrl = uploadedLogoUrl || orgLogoUrl;
    const logoUrl =
      withLogo && currentLogoUrl
        ? buildAbsoluteLogoUrl(currentLogoUrl)
        : undefined;
    return {
      width: size,
      height: size,
      data: shortUrl,
      margin: Math.round(size * 0.03),
      qrOptions: {
        errorCorrectionLevel: (logoUrl ? "H" : "M") as "H" | "M"
      },
      dotsOptions: { color: "#000000", type: "square" as const },
      backgroundOptions: { color: "#FFFFFF" },
      image: logoUrl,
      imageOptions: {
        crossOrigin: "anonymous" as const,
        margin: 4,
        imageSize: 0.25
      }
    };
  }

  // ── QR generation ────────────────────────────────────────────────────────
  $effect(() => {
    if (isOpen && link) {
      shortUrl = `${SHORT_LINK_BASE}/${link.short_code}`;
    }
  });

  $effect(() => {
    if (isOpen && link && shortUrl && container) {
      renderQR();
    }
  });

  async function renderQR() {
    if (!container || !shortUrl) return;
    // eslint-disable-next-line svelte/no-dom-manipulating
    container.innerHTML = "";
    const opts = getQROptions(selectedSize, useLogoInQR && isPro);
    qrInstance = new QRCodeStyling(opts);
    qrInstance.append(container);
  }

  // ── Downloads ────────────────────────────────────────────────────────────
  async function downloadPNG() {
    if (!qrInstance || !link) return;
    await qrInstance.download({
      name: `${link.short_code}-qr`,
      extension: "png"
    });
  }

  async function downloadSVG() {
    if (!qrInstance || !link) return;
    await qrInstance.download({
      name: `${link.short_code}-qr`,
      extension: "svg"
    });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }

  function selectSize(size: number) {
    if (size > 256 && !isPro) {
      lockedPopoverOpen = true;
      return;
    }
    selectedSize = size;
  }

  function toggleLogo() {
    if (!isPro) {
      lockedPopoverOpen = true;
      return;
    }
    useLogoInQR = !useLogoInQR;
  }

  function tryDownloadSVG() {
    if (!isPro) {
      lockedPopoverOpen = true;
      return;
    }
    downloadSVG();
  }

  async function handleLogoUpload(event: Event) {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];

    if (!file) return;

    // Validate file type
    const allowedTypes = [
      "image/png",
      "image/jpeg",
      "image/webp",
      "image/svg+xml"
    ];
    if (!allowedTypes.includes(file.type)) {
      uploadError = "Invalid file type. Please use PNG, JPEG, WebP, or SVG.";
      return;
    }

    // Validate file size (500KB)
    const maxSize = 500 * 1024;
    if (file.size > maxSize) {
      uploadError = "File too large. Please use an image under 500KB.";
      return;
    }

    isUploading = true;
    uploadError = "";

    try {
      if (!orgId) throw new Error("Organization ID not available");

      const result = await orgsApi.uploadOrgLogo(orgId, file);
      uploadedLogoUrl = result.logo_url;

      // Clear the file input
      target.value = "";
    } catch (error: any) {
      uploadError = error.message || "Failed to upload logo. Please try again.";
    } finally {
      isUploading = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen && link}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="fixed inset-0 z-50 overflow-y-auto"
    aria-labelledby="qr-modal-title"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
  >
    <!-- Backdrop -->
    <div class="fixed inset-0 bg-gray-900/50 transition-opacity"></div>

    <div class="flex min-h-full items-center justify-center p-4">
      <div
        class="relative transform overflow-hidden rounded-lg bg-white shadow-xl transition-all max-w-md w-full"
      >
        <!-- Header -->
        <div
          class="flex items-center justify-between px-6 py-4 border-b border-gray-200"
        >
          <h3 id="qr-modal-title" class="text-lg font-semibold text-gray-900">
            QR Code
          </h3>
          <button
            onclick={onClose}
            class="text-gray-400 hover:text-gray-600 transition-colors p-1 rounded-full hover:bg-gray-100"
            aria-label="Close"
          >
            <svg
              class="w-5 h-5"
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

        <!-- Content -->
        <div class="px-6 py-6">
          {#if link.title}
            <p class="text-sm text-gray-600 mb-4 font-medium truncate">
              {link.title}
            </p>
          {/if}

          <!-- QR Code container: always renders at selectedSize but scales to fit modal -->
          <div class="flex justify-center mb-4">
            <div
              style="width:256px;height:256px;position:relative;overflow:visible;"
            >
              <div
                bind:this={container}
                class="border border-gray-200 rounded-lg overflow-hidden"
                style="width:{selectedSize}px;height:{selectedSize}px;transform-origin:top left;transform:scale({256 /
                  selectedSize});position:absolute;top:0;left:0;"
              ></div>
            </div>
          </div>

          <!-- Short URL -->
          <p class="text-center text-sm text-gray-600 mb-5 font-mono break-all">
            {shortUrl}
          </p>

          <!-- Size selector -->
          <div class="mb-4">
            <p class="text-xs font-medium text-gray-500 mb-2">Size</p>
            <div class="flex gap-2">
              {#each sizes as size (size)}
                <button
                  onclick={() => selectSize(size)}
                  class="flex-1 py-1.5 rounded-lg text-sm font-medium border transition-colors
										{selectedSize === size
                    ? 'bg-orange-500 text-white border-orange-500'
                    : 'bg-white text-gray-700 border-gray-300 hover:border-orange-400'}
										{size > 256 && !isPro ? 'opacity-60' : ''}"
                >
                  {size}px
                  {#if size > 256 && !isPro}
                    <span class="ml-1">🔒</span>
                  {/if}
                </button>
              {/each}
            </div>
          </div>

          <!-- Logo options -->
          {#if orgLogoUrl || uploadedLogoUrl}
            <!-- User has logo - show toggle -->
            <div class="mb-5 flex items-center justify-between">
              <div>
                <p class="text-sm font-medium text-gray-700">Embed org logo</p>
                <p class="text-xs text-gray-500">
                  {#if isPro}
                    Centred in the QR code
                  {:else}
                    <span class="text-orange-600">Pro feature</span>
                  {/if}
                </p>
              </div>
              <button
                role="switch"
                aria-checked={useLogoInQR}
                aria-label="Embed org logo in QR code"
                onclick={toggleLogo}
                class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none
									{useLogoInQR && isPro ? 'bg-orange-500' : 'bg-gray-200'}"
              >
                <span
                  class="inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform
										{useLogoInQR && isPro ? 'translate-x-6' : 'translate-x-1'}"
                ></span>
              </button>
            </div>
          {:else if isPro && !orgLogoUrl && !uploadedLogoUrl}
            <!-- Pro user without logo - upload invitation -->
            <div class="mb-5 p-4 bg-blue-50 border border-blue-200 rounded-lg">
              <div class="flex items-start gap-3">
                <svg
                  class="w-5 h-5 text-blue-600 mt-0.5 flex-shrink-0"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M12 6v6m0 0v6m0-6h6m-6 0H6"
                  />
                </svg>
                <div>
                  <p class="text-sm font-semibold text-blue-800 mb-1">
                    Add your organization logo
                  </p>
                  <p class="text-sm text-blue-700 mb-3">
                    Upload a logo to embed it in your QR codes for professional
                    branding.
                  </p>

                  <!-- Error message -->
                  {#if uploadError}
                    <div
                      class="mb-3 p-2 bg-red-50 border border-red-200 rounded text-sm text-red-700"
                    >
                      {uploadError}
                    </div>
                  {/if}

                  <!-- File upload -->
                  <div class="flex items-center gap-2">
                    <input
                      type="file"
                      id="qr-logo-upload"
                      accept="image/png,image/jpeg,image/webp,image/svg+xml"
                      onchange={handleLogoUpload}
                      disabled={isUploading}
                      class="hidden"
                    />
                    <label
                      for="qr-logo-upload"
                      class="inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-blue-700 bg-blue-100 rounded-md hover:bg-blue-200 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      {#if isUploading}
                        <svg
                          class="w-4 h-4 animate-spin"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                          />
                        </svg>
                        Uploading...
                      {:else}
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
                            d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
                          />
                        </svg>
                        Choose file
                      {/if}
                    </label>

                    {#if !isUploading}
                      <span class="text-xs text-blue-600"
                        >PNG, JPEG, WebP, SVG (max 500KB)</span
                      >
                    {/if}
                  </div>

                  <!-- Success message -->
                  {#if uploadedLogoUrl}
                    <div
                      class="mt-3 p-2 bg-green-50 border border-green-200 rounded text-sm text-green-700"
                    >
                      ✅ Logo uploaded! You can now enable it in the QR code.
                    </div>
                  {/if}
                </div>
              </div>
            </div>
          {:else}
            <!-- Free user - upgrade teaser for logo feature -->
            <div
              class="mb-5 p-4 bg-orange-50 border border-orange-200 rounded-lg"
            >
              <div class="flex items-start gap-3">
                <svg
                  class="w-5 h-5 text-orange-600 mt-0.5 flex-shrink-0"
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
                <div>
                  <p class="text-sm font-semibold text-orange-800 mb-1">
                    Pro feature: Organization logos
                  </p>
                  <p class="text-sm text-orange-700 mb-3">
                    Upgrade to Pro to add your organization logo to QR codes for
                    professional branding.
                  </p>
                  <a
                    href="/billing"
                    class="inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-orange-700 bg-orange-100 rounded-md hover:bg-orange-200 transition-colors"
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
                        d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6"
                      />
                    </svg>
                    Upgrade to Pro
                  </a>
                </div>
              </div>
            </div>
          {/if}

          <!-- Upgrade popover -->
          {#if lockedPopoverOpen}
            <div
              class="mb-4 p-4 bg-orange-50 border border-orange-200 rounded-lg"
            >
              <p class="text-sm font-semibold text-orange-800 mb-1">
                Pro feature
              </p>
              <p class="text-sm text-orange-700 mb-3">
                Larger sizes, SVG download, and custom logo require a Pro plan.
              </p>
              <div class="flex gap-2">
                <a
                  href="/billing"
                  class="flex-1 text-center px-3 py-2 bg-orange-500 text-white rounded-lg text-sm font-medium hover:bg-orange-600 transition-colors"
                >
                  Upgrade to Pro
                </a>
                <button
                  onclick={() => (lockedPopoverOpen = false)}
                  class="px-3 py-2 border border-orange-300 text-orange-700 rounded-lg text-sm font-medium hover:bg-orange-100 transition-colors"
                >
                  Dismiss
                </button>
              </div>
            </div>
          {/if}

          <!-- Actions -->
          <div class="flex gap-3">
            <button
              onclick={downloadPNG}
              class="flex-1 px-4 py-2.5 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-medium hover:from-orange-600 hover:to-orange-700 transition-all flex items-center justify-center gap-2"
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
                  d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                ></path>
              </svg>
              PNG
            </button>
            <button
              onclick={tryDownloadSVG}
              class="flex-1 px-4 py-2.5 border font-medium rounded-lg transition-colors flex items-center justify-center gap-2
								{isPro
                ? 'border-orange-400 text-orange-600 hover:bg-orange-50'
                : 'border-gray-300 text-gray-500 opacity-60'}"
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
                  d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                ></path>
              </svg>
              SVG
              {#if !isPro}
                🔒
              {/if}
            </button>
            <button
              onclick={onClose}
              class="px-4 py-2.5 border border-gray-300 text-gray-700 rounded-lg font-medium hover:bg-gray-50 transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
