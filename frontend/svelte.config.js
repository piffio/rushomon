import adapterStatic from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		adapter: adapterStatic({
			pages: 'build',
			assets: 'build',
			fallback: 'index.html',
			precompress: true
		}),
		prerender: {
			origin: process.env.PUBLIC_VITE_SITE_URL || 'https://rushomon.cc'
		}
	}
};

export default config;
