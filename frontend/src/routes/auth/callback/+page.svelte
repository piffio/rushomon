<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import type { PageData } from './$types';

	export let data: PageData;

	onMount(() => {
		const { token } = data;

		if (!token) {
			goto('/?error=missing_token');
			return;
		}

		// Set cookie client-side (not httpOnly, but works with static deployment)
		// Cookie attributes match server-side version for consistency
		const isSecure = window.location.protocol === 'https:';
		const maxAge = 604800; // 7 days (matches backend SESSION_TTL_SECONDS)

		// Set cookie with secure attributes
		document.cookie = `rushomon_session=${token}; path=/; max-age=${maxAge}; samesite=lax${isSecure ? '; secure' : ''}`;

		// Redirect to dashboard
		goto('/dashboard');
	});
</script>

<div class="flex min-h-screen items-center justify-center">
	<div class="text-center">
		<div class="loading loading-spinner loading-lg text-primary"></div>
		<p class="mt-4 text-gray-600">Completing sign in...</p>
	</div>
</div>
