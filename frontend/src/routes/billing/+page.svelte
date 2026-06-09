<script lang="ts">
  import { goto } from "$app/navigation";
  import { authApi } from "$lib/api/auth";
  import type {
    BillingAccountSummary,
    BillingAccountsResponse
  } from "$lib/api/billing";
  import { billingApi } from "$lib/api/billing";
  import Footer from "$lib/components/Footer.svelte";
  import Header from "$lib/components/Header.svelte";
  import type { User } from "$lib/types/api";
  import { onMount } from "svelte";

  let currentUser = $state<User | undefined>(undefined);
  let accounts = $state<BillingAccountSummary[]>([]);
  let loading = $state(true);
  let portalLoading = $state(false);
  let error = $state<string | null>(null);

  // ── Ownership transfer state ──────────────────────────────────────────────
  // The BA the transfer modal is operating on (always the primary / highest-tier one).
  let transferBaId = $state<string | null>(null);
  let showTransferModal = $state(false);
  let transferEmail = $state("");
  let transferLoading = $state(false);
  let transferError = $state<string | null>(null);
  let transferSuccess = $state<{
    baId: string;
    token: string;
    toEmail: string;
    expiresAt: number;
  } | null>(null);
  let cancelTransferLoading = $state(false);

  const tierLabels: Record<string, string> = {
    free: "Free",
    pro: "Pro",
    business: "Business"
  };

  const tierColors: Record<string, string> = {
    free: "bg-gray-100 text-gray-600",
    pro: "bg-blue-100 text-blue-700",
    business: "bg-amber-100 text-amber-700"
  };

  const statusConfig: Record<string, { label: string; color: string }> = {
    active: {
      label: "Active",
      color: "text-green-700 bg-green-50 border-green-200"
    },
    trialing: {
      label: "Trialing",
      color: "text-blue-700 bg-blue-50 border-blue-200"
    },
    past_due: {
      label: "Past Due",
      color: "text-red-700 bg-red-50 border-red-200"
    },
    canceled: {
      label: "Canceled",
      color: "text-gray-600 bg-gray-50 border-gray-200"
    },
    unpaid: { label: "Unpaid", color: "text-red-700 bg-red-50 border-red-200" }
  };

  function formatDate(ts: number | null): string {
    if (!ts) return "—";
    return new Date(ts * 1000).toLocaleDateString("en-US", {
      year: "numeric",
      month: "long",
      day: "numeric"
    });
  }

  function formatPrice(cents: number, currency: string): string {
    const amount = cents / 100;
    const symbol = currency?.toLowerCase() === "eur" ? "€" : "$";
    return `${symbol}${amount.toFixed(2)}`;
  }

  /** The primary BA is the first one returned (already sorted highest-tier first). */
  const primaryAccount = $derived(accounts[0] ?? null);
  /** All subsequent BAs are secondary. */
  const secondaryAccounts = $derived(accounts.slice(1));

  async function initiateTransfer() {
    if (!transferBaId) return;
    transferError = null;
    transferLoading = true;
    try {
      const res = await billingApi.initiateTransfer(
        transferEmail.trim().toLowerCase(),
        transferBaId
      );
      transferSuccess = {
        baId: transferBaId,
        token: res.token,
        toEmail: res.to_email,
        expiresAt: res.expires_at
      };
      showTransferModal = false;
      transferEmail = "";
    } catch (e: unknown) {
      transferError =
        e && typeof e === "object" && "message" in e
          ? String((e as { message: unknown }).message)
          : "Failed to initiate transfer. Please try again.";
    } finally {
      transferLoading = false;
    }
  }

  async function cancelTransfer() {
    cancelTransferLoading = true;
    try {
      await billingApi.cancelTransfer();
      transferSuccess = null;
    } catch (e: unknown) {
      error =
        e && typeof e === "object" && "message" in e
          ? String((e as { message: unknown }).message)
          : "Failed to cancel transfer.";
    } finally {
      cancelTransferLoading = false;
    }
  }

  async function openPortal() {
    portalLoading = true;
    error = null;
    try {
      const { url } = await billingApi.createPortal();
      window.open(url, "_blank");
    } catch (e: unknown) {
      error =
        e && typeof e === "object" && "message" in e
          ? String((e as { message: unknown }).message)
          : "Failed to open billing portal. Please try again.";
      portalLoading = false;
    }
  }

  onMount(async () => {
    try {
      const [user, accountsRes] = await Promise.all([
        authApi.me(),
        billingApi.getAccounts()
      ]);
      currentUser = user;
      accounts = (accountsRes as BillingAccountsResponse).accounts ?? [];
    } catch {
      goto("/login");
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>Billing & Subscription - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50 flex flex-col">
  <Header user={currentUser} currentPage="dashboard" />

  <main class="flex-1 container mx-auto px-4 py-12 max-w-3xl">
    <div class="mb-8">
      <h1 class="text-3xl font-bold text-gray-900 mb-1">
        Billing & Subscription
      </h1>
      <p class="text-gray-500">
        Manage your plan, payment details, and the organizations covered by your
        billing account{accounts.length > 1 ? "s" : ""}.
      </p>
    </div>

    {#if loading}
      <div class="flex items-center justify-center py-20">
        <div
          class="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-500"
        ></div>
      </div>
    {:else if primaryAccount}
      <!-- ── Primary billing account ─────────────────────────────────────── -->
      {@render billingAccountCard(primaryAccount, true)}

      <!-- ── Secondary billing accounts (read-only) ─────────────────────── -->
      {#each secondaryAccounts as ba (ba.id)}
        <div class="mt-2 mb-1">
          <p
            class="text-xs font-semibold text-gray-400 uppercase tracking-wider px-1 mb-2"
          >
            Also owned
          </p>
        </div>
        {@render billingAccountCard(ba, false)}
      {/each}

      {#if error}
        <div
          class="p-4 bg-red-50 border border-red-200 rounded-xl text-red-700 text-sm mt-4"
        >
          {error}
        </div>
      {/if}
    {:else if !loading}
      <div
        class="bg-white rounded-2xl border border-gray-200 p-8 text-center text-gray-500"
      >
        No billing account found.
        <a
          href="/pricing"
          class="ml-1 text-orange-600 font-medium hover:underline"
        >
          View pricing
        </a>
      </div>
    {/if}
  </main>

  <!-- Transfer ownership modal -->
  {#if showTransferModal}
    <div
      class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
    >
      <div class="bg-white rounded-2xl p-6 max-w-md w-full shadow-xl">
        <h2 class="text-lg font-semibold text-gray-900 mb-2">
          Transfer Billing Account Ownership
        </h2>
        <p class="text-sm text-gray-500 mb-4">
          Enter the email address of an existing member of one of the
          organizations on this billing account. They will receive a
          confirmation email and must accept within <strong>7 days</strong>.
          <br /><br />
          <span class="text-amber-700 font-medium"
            >You will become an Admin</span
          > in the associated organization(s) after the transfer completes.
        </p>

        <label
          for="transfer-email"
          class="block text-sm font-medium text-gray-700 mb-1"
        >
          Recipient email
        </label>
        <input
          id="transfer-email"
          type="email"
          bind:value={transferEmail}
          placeholder="member@example.com"
          class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-orange-500 mb-4"
        />

        {#if transferError}
          <div
            class="p-3 bg-red-50 border border-red-200 rounded-lg text-red-700 text-sm mb-4"
          >
            {transferError}
          </div>
        {/if}

        <div class="flex gap-3">
          <button
            onclick={() => {
              showTransferModal = false;
            }}
            class="flex-1 px-4 py-2 border border-gray-200 text-gray-700 rounded-lg text-sm font-medium hover:bg-gray-50 transition-colors"
          >
            Cancel
          </button>
          <button
            onclick={initiateTransfer}
            disabled={transferLoading || !transferEmail.trim()}
            class="flex-1 px-4 py-2 bg-orange-500 text-white rounded-lg text-sm font-medium hover:bg-orange-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {transferLoading ? "Sending…" : "Send Transfer Request"}
          </button>
        </div>
      </div>
    </div>
  {/if}

  <Footer />
</div>

<!-- ── Reusable billing account card snippet ─────────────────────────────── -->
{#snippet billingAccountCard(ba: BillingAccountSummary, isPrimary: boolean)}
  {@const isPaid = ba.tier?.toLowerCase() !== "free"}
  {@const isActive = ba.subscription_status !== "canceled"}
  {@const isPending = transferSuccess?.baId === ba.id}

  <!-- Plan header card -->
  <div class="bg-white rounded-2xl border border-gray-200 p-6 mb-5">
    <div class="flex items-start justify-between gap-4">
      <div>
        <p
          class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-1"
        >
          {isPrimary ? "Current Plan" : "Plan"}
        </p>
        <div class="flex items-center gap-3">
          <p class="text-2xl font-bold text-gray-900">
            {tierLabels[ba.tier?.toLowerCase()] ?? ba.tier}
          </p>
          {#if ba.subscription_status}
            {@const s = statusConfig[ba.subscription_status]}
            <span
              class="px-2.5 py-0.5 rounded-full text-xs font-semibold border {s?.color ??
                'text-gray-600 bg-gray-50 border-gray-200'}"
            >
              {s?.label ?? ba.subscription_status}
            </span>
          {/if}
        </div>

        {#if ba.amount_cents && ba.interval}
          <p class="mt-1 text-sm text-gray-500">
            {formatPrice(ba.amount_cents, ba.currency ?? "eur")} / {ba.interval}
          </p>
        {/if}

        {#if ba.current_period_end}
          <p
            class="mt-1 text-sm {ba.cancel_at_period_end
              ? 'text-red-600 font-medium'
              : 'text-gray-500'}"
          >
            {ba.cancel_at_period_end ? "Cancels on" : "Renews on"}
            {formatDate(ba.current_period_end)}
          </p>
        {/if}

        {#if ba.subscription_status === "canceled"}
          <div class="mt-3 p-3 bg-amber-50 border border-amber-200 rounded-lg">
            <p class="text-sm text-amber-800">
              Your subscription has been canceled. Your account has been moved
              to the Free tier. You can resubscribe at any time from the
              <a href="/pricing" class="font-medium underline">pricing page</a>.
            </p>
          </div>
        {/if}
      </div>

      <!-- Primary plan actions in header -->
      {#if isPrimary}
        <div class="flex-shrink-0">
          {#if isPaid && isActive}
            <button
              onclick={openPortal}
              disabled={portalLoading}
              class="px-4 py-2 bg-orange-500 text-white rounded-lg font-medium hover:bg-orange-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
            >
              {portalLoading ? "Loading…" : "Manage Subscription"}
            </button>
          {:else if !isPaid}
            <a
              href="/pricing"
              class="px-4 py-2 bg-orange-500 text-white rounded-lg font-medium hover:bg-orange-600 transition-colors whitespace-nowrap"
            >
              Upgrade Plan
            </a>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- Organizations covered by this BA -->
  {#if ba.organizations.length > 0}
    <div class="bg-white rounded-2xl border border-gray-200 p-6 mb-5">
      <p
        class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-4"
      >
        Organizations on this plan
      </p>
      <ul class="divide-y divide-gray-100">
        {#each ba.organizations as org (org.id)}
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
                <p class="text-sm font-medium text-gray-900">{org.name}</p>
                <p class="text-xs text-gray-400">
                  {org.member_count}
                  {org.member_count === 1 ? "member" : "members"} · {org.link_count}
                  link{org.link_count === 1 ? "" : "s"}
                </p>
              </div>
            </div>
            <div class="flex items-center gap-3">
              <span
                class="text-xs px-2 py-0.5 rounded-full font-medium capitalize {tierColors[
                  ba.tier?.toLowerCase()
                ] ?? 'bg-gray-100 text-gray-600'}"
              >
                {tierLabels[ba.tier?.toLowerCase()] ?? ba.tier}
              </span>
              <a
                href="/dashboard/org"
                class="text-xs text-orange-600 hover:text-orange-700 font-medium transition-colors"
              >
                Settings →
              </a>
            </div>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <!-- Subscription management actions — primary paid plans only -->
  {#if isPrimary && isPaid && isActive}
    <div class="bg-white rounded-2xl border border-gray-200 p-6 mb-5">
      <p
        class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-4"
      >
        Actions
      </p>
      <div class="space-y-2">
        <button
          onclick={openPortal}
          disabled={portalLoading}
          class="w-full text-left px-4 py-3 rounded-lg border border-gray-200 hover:bg-gray-50 transition-colors disabled:opacity-50"
        >
          <p class="font-medium text-gray-900 text-sm">Update payment method</p>
          <p class="text-xs text-gray-500 mt-0.5">
            Change your card or billing details
          </p>
        </button>
        <button
          onclick={openPortal}
          disabled={portalLoading}
          class="w-full text-left px-4 py-3 rounded-lg border border-gray-200 hover:bg-gray-50 transition-colors disabled:opacity-50"
        >
          <p class="font-medium text-gray-900 text-sm">View invoices</p>
          <p class="text-xs text-gray-500 mt-0.5">
            Download receipts and past invoices
          </p>
        </button>
        <button
          onclick={openPortal}
          disabled={portalLoading}
          class="w-full text-left px-4 py-3 rounded-lg border border-red-100 hover:bg-red-50 transition-colors disabled:opacity-50"
        >
          <p class="font-medium text-red-600 text-sm">Cancel subscription</p>
          <p class="text-xs text-gray-500 mt-0.5">
            Your plan stays active until end of billing period
          </p>
        </button>
      </div>
    </div>
  {/if}

  <!-- Ownership Transfer — only on primary BA, business tier only -->
  {#if isPrimary && ba.tier?.toLowerCase() === "business"}
    <div class="bg-white rounded-2xl border border-gray-200 p-6 mb-5">
      <p
        class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-4"
      >
        Ownership Transfer
      </p>

      {#if isPending}
        <!-- Pending transfer status -->
        <div class="p-4 bg-amber-50 border border-amber-200 rounded-xl mb-4">
          <p class="text-sm font-medium text-amber-800 mb-1">
            Transfer pending
          </p>
          <p class="text-sm text-amber-700">
            An ownership transfer to <strong>{transferSuccess!.toEmail}</strong>
            is waiting for acceptance. The offer expires on
            <strong>{formatDate(transferSuccess!.expiresAt)}</strong>.
          </p>
        </div>
        <button
          onclick={cancelTransfer}
          disabled={cancelTransferLoading}
          class="px-4 py-2 border border-red-200 text-red-600 rounded-lg text-sm font-medium hover:bg-red-50 transition-colors disabled:opacity-50"
        >
          {cancelTransferLoading ? "Cancelling…" : "Cancel Transfer"}
        </button>
      {:else}
        <p class="text-sm text-gray-500 mb-4">
          Transfer this billing account to another member of one of its
          organizations. They will receive an email and must accept within 7
          days. You will become an Admin in the associated organization(s).
        </p>
        <button
          onclick={() => {
            transferBaId = ba.id;
            showTransferModal = true;
            transferError = null;
            transferEmail = "";
          }}
          class="px-4 py-2 border border-gray-200 text-gray-700 rounded-lg text-sm font-medium hover:bg-gray-50 transition-colors"
        >
          Transfer Ownership…
        </button>
      {/if}
    </div>
  {/if}
{/snippet}
