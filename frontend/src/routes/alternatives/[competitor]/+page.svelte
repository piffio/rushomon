<script lang="ts">
  import { authApi } from "$lib/api/auth";
  import Footer from "$lib/components/Footer.svelte";
  import Header from "$lib/components/Header.svelte";
  import SEO from "$lib/components/SEO.svelte";
  import type { User } from "$lib/types/api";
  import { onMount } from "svelte";
  import type { PageData } from "./$types";

  const { data }: { data: PageData } = $props();
  const competitor = $derived(data.competitor);

  let currentUser = $state<User | undefined>(undefined);

  onMount(async () => {
    try {
      const user = await authApi.me();
      currentUser = user;
    } catch {
      currentUser = undefined;
    }
  });
</script>

<svelte:head>
  <SEO title={competitor.metaTitle} description={competitor.metaDescription} />
</svelte:head>

<div class="min-h-screen bg-white flex flex-col">
  <Header user={currentUser} currentPage="landing" />

  <main class="flex-1">
    <!-- Hero -->
    <section class="bg-gradient-to-b from-orange-50 to-white py-20 md:py-28">
      <div class="container mx-auto px-4 max-w-4xl text-center">
        <div
          class="inline-flex items-center gap-2 bg-orange-100 text-orange-700 text-sm font-medium px-4 py-1.5 rounded-full mb-6"
        >
          Rushomon vs {competitor.name}
        </div>
        <h1
          class="text-4xl md:text-6xl font-bold text-gray-900 mb-6 leading-tight"
        >
          {competitor.heroHeading}
        </h1>
        <p
          class="text-xl text-gray-600 mb-10 max-w-2xl mx-auto leading-relaxed"
        >
          {competitor.heroSubheading}
        </p>
        <div class="flex flex-col sm:flex-row gap-4 justify-center">
          <a
            href="/login"
            class="bg-orange-500 hover:bg-orange-600 text-white px-8 py-3 rounded-xl font-semibold text-lg transition-colors shadow-lg shadow-orange-200"
          >
            Get Started Free
          </a>
          <a
            href="/pricing"
            class="border-2 border-orange-500 text-orange-700 px-8 py-3 rounded-xl font-semibold text-lg hover:bg-orange-50 transition-colors"
          >
            View Pricing
          </a>
        </div>
      </div>
    </section>

    <!-- Main pitch -->
    <section class="py-16">
      <div class="container mx-auto px-4 max-w-3xl">
        <p class="text-lg text-gray-700 leading-relaxed text-center">
          {competitor.mainPitch}
        </p>
      </div>
    </section>

    <!-- Comparison table -->
    <section class="py-8 pb-20">
      <div class="container mx-auto px-4 max-w-3xl">
        <h2
          class="text-2xl md:text-3xl font-bold text-gray-900 text-center mb-10"
        >
          Rushomon vs {competitor.name} — Feature Comparison
        </h2>
        <div
          class="overflow-x-auto rounded-2xl border border-gray-200 shadow-sm"
        >
          <table class="w-full text-left">
            <thead>
              <tr class="bg-gray-50 border-b border-gray-200">
                <th class="px-6 py-4 text-sm font-semibold text-gray-600 w-1/2"
                  >Feature</th
                >
                <th class="px-6 py-4 text-sm font-semibold text-orange-600"
                  >Rushomon</th
                >
                <th class="px-6 py-4 text-sm font-semibold text-gray-500"
                  >{competitor.name}</th
                >
              </tr>
            </thead>
            <tbody>
              {#each competitor.features as row, i (row.feature)}
                <tr class={i % 2 === 0 ? "bg-white" : "bg-gray-50"}>
                  <td class="px-6 py-4 text-sm font-medium text-gray-700"
                    >{row.feature}</td
                  >
                  <td class="px-6 py-4 text-sm text-gray-900">
                    {#if row.rushomon === true}
                      <span class="text-green-600 font-semibold">✓ Yes</span>
                    {:else if row.rushomon === false || row.rushomon === "No"}
                      <span class="text-red-500">✗ No</span>
                    {:else}
                      <span class="text-green-700">{row.rushomon}</span>
                    {/if}
                  </td>
                  <td class="px-6 py-4 text-sm text-gray-600">
                    {#if row.competitor === true}
                      <span class="text-green-600">✓ Yes</span>
                    {:else if row.competitor === false || row.competitor === "No"}
                      <span class="text-red-500">✗ No</span>
                    {:else}
                      {row.competitor}
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    </section>

    <!-- Why Rushomon -->
    <section class="py-16 bg-gray-50">
      <div class="container mx-auto px-4 max-w-4xl">
        <h2
          class="text-2xl md:text-3xl font-bold text-gray-900 text-center mb-12"
        >
          Why Choose Rushomon?
        </h2>
        <div class="grid md:grid-cols-3 gap-8">
          {#each competitor.whyRushomon as item (item.title)}
            <div
              class="bg-white rounded-2xl p-6 border border-gray-200 shadow-sm"
            >
              <h3 class="text-lg font-semibold text-gray-900 mb-3">
                {item.title}
              </h3>
              <p class="text-gray-600 leading-relaxed">{item.body}</p>
            </div>
          {/each}
        </div>
      </div>
    </section>

    <!-- CTA -->
    <section class="py-20">
      <div class="container mx-auto px-4 max-w-2xl text-center">
        <h2 class="text-3xl md:text-4xl font-bold text-gray-900 mb-4">
          Ready to switch from {competitor.name}?
        </h2>

        <p class="text-lg text-gray-600 mb-8">
          Start free, no credit card required. Import your existing links or
          start fresh.
        </p>
        <a
          href="/login"
          class="inline-block bg-orange-500 hover:bg-orange-600 text-white px-10 py-4 rounded-xl font-semibold text-lg transition-colors shadow-lg shadow-orange-200"
        >
          Get Started Free
        </a>
      </div>
    </section>
  </main>

  <Footer />
</div>
