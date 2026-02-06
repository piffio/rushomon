<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import type { PageData } from './$types';
	import { setAccessToken } from '$lib/api/client';

	export let data: PageData;

	onMount(() => {
		const { token } = data;

		if (!token) {
			goto('/?error=missing_token');
			return;
		}

		// Store access token in localStorage (for cross-domain API calls)
		setAccessToken(token);

		// For backward compatibility with local development, also set session cookie
		// This ensures existing code that relies on cookies continues to work
		const isSecure = window.location.protocol === 'https:';
		const maxAge = 3600; // 1 hour (matches access token expiry)
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
