<script lang="ts">
	import type { HTMLButtonAttributes } from "svelte/elements";

	type Variant = "primary" | "secondary" | "danger" | "ghost";
	type Size = "sm" | "md" | "lg";

	let {
		loading = false,
		disabled = false,
		variant = "primary" as Variant,
		size = "md" as Size,
		fullWidth = false,
		children,
		...restProps
	} = $props();
</script>

<button
	type="button"
	class="
		inline-flex items-center justify-center gap-2 font-medium rounded-lg
		transition-all duration-150
		disabled:cursor-not-allowed
		{fullWidth ? 'w-full' : ''}
		{size === 'sm' ? 'px-3 py-1.5 text-sm' : size === 'lg' ? 'px-6 py-3 text-base' : 'px-4 py-2 text-sm'}
		{variant === 'primary'
			? 'bg-orange-500 hover:bg-orange-600 disabled:bg-orange-300 text-white'
			: variant === 'secondary'
				? 'bg-gray-100 hover:bg-gray-200 disabled:bg-gray-50 text-gray-800 border border-gray-300'
				: variant === 'danger'
					? 'bg-red-500 hover:bg-red-600 disabled:bg-red-300 text-white'
					: 'bg-transparent hover:bg-gray-100 disabled:bg-transparent text-gray-600'}
		{loading ? 'opacity-70' : ''}
	"
	disabled={disabled || loading}
	{...restProps}
>
	{#if loading}
		<svg
			class="animate-spin"
			class:h-4={size !== 'lg'}
			class:w-4={size !== 'lg'}
			class:h-5={size === 'lg'}
			class:w-5={size === 'lg'}
			viewBox="0 0 24 24"
			fill="none"
			aria-hidden="true"
		>
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
	{/if}
	<span class={loading ? 'opacity-75' : ''}>
		{@render children?.()}
	</span>
</button>
