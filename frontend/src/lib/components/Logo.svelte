<script lang="ts">
	interface Props {
		href: string;
		className?: string;
	}

	let { href, className = '' }: Props = $props();
</script>

<a {href} class="flex items-center gap-2 {className}" aria-label="Rushomon Home">
	<div
		class="w-8 h-8 bg-gradient-to-br from-orange-500 to-orange-600 rounded-lg flex items-center justify-center"
	>
		<span class="text-white font-bold text-sm">R</span>
	</div>
	<span class="text-2xl font-bold text-gray-900">Rushomon</span>
</a>
