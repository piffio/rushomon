<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { authApi } from "$lib/api/auth";
  import { billingApi } from "$lib/api/billing";
  import Footer from "$lib/components/Footer.svelte";
  import Header from "$lib/components/Header.svelte";
  import type { TransferInfo, User } from "$lib/types/api";
  import { onMount } from "svelte";

  const token = $derived($page.params.token ?? "");

  let currentUser = $state<User | null>(null);
  let transferInfo = $state<TransferInfo | null>(null);
  let loading = $state(true);
  let accepting = $state(false);
  let error = $state<string | null>(null);
  let accepted = $state(false);

  const tierLabels: Record<string, string> = {
    free: "Free",
    pro: "Pro",
    business: "Business",
    unlimited: "Unlimited"
  };

  function formatDate(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString("en-US", {
      year: "numeric",
      month: "long",
      day: "numeric"
    });
  }

  async function acceptTransfer() {
    if (!currentUser) {
      // Redirect to login, then back here.
      goto(`/login?next=${encodeURIComponent($page.url.pathname)}`);
      return;
    }

    // Email cross-check: warn if the logged-in email doesn't match.
    if (
      transferInfo &&
      currentUser.email.toLowerCase() !== transferInfo.to_email.toLowerCase()
    ) {
      error = `This transfer was sent to ${transferInfo.to_email}. You are logged in as ${currentUser.email}. Please log in with the correct account.`;
      return;
    }

    accepting = true;
    error = null;
    try {
      await billingApi.acceptTransfer(token);
      accepted = true;
    } catch (e: unknown) {
      error =
        e && typeof e === "object" && "message" in e
          ? String((e as { message: unknown }).message)
          : "Failed to accept the transfer. Please try again.";
    } finally {
      accepting = false;
    }
  }

  onMount(async () => {
    try {
      // Fetch transfer info (public — no auth required).
      transferInfo = await billingApi.getTransferInfo(token);
    } catch (e: unknown) {
      error =
        e && typeof e === "object" && "message" in e
          ? String((e as { message: unknown }).message)
          : "Transfer not found or no longer valid.";
    }

    // Try to load the current user (optional — page is accessible when logged out).
    try {
      currentUser = await authApi.me();
    } catch {
      // Not logged in — that's fine, we'll show a login prompt.
    }

    loading = false;
  });
</script>

