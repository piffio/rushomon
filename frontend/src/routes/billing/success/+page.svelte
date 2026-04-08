<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import Header from "$lib/components/Header.svelte";
  import Footer from "$lib/components/Footer.svelte";
  import { authApi } from "$lib/api/auth";
  import type { User } from "$lib/types/api";

  let currentUser = $state<User | undefined>(undefined);
  let countdown = $state(5);

  onMount(() => {
    authApi
      .me()
      .then((user) => {
        currentUser = user;
      })
      .catch(() => {
        // Not critical for this page
      });

    const interval = setInterval(() => {
      countdown -= 1;
      if (countdown <= 0) {
        clearInterval(interval);
        goto("/dashboard");
      }
    }, 1000);

    return () => clearInterval(interval);
  });
</script>

<svelte:head>
  <title>Subscription Confirmed - Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-gray-50 flex flex-col">
  <Header user={currentUser} currentPage="dashboard" />

  <main class="flex-1 flex items-center justify-center px-4 py-20">
    <div
      class="max-w-md w-full bg-white rounded-2xl border border-gray-200 p-10 text-center"
    >
      <div
        class="w-16 h-16 bg-green-100 rounded-full flex items-center justify-center mx-auto mb-6"
      >
        <svg
          class="w-8 h-8 text-green-600"
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
      </div>

      <h1 class="text-2xl font-bold text-gray-900 mb-3">You're all set!</h1>
      <p class="text-gray-600 mb-2">
        Your subscription has been activated. Welcome to the next level of
        Rushomon.
      </p>
      <p class="text-sm text-gray-400 mb-8">
        It may take a moment for your new limits to reflect in the dashboard.
      </p>

      <a
        href="/dashboard"
        class="block w-full px-6 py-3 bg-gradient-to-r from-orange-500 to-orange-600 text-white rounded-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-sm text-center"
      >
        Go to Dashboard
      </a>

      <p class="mt-4 text-xs text-gray-400">
        Redirecting automatically in {countdown}s…
      </p>
    </div>
  </main>

  <Footer />
</div>
