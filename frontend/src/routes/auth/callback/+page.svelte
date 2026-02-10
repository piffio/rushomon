<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";

	onMount(() => {
		goto("/dashboard");
	});
</script>

<div class="flex min-h-screen items-center justify-center">
	<div class="text-center">
		<div class="loading loading-spinner loading-lg text-primary"></div>
		<p class="mt-4 text-gray-600">Completing sign in...</p>
	</div>
</div>
