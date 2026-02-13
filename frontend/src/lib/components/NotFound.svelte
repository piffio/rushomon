<script lang="ts">
	import Logo from "./Logo.svelte";
	import Footer from "./Footer.svelte";
</script>

<svelte:head>
	<title>404 - Page Not Found | Rushomon</title>
</svelte:head>

<div class="min-h-screen bg-white flex flex-col">
	<header class="bg-white border-b border-gray-200">
		<div class="container mx-auto px-4 py-4">
			<div class="flex justify-between items-center">
				<Logo href="/" />
			</div>
		</div>
	</header>

	<main class="flex-1 flex items-center justify-center">
		<div class="text-center px-4 py-20">
			<h1
				class="text-8xl md:text-9xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-orange-500 to-orange-600 mb-6"
			>
				404
			</h1>
			<h2 class="text-2xl md:text-3xl font-semibold text-gray-900 mb-4">
				Page Not Found
			</h2>
			<p
				class="text-lg text-gray-600 mb-10 max-w-md mx-auto leading-relaxed"
			>
				The page you're looking for doesn't exist or the link may have
				been removed.
			</p>
			<a
				href="/"
				class="inline-flex items-center gap-2 bg-gradient-to-r from-orange-500 to-orange-600 text-white px-8 py-4 rounded-xl text-lg font-semibold hover:from-orange-600 hover:to-orange-700 transition-all shadow-lg hover:shadow-xl hover:scale-105 transform duration-200"
			>
				<svg
					class="w-5 h-5"
					fill="none"
					stroke="currentColor"
					viewBox="0 0 24 24"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"
					/>
				</svg>
				Go Home
			</a>
		</div>
	</main>

	<Footer />
</div>
