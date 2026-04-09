<script lang="ts">
  import "../app.css";

  const { children } = $props();
</script>

<svelte:head>
  <link rel="icon" href="/favicon.svg" />
</svelte:head>

{@render children()}
