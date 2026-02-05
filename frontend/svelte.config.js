import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapter({
			pages: 'build',      // Output directory
			assets: 'build',     // Assets directory
			fallback: 'index.html',  // SPA fallback
			precompress: true    // Gzip/brotli compression
		})
	}
};

export default config;
