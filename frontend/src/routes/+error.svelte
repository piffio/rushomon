<script lang="ts">
	import NotFound from "$lib/components/NotFound.svelte";
	import { page } from "$app/stores";
</script>

{#if $page.status === 404}
	<NotFound />
{:else}
	<div class="min-h-screen bg-white flex items-center justify-center">
		<div class="text-center px-4">
			<h1 class="text-6xl font-bold text-gray-900 mb-4">
				{$page.status}
			</h1>
			<p class="text-lg text-gray-600 mb-8">
				{$page.error?.message || "Something went wrong"}
			</p>
			<a
				href="/"
				class="inline-flex items-center gap-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white px-8 py-4 rounded-xl text-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-lg hover:shadow-xl"
			>
				Go Home
			</a>
		</div>
	</div>
{/if}
