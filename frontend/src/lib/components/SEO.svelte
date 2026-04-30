<script lang="ts">
  interface Props {
    title: string;
    description?: string;
    canonical?: string;
    ogType?: string;
    noindex?: boolean;
  }

  const props: Props = $props();

  // Use environment variable or fallback to production URL
  const siteUrl = import.meta.env.PUBLIC_VITE_SITE_URL || "https://rushomon.cc";
</script>

<svelte:head>
  <title>{props.title} – Rushomon</title>
  {#if props.description}
    <meta name="description" content={props.description} />
  {/if}
  {#if props.noindex}
    <meta name="robots" content="noindex, follow" />
  {/if}

  <!-- Canonical URL -->
  <link rel="canonical" href={props.canonical || siteUrl} />

  <!-- Open Graph tags -->
  <meta property="og:type" content={props.ogType} />
  <meta property="og:site_name" content="Rushomon" />
  <meta property="og:title" content={`${props.title} – Rushomon`} />
  {#if props.description}
    <meta property="og:description" content={props.description} />
  {/if}
  <meta property="og:url" content={props.canonical || siteUrl} />
  <meta property="og:image" content={`${siteUrl}/favicon-192x192.png`} />
  <meta property="og:image:width" content="192" />
  <meta property="og:image:height" content="192" />

  <!-- Twitter/X Card tags -->
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:title" content={`${props.title} – Rushomon`} />
  {#if props.description}
    <meta name="twitter:description" content={props.description} />
  {/if}
  <meta name="twitter:image" content={`${siteUrl}/favicon-192x192.png`} />
</svelte:head>
