<script lang="ts">
	import { page } from '$app/stores';
	import { browser } from '$app/environment';

	interface Props {
		title: string;
		description?: string;
		canonical?: string;
		ogType?: string;
		noindex?: boolean;
	}

	let {
		title,
		description = '',
		canonical,
		ogType = 'website',
		noindex = false
	}: Props = $props();

	const siteUrl = $derived(browser 
		? window.location.origin 
		: import.meta.env.PUBLIC_VITE_SITE_URL || 'https://rushomon.cc'
	);
	const canonicalUrl = $derived(canonical || $page.url.href);
	const fullTitle = $derived(`${title} – Rushomon`);
</script>

<svelte:head>
	<title>{fullTitle}</title>
	{#if description}
		<meta name="description" content={description} />
	{/if}
	{#if noindex}
		<meta name="robots" content="noindex, follow" />
	{/if}
	
	<!-- Canonical URL -->
	<link rel="canonical" href={canonicalUrl} />
	
	<!-- Open Graph tags -->
	<meta property="og:type" content={ogType} />
	<meta property="og:site_name" content="Rushomon" />
	<meta property="og:title" content={fullTitle} />
	{#if description}
		<meta property="og:description" content={description} />
	{/if}
	<meta property="og:url" content={canonicalUrl} />
	<meta property="og:image" content={`${siteUrl}/favicon-192x192.png`} />
	<meta property="og:image:width" content="192" />
	<meta property="og:image:height" content="192" />
	
	<!-- Twitter/X Card tags -->
	<meta name="twitter:card" content="summary" />
	<meta name="twitter:title" content={fullTitle} />
	{#if description}
		<meta name="twitter:description" content={description} />
	{/if}
	<meta name="twitter:image" content={`${siteUrl}/favicon-192x192.png`} />
</svelte:head>
