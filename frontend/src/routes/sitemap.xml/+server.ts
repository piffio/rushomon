import type { RequestHandler } from './$types';
import { dev } from '$app/environment';
import { PUBLIC_PAGES } from '$lib/seo/pages';

function buildSitemap(baseUrl: string): string {
	const lastmod = new Date().toISOString().split('T')[0];

	const urls = PUBLIC_PAGES.map(
		({ path, changefreq, priority }) => `
  <url>
    <loc>${baseUrl}${path}</loc>
    <lastmod>${lastmod}</lastmod>
    <changefreq>${changefreq}</changefreq>
    <priority>${priority}</priority>
  </url>`
	).join('');

	return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>`;
}

export const GET: RequestHandler = async ({ request }) => {
	// Only serve in development mode - in production sitemap is generated as static file
	if (!dev) {
		return new Response('Not found', { status: 404 });
	}

	const origin = new URL(request.url).origin;
	const sitemap = buildSitemap(origin);

	return new Response(sitemap, {
		headers: {
			'Content-Type': 'application/xml',
			'Cache-Control': 'no-cache'
		}
	});
};
