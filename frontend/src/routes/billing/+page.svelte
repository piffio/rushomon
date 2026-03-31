<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import Header from "$lib/components/Header.svelte";
  import Footer from "$lib/components/Footer.svelte";
  import { authApi } from "$lib/api/auth";
  import { billingApi } from "$lib/api/billing";
  import { orgsApi } from "$lib/api/orgs";
  import type { User, OrgWithRole } from "$lib/types/api";
  import type { BillingStatus } from "$lib/api/billing";

  let currentUser = $state<User | undefined>(undefined);
  let billingStatus = $state<BillingStatus | null>(null);
  let orgs = $state<OrgWithRole[]>([]);
  let loading = $state(true);
  let portalLoading = $state(false);
  let error = $state<string | null>(null);

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
    unpaid: {
      label: "Unpaid",
      color: "text-red-700 bg-red-50 border-red-200"
    }
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
      const [user, status, orgsRes] = await Promise.all([
        authApi.me(),
        billingApi.getStatus(),
        orgsApi.listMyOrgs()
      ]);
      currentUser = user;
      billingStatus = status;
      orgs = orgsRes.orgs;
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
        Manage your plan, payment details, and the organizations covered by this
        billing account.
      </p>
    </div>

    {#if loading}
      <div class="flex items-center justify-center py-20">
        <div
          class="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-500"
        ></div>
      </div>
    {:else if billingStatus}
      <!-- Current Plan Card -->
      <div class="bg-white rounded-2xl border border-gray-200 p-6 mb-5">
        <div class="flex items-start justify-between gap-4">
          <div>
            <p
              class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-1"
            >
              Current Plan
            </p>
            <div class="flex items-center gap-3">
              <p class="text-2xl font-bold text-gray-900">
                {tierLabels[billingStatus.tier?.toLowerCase()] ??
                  billingStatus.tier}
              </p>
              {#if billingStatus.subscription_status}
                {@const s = statusConfig[billingStatus.subscription_status]}
                <span
                  class="px-2.5 py-0.5 rounded-full text-xs font-semibold border {s?.color ??
                    'text-gray-600 bg-gray-50 border-gray-200'}"
                >
                  {s?.label ?? billingStatus.subscription_status}
                </span>
              {/if}
            </div>

            {#if billingStatus.amount_cents && billingStatus.interval}
              <p class="mt-1 text-sm text-gray-500">
                {formatPrice(
                  billingStatus.amount_cents,
                  billingStatus.currency ?? "eur"
                )}
                / {billingStatus.interval}
                {#if billingStatus.discount_name}
                  <span class="ml-2 text-green-600 font-medium"
                    >{billingStatus.discount_name}</span
                  >
                {/if}
              </p>
            {/if}

            {#if billingStatus.current_period_end}
              <p
                class="mt-1 text-sm {billingStatus.cancel_at_period_end
                  ? 'text-red-600 font-medium'
                  : 'text-gray-500'}"
              >
                {billingStatus.cancel_at_period_end
                  ? "Cancels on"
                  : "Renews on"}
                {formatDate(billingStatus.current_period_end)}
              </p>
            {/if}

            {#if billingStatus.subscription_status === "canceled"}
              <div
                class="mt-3 p-3 bg-amber-50 border border-amber-200 rounded-lg"
              >
                <p class="text-sm text-amber-800">
                  Your subscription has been canceled. Your account has been
                  moved to the Free tier. You can resubscribe at any time from
                  the <a href="/pricing" class="font-medium underline"
                    >pricing page</a
                  >.
                </p>
              </div>
            {/if}
          </div>

          <div class="flex-shrink-0">
            {#if billingStatus.tier?.toLowerCase() !== "free" && billingStatus.subscription_status !== "canceled"}
              <button
                onclick={openPortal}
                disabled={portalLoading}
                class="px-4 py-2 bg-orange-500 text-white rounded-lg font-medium hover:bg-orange-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
              >
                {portalLoading ? "Loading…" : "Manage Subscription"}
              </button>
            {:else if billingStatus.tier?.toLowerCase() === "free"}
              <a
                href="/pricing"
                class="px-4 py-2 bg-orange-500 text-white rounded-lg font-medium hover:bg-orange-600 transition-colors whitespace-nowrap"
              >
                Upgrade Plan
              </a>
            {/if}
          </div>
        </div>
      </div>

      <!-- Organizations on this billing account -->
      {#if orgs.length > 0}
        <div class="bg-white rounded-2xl border border-gray-200 p-6 mb-5">
          <p
            class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-4"
          >
            Organizations on this plan
          </p>
          <ul class="divide-y divide-gray-100">
            {#each orgs as org}
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
                    <p class="text-xs text-gray-400 capitalize">
                      {org.role}
                    </p>
                  </div>
                </div>
                <div class="flex items-center gap-3">
                  <span
                    class="text-xs px-2 py-0.5 rounded-full font-medium capitalize {tierColors[
                      org.tier
                    ] ?? 'bg-gray-100 text-gray-600'}"
                  >
                    {org.tier}
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

      <!-- Actions (paid plans only) -->
      {#if billingStatus.tier?.toLowerCase() !== "free" && billingStatus.subscription_status !== "canceled"}
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
              <p class="font-medium text-gray-900 text-sm">
                Update payment method
              </p>
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
              <p class="font-medium text-red-600 text-sm">
                Cancel subscription
              </p>
              <p class="text-xs text-gray-500 mt-0.5">
                Your plan stays active until end of billing period
              </p>
            </button>
          </div>
        </div>
      {/if}

      {#if error}
        <div
          class="p-4 bg-red-50 border border-red-200 rounded-xl text-red-700 text-sm"
        >
          {error}
        </div>
      {/if}
    {/if}
  </main>

  <Footer />
</div>
