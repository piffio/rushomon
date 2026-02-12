<script lang="ts">
	import type { User } from '$lib/types/api';

	interface Props {
		user: User;
		size?: 'sm' | 'md' | 'lg';
		className?: string;
	}

	let { user, size = 'md', className = '' }: Props = $props();

	const sizeClasses = {
		sm: 'w-6 h-6',
		md: 'w-8 h-8',
		lg: 'w-12 h-12'
	};

	const textSizeClasses = {
		sm: 'text-xs',
		md: 'text-sm',
		lg: 'text-base'
	};

	const sizeClass = $derived(sizeClasses[size]);
	const textSizeClass = $derived(textSizeClasses[size]);
	const displayName = $derived(user.name || user.email);
	const initial = $derived(displayName.charAt(0).toUpperCase());
</script>

{#if user.avatar_url}
	<img
		src={user.avatar_url}
		alt={displayName}
		class="{sizeClass} rounded-full {className}"
	/>
{:else}
	<div
		class="{sizeClass} rounded-full bg-gray-300 flex items-center justify-center {className}"
	>
		<span class="text-gray-600 font-medium {textSizeClass}">
			{initial}
		</span>
	</div>
{/if}
