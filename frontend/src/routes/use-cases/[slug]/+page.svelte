<script lang="ts">
  import { authApi } from "$lib/api/auth";
  import Footer from "$lib/components/Footer.svelte";
  import Header from "$lib/components/Header.svelte";
  import SEO from "$lib/components/SEO.svelte";
  import type { User } from "$lib/types/api";
  import { onMount } from "svelte";
  import type { PageData } from "./$types";

  const { data }: { data: PageData } = $props();
  const useCase = $derived(data.useCase);

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
  <SEO title={useCase.metaTitle} description={useCase.metaDescription} />
</svelte:head>

<div class="min-h-screen bg-white flex flex-col">
  <Header user={currentUser} currentPage="landing" />

  <main class="flex-1">
    <!-- Hero -->
    <section class="bg-gradient-to-b from-orange-50 to-white py-20 md:py-28">
      <div class="container mx-auto px-4 max-w-4xl text-center">
        <h1
          class="text-4xl md:text-6xl font-bold text-gray-900 mb-6 leading-tight"
        >
          {useCase.heroHeading}
        </h1>
        <p
          class="text-xl text-gray-600 mb-10 max-w-2xl mx-auto leading-relaxed"
        >
          {useCase.heroSubheading}
        </p>
        <div class="flex flex-col sm:flex-row gap-4 justify-center">
          <a
            href="/login"
            class="bg-orange-500 hover:bg-orange-600 text-white px-8 py-3 rounded-xl font-semibold text-lg transition-colors shadow-lg shadow-orange-200"
          >
            Get Started Free
          </a>
          <a
            href="https://github.com/piffio/rushomon/blob/main/docs/SELF_HOSTING.md"
            target="_blank"
            rel="noopener noreferrer"
            class="border-2 border-orange-500 text-orange-700 px-8 py-3 rounded-xl font-semibold text-lg hover:bg-orange-50 transition-colors"
          >
            Self-Hosting Guide
          </a>
        </div>
      </div>
    </section>

    <!-- Intro -->
    <section class="py-16">
      <div class="container mx-auto px-4 max-w-3xl">
        <p class="text-lg text-gray-700 leading-relaxed text-center">
          {useCase.intro}
        </p>
      </div>
    </section>

    <!-- Features -->
    <section class="py-8 pb-20 bg-gray-50">
      <div class="container mx-auto px-4 max-w-5xl">
        <h2
          class="text-2xl md:text-3xl font-bold text-gray-900 text-center mb-12"
        >
          Why Rushomon?
        </h2>
        <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
          {#each useCase.features as feature (feature.title)}
            <div
              class="bg-white rounded-2xl p-6 border border-gray-200 shadow-sm"
            >
              <h3 class="text-lg font-semibold text-gray-900 mb-2">
                {feature.title}
              </h3>
              <p class="text-gray-600 leading-relaxed text-sm">
                {feature.body}
              </p>
            </div>
          {/each}
        </div>
      </div>
    </section>

    <!-- FAQ -->
    <section class="py-20">
      <div class="container mx-auto px-4 max-w-3xl">
        <h2
          class="text-2xl md:text-3xl font-bold text-gray-900 text-center mb-12"
        >
          Frequently Asked Questions
        </h2>
        <div class="space-y-6">
          {#each useCase.faqs as faq (faq.q)}
            <div class="border border-gray-200 rounded-2xl p-6">
              <h3 class="text-base font-semibold text-gray-900 mb-2">
                {faq.q}
              </h3>
              <p class="text-gray-600 leading-relaxed">{faq.a}</p>
            </div>
          {/each}
        </div>
      </div>
    </section>

    <!-- CTA -->
    <section class="py-16 bg-orange-50">
      <div class="container mx-auto px-4 max-w-2xl text-center">
        <h2 class="text-3xl md:text-4xl font-bold text-gray-900 mb-4">
          Ready to get started?
        </h2>
        <p class="text-lg text-gray-600 mb-8">
          Free tier available. No credit card required.
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
