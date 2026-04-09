<script lang="ts">
  import { page } from "$app/state";
  import { browser } from "$app/environment";

  interface Props {
    title: string;
    description?: string;
    canonical?: string;
    ogType?: string;
    noindex?: boolean;
  }

  const props: Props = $props();

  const siteUrl = $derived(
    browser
      ? window.location.origin
      : import.meta.env.PUBLIC_VITE_SITE_URL || "https://rushomon.cc"
  );
  const canonicalUrl = $derived(props.canonical || page.url.href);
  const fullTitle = $derived(`${props.title} – Rushomon`);
</script>

<svelte:head>
  <title>{fullTitle}</title>
  {#if props.description}
    <meta name="description" content={props.description} />
  {/if}
  {#if props.noindex}
    <meta name="robots" content="noindex, follow" />
  {/if}

  <!-- Canonical URL -->
  <link rel="canonical" href={canonicalUrl} />

  <!-- Open Graph tags -->
  <meta property="og:type" content={props.ogType} />
  <meta property="og:site_name" content="Rushomon" />
  <meta property="og:title" content={fullTitle} />
  {#if props.description}
    <meta property="og:description" content={props.description} />
  {/if}
  <meta property="og:url" content={canonicalUrl} />
  <meta property="og:image" content={`${siteUrl}/favicon-192x192.png`} />
  <meta property="og:image:width" content="192" />
  <meta property="og:image:height" content="192" />

  <!-- Twitter/X Card tags -->
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:title" content={fullTitle} />
  {#if props.description}
    <meta name="twitter:description" content={props.description} />
  {/if}
  <meta name="twitter:image" content={`${siteUrl}/favicon-192x192.png`} />

  <!-- Sitemap reference for search engines -->
  <link rel="sitemap" type="application/xml" href={`${siteUrl}/sitemap.xml`} />
</svelte:head>
