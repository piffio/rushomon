<script lang="ts">
  import { page } from "$app/state";
  import type { User } from "$lib/types/api";
  import Avatar from "./Avatar.svelte";
  import { clickOutside } from "$lib/utils/clickOutside";
  import { PUBLIC_VITE_DOCS_URL } from "$env/static/public";

  const DOCS_URL = PUBLIC_VITE_DOCS_URL || "https://github.com/piffio/rushomon";

  interface Props {
    user: User;
    onLogout: () => Promise<void>;
    showBilling?: boolean;
  }

  const { user, onLogout, showBilling = false }: Props = $props();
  let showMenu = $state(false);
  let isLoggingOut = $state(false);

  const isOnDashboard = $derived(page.url.pathname === "/dashboard");

  function handleClickOutside() {
    showMenu = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && showMenu) {
      showMenu = false;
      e.preventDefault();
    }
  }

  async function handleLogout() {
    isLoggingOut = true;
    try {
      await onLogout();
    } catch (error) {
      console.error("Logout failed:", error);
      isLoggingOut = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="relative" use:clickOutside={handleClickOutside}>
  <!-- Trigger Button -->
  <button
    onclick={() => (showMenu = !showMenu)}
    aria-haspopup="true"
    aria-expanded={showMenu}
    aria-label="User menu"
    class="flex items-center gap-3 px-4 py-2 rounded-lg hover:bg-gray-50 transition-colors"
  >
    <Avatar {user} size="md" />
    <span class="text-sm font-medium text-gray-700 hidden md:block">
      {user.name || user.email}
    </span>
    <svg
      class="w-4 h-4 text-gray-500 transition-transform {showMenu
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

  <!-- Dropdown Menu -->
  {#if showMenu}
    <div
      role="menu"
      aria-orientation="vertical"
      class="absolute right-0 mt-2 w-56 bg-white rounded-lg shadow-lg border border-gray-200 py-1 z-50"
    >
      <!-- User Info Section -->
      <div class="px-4 py-3 border-b border-gray-100">
        <p class="text-sm font-medium text-gray-900">
          {user.name || "User"}
        </p>
        <p class="text-sm text-gray-500">{user.email}</p>
        <p class="text-xs text-gray-400 mt-1">
          {user.role === "admin" ? "Administrator" : "Member"}
        </p>
      </div>

      <!-- Navigation Links -->
      <a
        href="/settings"
        role="menuitem"
        class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
      >
        ⚙️ Account Settings
      </a>
      {#if showBilling}
        <a
          href="/billing"
          role="menuitem"
          class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
        >
          💳 Billing
        </a>
      {/if}
      <a
        href={DOCS_URL}
        target="_blank"
        rel="noopener noreferrer"
        role="menuitem"
        class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
      >
        📖 Documentation
      </a>

      {#if user.role === "admin"}
        <div class="border-t border-gray-100 my-1"></div>
        {#if isOnDashboard}
          <a
            href="/admin/dashboard"
            role="menuitem"
            class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
          >
            👥 Admin Dashboard
          </a>
        {:else}
          <a
            href="/dashboard"
            role="menuitem"
            class="block px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors"
          >
            📊 Dashboard
          </a>
        {/if}
      {/if}

      <!-- Logout Section -->
      <div class="border-t border-gray-100 mt-1 pt-1">
        <button
          onclick={handleLogout}
          disabled={isLoggingOut}
          role="menuitem"
          class="w-full text-left px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isLoggingOut ? "🔄 Logging out..." : "🚪 Log out"}
        </button>
      </div>
    </div>
  {/if}
</div>