<svelte:head>
  <title>Accept Ownership Transfer - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50 flex flex-col">
  <Header user={currentUser ?? undefined} />

  <main class="flex-1 container mx-auto px-4 py-16 max-w-lg">
    {#if loading}
      <div class="flex items-center justify-center py-20">
        <div
          class="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-500"
        ></div>
      </div>
    {:else if accepted}
      <!-- Success state -->
      <div class="bg-white rounded-2xl border border-gray-200 p-8 text-center">
        <div
          class="w-16 h-16 rounded-full bg-green-100 flex items-center justify-center mx-auto mb-4"
        >
          <svg
            class="w-8 h-8 text-green-600"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M5 13l4 4L19 7"
            />
          </svg>
        </div>
        <h1 class="text-2xl font-bold text-gray-900 mb-2">
          Ownership Transferred!
        </h1>
        <p class="text-gray-500 mb-6">
          You are now the owner of the billing account. A confirmation email has
          been sent to both you and the previous owner.
        </p>
        <a
          href="/billing"
          class="inline-block px-6 py-3 bg-orange-500 text-white rounded-xl font-medium hover:bg-orange-600 transition-colors"
        >
          Go to Billing
        </a>
      </div>
    {:else if error && !transferInfo}
      <!-- Transfer not found / expired / already accepted -->
      <div class="bg-white rounded-2xl border border-gray-200 p-8 text-center">
        <div
          class="w-16 h-16 rounded-full bg-red-100 flex items-center justify-center mx-auto mb-4"
        >
          <svg
            class="w-8 h-8 text-red-500"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </div>
        <h1 class="text-2xl font-bold text-gray-900 mb-2">
          Transfer Not Available
        </h1>
        <p class="text-gray-500 mb-6">{error}</p>
        <a
          href="/billing"
          class="inline-block px-6 py-3 border border-gray-200 text-gray-700 rounded-xl font-medium hover:bg-gray-50 transition-colors"
        >
          Go to Billing
        </a>
      </div>
    {:else if transferInfo}
      <!-- Main acceptance card -->
      <div class="bg-white rounded-2xl border border-gray-200 p-8">
        <div
          class="w-16 h-16 rounded-full bg-orange-100 flex items-center justify-center mx-auto mb-4"
        >
          <svg
            class="w-8 h-8 text-orange-500"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"
            />
          </svg>
        </div>

        <h1 class="text-2xl font-bold text-gray-900 text-center mb-1">
          Billing Account Transfer
        </h1>
        <p class="text-gray-500 text-center text-sm mb-6">
          This offer expires on {formatDate(transferInfo.expires_at)}
        </p>

        <!-- Transfer details -->
        <div class="bg-gray-50 rounded-xl p-4 mb-6 space-y-3 text-sm">
          <div class="flex justify-between">
            <span class="text-gray-500">From</span>
            <span class="text-gray-900 font-medium">
              {transferInfo.from_user_name
                ? `${transferInfo.from_user_name} (${transferInfo.from_user_email})`
                : transferInfo.from_user_email}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">To</span>
            <span class="text-gray-900 font-medium"
              >{transferInfo.to_email}</span
            >
          </div>
          <div class="flex justify-between">
            <span class="text-gray-500">Plan</span>
            <span class="font-medium text-orange-600">
              {tierLabels[transferInfo.billing_account_tier] ??
                transferInfo.billing_account_tier}
            </span>
          </div>
        </div>

        <!-- What this means -->
        <div
          class="bg-amber-50 border border-amber-200 rounded-xl p-4 mb-6 text-sm text-amber-800"
        >
          <p class="font-medium mb-1">What happens when you accept:</p>
          <ul class="list-disc list-inside space-y-1 text-amber-700">
            <li>You become the <strong>billing account owner</strong></li>
            <li>You take responsibility for the subscription</li>
            <li>
              The current owner becomes an <strong>Admin</strong> in the organization(s)
            </li>
          </ul>
        </div>

        {#if !currentUser}
          <!-- Not logged in -->
          <div
            class="p-4 bg-blue-50 border border-blue-200 rounded-xl text-blue-800 text-sm mb-4"
          >
            You must be logged in with the account associated with
            <strong>{transferInfo.to_email}</strong> to accept this transfer.
          </div>
          <a
            href="/login?next={encodeURIComponent($page.url.pathname)}"
            class="block w-full text-center px-6 py-3 bg-orange-500 text-white rounded-xl font-medium hover:bg-orange-600 transition-colors"
          >
            Log in to Accept
          </a>
        {:else if currentUser.email.toLowerCase() !== transferInfo.to_email.toLowerCase()}
          <!-- Wrong account -->
          <div
            class="p-4 bg-red-50 border border-red-200 rounded-xl text-red-700 text-sm mb-4"
          >
            This transfer was sent to <strong>{transferInfo.to_email}</strong>.
            You are logged in as
            <strong>{currentUser.email}</strong>. Please log out and log in with
            the correct account.
          </div>
        {:else}
          <!-- Correct account — ready to accept -->
          {#if error}
            <div
              class="p-3 bg-red-50 border border-red-200 rounded-xl text-red-700 text-sm mb-4"
            >
              {error}
            </div>
          {/if}
          <button
            onclick={acceptTransfer}
            disabled={accepting}
            class="w-full px-6 py-3 bg-orange-500 text-white rounded-xl font-semibold hover:bg-orange-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {accepting ? "Accepting…" : "Accept Ownership Transfer"}
          </button>
          <p class="text-center text-xs text-gray-400 mt-3">
            You can decline by simply ignoring this page.
          </p>
        {/if}
      </div>
    {/if}
  </main>

  <Footer />
</div>
