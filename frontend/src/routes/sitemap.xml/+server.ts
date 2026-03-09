import type { RequestHandler } from './$types';
import { PUBLIC_VITE_API_BASE_URL } from '$env/static/public';

const PUBLIC_PAGES = [
	{ path: '/', changefreq: 'weekly', priority: '1.0' },
	{ path: '/pricing', changefreq: 'monthly', priority: '0.8' },
	{ path: '/login', changefreq: 'monthly', priority: '0.7' },
	{ path: '/report', changefreq: 'monthly', priority: '0.6' },
	{ path: '/terms', changefreq: 'yearly', priority: '0.3' },
	{ path: '/privacy', changefreq: 'yearly', priority: '0.3' }
];

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
	const origin = new URL(request.url).origin;

	const sitemap = buildSitemap(origin);

	return new Response(sitemap, {
		headers: {
			'Content-Type': 'application/xml',
			'Cache-Control': 'public, max-age=86400'
		}
	});
};
