<script lang="ts">
  import Header from "$lib/components/Header.svelte";
  import { page } from "$app/state";
  import type { LayoutData } from "./$types";

  interface Props {
    data: LayoutData;
    children: any;
  }

  let { data, children }: Props = $props();

  const tabs = [
    { label: "My Links", href: "/dashboard", id: "links" },
    { label: "Analytics", href: "/dashboard/analytics", id: "analytics" },
    { label: "Settings", href: "/dashboard/org", id: "settings" }
  ];

  function isActive(tab: { href: string; id: string }) {
    const path = page.url.pathname;
    if (tab.id === "links") {
      return path === "/dashboard" || path.startsWith("/dashboard/links");
    }
    if (tab.id === "analytics") {
      return path.startsWith("/dashboard/analytics");
    }
    if (tab.id === "settings") {
      return path.startsWith("/dashboard/org");
    }
    return false;
  }

  let currentHeaderPage = $derived(
    page.url.pathname.startsWith("/dashboard/analytics")
      ? ("analytics" as const)
      : ("dashboard" as const)
  );
</script>

{#if data.user}
  <Header user={data.user} currentPage={currentHeaderPage} />

  <!-- Sub-nav tabs -->
  <div class="border-b border-gray-200 bg-white">
    <div class="max-w-6xl mx-auto px-6">
      <nav class="flex gap-0 -mb-px" aria-label="Dashboard navigation">
        {#each tabs as tab}
          <a
            href={tab.href}
            class="relative px-4 py-3 text-sm font-medium transition-colors whitespace-nowrap
							{isActive(tab)
              ? 'text-orange-600 border-b-2 border-orange-500'
              : 'text-gray-500 hover:text-gray-700 border-b-2 border-transparent'}"
            aria-current={isActive(tab) ? "page" : undefined}
          >
            {tab.label}
          </a>
        {/each}
      </nav>
    </div>
  </div>
{/if}

{@render children()}
