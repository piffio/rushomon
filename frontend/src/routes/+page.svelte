<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import Footer from "$lib/components/Footer.svelte";
  import Header from "$lib/components/Header.svelte";
  import SEO from "$lib/components/SEO.svelte";
  import { onMount } from "svelte";
  import type { PageData } from "./$types";

  const { data }: { data: PageData } = $props();

  let signupsDisabled = $state(false);
  let navigating = $state(false);

  onMount(() => {
    signupsDisabled = page.url.searchParams.get("error") === "signups_disabled";
  });

  function handleNavigation() {
    if (data.user) {
      navigating = true;
      goto("/dashboard");
    }
  }
</script>

<svelte:head>
  <SEO
    title="Rushomon — URL Shortener with Analytics"
    description="URL shortener with powerful analytics. Free tier available, open source, self-hostable."
  />
</svelte:head>

<div class="min-h-screen bg-white flex flex-col">
  <Header user={data.user} currentPage="landing" />

  <!-- Hero Section -->
  <main class="flex-1">
    <section class="container mx-auto px-4 py-20 md:py-32">
      <div class="max-w-4xl mx-auto text-center">
        <div class="hero-fade-in">
          <h1
            class="text-5xl md:text-7xl font-bold text-gray-900 mb-8 leading-tight"
          >
            Short Links with<br />
            <span
              class="text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-orange-600"
            >
              Powerful Analytics
            </span>
          </h1>
          <p
            class="text-xl md:text-2xl text-gray-600 mb-12 leading-relaxed max-w-2xl mx-auto"
          >
            The URL shortener that grows with you. Start free, no setup
            required.
          </p>

          <!-- Signups Disabled Error -->
          {#if signupsDisabled}
            <div
              class="bg-red-50 border border-red-200 text-red-700 px-6 py-4 rounded-xl mb-8 max-w-xl mx-auto"
            >
              <p class="font-medium">New signups are currently disabled</p>
              <p class="text-sm mt-1 text-red-600">
                Contact the administrator if you need access to this instance.
              </p>
            </div>
          {/if}

          <!-- CTA Button -->
          <a
            href={data.user ? "/dashboard" : "/login"}
            onclick={data.user ? handleNavigation : undefined}
            class="inline-flex items-center gap-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white px-10 py-5 rounded-xl text-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-lg hover:shadow-xl hover:scale-105 transform duration-200 {navigating
              ? 'opacity-70 cursor-not-allowed'
              : ''}"
          >
            {#if navigating}
              <svg class="animate-spin w-6 h-6" fill="none" viewBox="0 0 24 24">
                <circle
                  class="opacity-25"
                  cx="12"
                  cy="12"
                  r="10"
                  stroke="currentColor"
                  stroke-width="4"
                />
                <path
                  class="opacity-75"
                  fill="currentColor"
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                />
              </svg>
            {:else}
              {data.user ? "Go to Dashboard" : "Create Your First Link — Free"}
              <svg
                class="w-6 h-6"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M13 7l5 5m0 0l-5 5m5-5H6"
                />
              </svg>
            {/if}
          </a>
        </div>
      </div>
    </section>

    <!-- How It Works Section -->
    <section class="bg-gradient-to-b from-gray-50 to-white py-20">
      <div class="container mx-auto px-4">
        <div class="max-w-6xl mx-auto">
          <h3
            class="text-3xl md:text-4xl font-bold text-gray-900 text-center mb-6"
          >
            Get Started in Seconds
          </h3>
          <p class="text-lg text-gray-600 text-center mb-12 max-w-2xl mx-auto">
            No complex setup. Just sign up and start creating links.
          </p>

          <div class="grid md:grid-cols-3 gap-8">
            <!-- Step 1: Sign Up -->
            <div class="text-center">
              <div class="relative mb-6">
                <div
                  class="absolute -top-3 -left-3 w-10 h-10 bg-orange-500 text-white rounded-full flex items-center justify-center font-bold text-lg"
                >
                  1
                </div>
                <img
                  src="/step-1-signup.webp"
                  alt="Sign up with GitHub or Google OAuth"
                  width="1324"
                  height="746"
                  loading="eager"
                  class="rounded-xl aspect-video object-cover shadow-lg border border-gray-200"
                />
              </div>
              <h4 class="text-xl font-semibold text-gray-900 mb-2">
                Sign Up Free
              </h4>
              <p class="text-gray-600">
                Connect with your GitHub or Google account. No credit card
                required.
              </p>
            </div>

            <!-- Step 2: Create Links -->
            <div class="text-center">
              <div class="relative mb-6">
                <div
                  class="absolute -top-3 -left-3 w-10 h-10 bg-orange-500 text-white rounded-full flex items-center justify-center font-bold text-lg"
                >
                  2
                </div>
                <img
                  src="/step-2-create.webp"
                  alt="Dashboard with link creation form"
                  width="1324"
                  height="746"
                  loading="lazy"
                  class="rounded-xl aspect-video object-cover shadow-lg border border-gray-200"
                />
              </div>
              <h4 class="text-xl font-semibold text-gray-900 mb-2">
                Create Your Links
              </h4>
              <p class="text-gray-600">
                Paste any URL, customize your short code, and share instantly.
              </p>
            </div>

            <!-- Step 3: Track Analytics -->
            <div class="text-center">
              <div class="relative mb-6">
                <div
                  class="absolute -top-3 -left-3 w-10 h-10 bg-orange-500 text-white rounded-full flex items-center justify-center font-bold text-lg"
                >
                  3
                </div>
                <img
                  src="/step-3-analytics.webp"
                  alt="Analytics dashboard showing link performance"
                  width="1324"
                  height="746"
                  loading="lazy"
                  class="rounded-xl aspect-video object-cover shadow-lg border border-gray-200"
                />
              </div>
              <h4 class="text-xl font-semibold text-gray-900 mb-2">
                Track Analytics
              </h4>
              <p class="text-gray-600">
                See clicks, referrers, and geographic data in real-time.
              </p>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Features Section -->
    <section id="features" class="py-20 bg-white">
      <div class="container mx-auto px-4">
        <div class="max-w-6xl mx-auto">
          <h2
            class="text-3xl md:text-4xl font-bold text-gray-900 text-center mb-16"
          >
            Everything You Need
          </h2>

          <div class="grid md:grid-cols-3 gap-8">
            <!-- Lightning Fast -->
            <div
              class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
            >
              <div
                class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
              >
                <svg
                  class="w-7 h-7 text-white"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M13 10V3L4 14h7v7l9-11h-7z"
                  />
                </svg>
              </div>
              <h4 class="text-xl font-semibold text-gray-900 mb-3">
                Lightning Fast
              </h4>
              <p class="text-gray-600 leading-relaxed">
                Powered by Cloudflare's edge network for sub-millisecond
                redirects worldwide.
              </p>
            </div>

            <!-- Analytics -->
            <div
              class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
            >
              <div
                class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
              >
                <svg
                  class="w-7 h-7 text-white"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
                  />
                </svg>
              </div>
              <h4 class="text-xl font-semibold text-gray-900 mb-3">
                Analytics
              </h4>
              <p class="text-gray-600 leading-relaxed">
                Track clicks, referrers, and geographic data to understand your
                link performance.
              </p>
            </div>

            <!-- Custom Codes -->
            <div
              class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
            >
              <div
                class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
              >
                <svg
                  class="w-7 h-7 text-white"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
                  />
                </svg>
              </div>
              <h4 class="text-xl font-semibold text-gray-900 mb-3">
                Custom Codes
              </h4>
              <p class="text-gray-600 leading-relaxed">
                Choose your own memorable short codes or let the system generate
                them automatically.
              </p>
            </div>

            <!-- Team Collaboration -->
            <div
              class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
            >
              <div
                class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
              >
                <svg
                  class="w-7 h-7 text-white"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"
                  />
                </svg>
              </div>
              <h4 class="text-xl font-semibold text-gray-900 mb-3">
                Team Collaboration
              </h4>
              <p class="text-gray-600 leading-relaxed">
                Organization support built-in. Perfect for teams and personal
                use alike.
              </p>
            </div>

            <!-- Open Source -->
            <div
              class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
            >
              <div
                class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
              >
                <svg
                  class="w-7 h-7 text-white"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
                  />
                </svg>
              </div>
              <h4 class="text-xl font-semibold text-gray-900 mb-3">
                Open Source
              </h4>
              <p class="text-gray-600 leading-relaxed">
                Built with Rust and SvelteKit. Free and open source under
                AGPL-3.0 license.
              </p>
            </div>

            <!-- Self-Hosted -->
            <div
              class="group p-8 rounded-2xl border-2 border-gray-200 hover:border-orange-500 transition-all duration-300 hover:shadow-lg bg-white"
            >
              <div
                class="w-14 h-14 bg-gradient-to-br from-orange-500 to-orange-600 rounded-xl flex items-center justify-center mb-5"
              >
                <svg
                  class="w-7 h-7 text-white"
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
              </div>
              <h4 class="text-xl font-semibold text-gray-900 mb-3">
                Self-Hostable
              </h4>
              <p class="text-gray-600 leading-relaxed">
                Full control over your data. Deploy on your own domain with
                Cloudflare Workers.
              </p>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Pricing Preview Section -->
    <section class="py-20 bg-gray-50">
      <div class="container mx-auto px-4">
        <div class="max-w-6xl mx-auto">
          <h2
            class="text-3xl md:text-4xl font-bold text-gray-900 text-center mb-6"
          >
            Choose What Works for You
          </h2>
          <p class="text-xl text-gray-600 text-center mb-12 max-w-2xl mx-auto">
            Start free and upgrade when you need more.
          </p>

          <div class="grid md:grid-cols-3 gap-8">
            <!-- Free Tier -->
            <div
              class="bg-white rounded-2xl border-2 border-orange-500 p-8 shadow-lg"
            >
              <div class="text-center mb-6">
                <h3 class="text-2xl font-bold text-gray-900 mb-2">Free</h3>
                <p class="text-gray-600">Perfect for personal use</p>
              </div>
              <ul class="space-y-3 mb-8 text-gray-600">
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  15 links/month
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  7-day analytics history
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Basic click tracking
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Custom short codes
                </li>
              </ul>
              <a
                href="/login"
                class="block w-full text-center bg-gradient-to-r from-orange-500 to-orange-600 text-white px-6 py-3 rounded-xl font-semibold hover:from-orange-600 hover:to-orange-700 transition-all"
              >
                Get Started
              </a>
            </div>

            <!-- Pro Tier -->
            <div
              class="bg-white rounded-2xl border-2 border-gray-200 p-8 hover:border-orange-300 transition-all"
            >
              <div class="text-center mb-6">
                <h3 class="text-2xl font-bold text-gray-900 mb-2">Pro</h3>
                <p class="text-gray-600">For creators & professionals</p>
              </div>
              <ul class="space-y-3 mb-8 text-gray-600">
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Unlimited links
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  365-day analytics history
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Advanced analytics
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Custom domains
                </li>
              </ul>
              <a
                href="/pricing"
                class="block w-full text-center border-2 border-orange-500 text-orange-600 px-6 py-3 rounded-xl font-semibold hover:bg-orange-50 transition-all"
              >
                View Pricing
              </a>
            </div>

            <!-- Business Tier -->
            <div
              class="bg-white rounded-2xl border-2 border-gray-200 p-8 hover:border-orange-300 transition-all"
            >
              <div class="text-center mb-6">
                <h3 class="text-2xl font-bold text-gray-900 mb-2">Business</h3>
                <p class="text-gray-600">For teams & organizations</p>
              </div>
              <ul class="space-y-3 mb-8 text-gray-600">
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Unlimited links
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Unlimited analytics history
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Team management
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  Priority support
                </li>
                <li class="flex items-center gap-3">
                  <svg
                    class="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                  SSO & API access
                </li>
              </ul>
              <a
                href="/pricing"
                class="block w-full text-center border-2 border-orange-500 text-orange-600 px-6 py-3 rounded-xl font-semibold hover:bg-orange-50 transition-all"
              >
                View Pricing
              </a>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Self-Hosting Section (For Developers) -->
    <section class="py-20 bg-white">
      <div class="container mx-auto px-4">
        <div class="max-w-4xl mx-auto text-center">
          <div
            class="w-16 h-16 bg-gradient-to-br from-orange-500 to-orange-600 rounded-2xl flex items-center justify-center mb-6 mx-auto"
          >
            <svg
              class="w-8 h-8 text-white"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"
              />
            </svg>
          </div>
          <h2 class="text-3xl md:text-4xl font-bold text-gray-900 mb-4">
            For Developers & Privacy-Focused Teams
          </h2>
          <p class="text-xl text-gray-600 mb-10">
            Full control over your data and infrastructure. Deploy on your own
            domain with Cloudflare Workers.
          </p>

          <div class="grid md:grid-cols-3 gap-8 mb-10">
            <div class="text-center">
              <div
                class="w-12 h-12 bg-orange-100 rounded-xl flex items-center justify-center mb-3 mx-auto"
              >
                <svg
                  class="w-6 h-6 text-orange-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                  />
                </svg>
              </div>
              <h4 class="font-semibold text-gray-900 mb-1">Open Source</h4>
              <p class="text-gray-600 text-sm">
                AGPL-3.0 license. Audit the code.
              </p>
            </div>
            <div class="text-center">
              <div
                class="w-12 h-12 bg-orange-100 rounded-xl flex items-center justify-center mb-3 mx-auto"
              >
                <svg
                  class="w-6 h-6 text-orange-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                  />
                </svg>
              </div>
              <h4 class="font-semibold text-gray-900 mb-1">Self-Hostable</h4>
              <p class="text-gray-600 text-sm">Deploy on Cloudflare Workers.</p>
            </div>
            <div class="text-center">
              <div
                class="w-12 h-12 bg-orange-100 rounded-xl flex items-center justify-center mb-3 mx-auto"
              >
                <svg
                  class="w-6 h-6 text-orange-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"
                  />
                </svg>
              </div>
              <h4 class="font-semibold text-gray-900 mb-1">Your Domain</h4>
              <p class="text-gray-600 text-sm">Use your own custom domain.</p>
            </div>
          </div>

          <a
            href="https://github.com/piffio/rushomon/blob/main/docs/SELF_HOSTING.md"
            target="_blank"
            rel="noopener noreferrer"
            class="inline-flex items-center gap-2 text-orange-600 font-semibold hover:text-orange-700 transition-colors"
          >
            View Self-Hosting Guide
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
                d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
              />
            </svg>
          </a>
        </div>
      </div>
    </section>
  </main>

  <Footer />
</div>
