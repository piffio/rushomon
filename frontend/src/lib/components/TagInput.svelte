<script lang="ts">
	import type { TagWithCount } from "$lib/types/api";

	let {
		tags = $bindable<string[]>([]),
		availableTags = [],
		placeholder = "Add tags...",
		disabled = false,
	}: {
		tags?: string[];
		availableTags?: TagWithCount[];
		placeholder?: string;
		disabled?: boolean;
	} = $props();

	let inputValue = $state("");
	let showDropdown = $state(false);
	let inputEl = $state<HTMLInputElement | undefined>(undefined);

	const MAX_TAGS = 20;
	const MAX_TAG_LENGTH = 50;

	const filteredSuggestions = $derived(
		availableTags
			.filter(
				(t) =>
					t.name.toLowerCase().includes(inputValue.toLowerCase()) &&
					!tags.includes(t.name),
			)
			.slice(0, 8),
	);

	function normalizeTag(raw: string): string {
		return raw.replace(/\s+/g, " ").trim();
	}

	function addTag(raw: string) {
		const tag = normalizeTag(raw);
		if (
			!tag ||
			tag.length > MAX_TAG_LENGTH ||
			tags.includes(tag) ||
			tags.length >= MAX_TAGS
		) {
			inputValue = "";
			showDropdown = false;
			return;
		}
		tags = [...tags, tag];
		inputValue = "";
		showDropdown = false;
	}

	function removeTag(tag: string) {
		tags = tags.filter((t) => t !== tag);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === "Enter" || e.key === ",") {
			e.preventDefault();
			if (inputValue.trim()) {
				addTag(inputValue);
			}
		} else if (e.key === "Backspace" && !inputValue && tags.length > 0) {
			removeTag(tags[tags.length - 1]);
		} else if (e.key === "Escape") {
			showDropdown = false;
		}
	}

	function handleInput() {
		showDropdown =
			inputValue.trim().length > 0 || filteredSuggestions.length > 0;
	}

	function handleFocus() {
		if (availableTags.length > 0) {
			showDropdown = true;
		}
	}

	function handleBlur() {
		// Delay to allow click on suggestion
		setTimeout(() => {
			showDropdown = false;
			if (inputValue.trim()) {
				addTag(inputValue);
			}
		}, 150);
	}

	// Deterministic color from tag name
	const TAG_COLORS = [
		"bg-blue-100 text-blue-800",
		"bg-green-100 text-green-800",
		"bg-purple-100 text-purple-800",
		"bg-yellow-100 text-yellow-800",
		"bg-pink-100 text-pink-800",
		"bg-indigo-100 text-indigo-800",
		"bg-orange-100 text-orange-800",
		"bg-teal-100 text-teal-800",
	];

	function tagColor(tag: string): string {
		let hash = 0;
		for (let i = 0; i < tag.length; i++) {
			hash = (hash * 31 + tag.charCodeAt(i)) & 0xffffffff;
		}
		return TAG_COLORS[Math.abs(hash) % TAG_COLORS.length];
	}
</script>

<div class="relative">
	<!-- Tag pills + input -->
	<div
		class="flex flex-wrap gap-1.5 min-h-[42px] w-full px-3 py-2 border-2 border-gray-200 rounded-xl focus-within:border-orange-500 transition-colors bg-white cursor-text"
		onclick={() => inputEl?.focus()}
		onkeydown={(e) => e.key === "Enter" && inputEl?.focus()}
		role="group"
		aria-label="Tags"
		tabindex="-1"
	>
		{#each tags as tag (tag)}
			<span
				class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium {tagColor(
					tag,
				)}"
			>
				{tag}
				{#if !disabled}
					<button
						type="button"
						onclick={() => removeTag(tag)}
						class="hover:opacity-70 transition-opacity leading-none"
						aria-label="Remove tag {tag}"
					>
						<svg
							class="w-3 h-3"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2.5"
								d="M6 18L18 6M6 6l12 12"
							/>
						</svg>
					</button>
				{/if}
			</span>
		{/each}

		{#if !disabled && tags.length < MAX_TAGS}
			<input
				bind:this={inputEl}
				bind:value={inputValue}
				type="text"
				{placeholder}
				class="flex-1 min-w-[120px] outline-none text-sm bg-transparent placeholder-gray-400"
				maxlength={MAX_TAG_LENGTH}
				onkeydown={handleKeydown}
				oninput={handleInput}
				onfocus={handleFocus}
				onblur={handleBlur}
				aria-label="Add tag"
				autocomplete="off"
			/>
		{/if}
	</div>

	<!-- Autocomplete dropdown -->
	{#if showDropdown && filteredSuggestions.length > 0}
		<div
			class="absolute z-20 top-full left-0 right-0 mt-1 bg-white border-2 border-gray-200 rounded-xl shadow-lg overflow-hidden"
		>
			{#each filteredSuggestions as suggestion (suggestion.name)}
				<button
					type="button"
					class="w-full flex items-center justify-between px-3 py-2 text-sm hover:bg-orange-50 hover:text-orange-700 transition-colors text-left"
					onmousedown={() => addTag(suggestion.name)}
				>
					<span class="flex items-center gap-2">
						<span
							class="inline-block w-2 h-2 rounded-full {tagColor(
								suggestion.name,
							).split(' ')[0]}"
						></span>
						{suggestion.name}
					</span>
					<span class="text-xs text-gray-400">{suggestion.count}</span
					>
				</button>
			{/each}
		</div>
	{/if}

	{#if tags.length > 0}
		<p class="mt-1 text-xs text-gray-400">
			{tags.length}/{MAX_TAGS} tags · Press Enter or comma to add
		</p>
	{:else}
		<p class="mt-1 text-xs text-gray-400">
			Press Enter or comma to add a tag
		</p>
	{/if}
</div>
