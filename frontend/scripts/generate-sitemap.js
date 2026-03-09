#!/usr/bin/env node

import { writeFileSync } from 'fs';
import { join } from 'path';

const PUBLIC_PAGES = [
	{ path: '/', changefreq: 'weekly', priority: '1.0' },
	{ path: '/pricing', changefreq: 'monthly', priority: '0.8' },
	{ path: '/login', changefreq: 'monthly', priority: '0.7' },
	{ path: '/report', changefreq: 'monthly', priority: '0.6' },
	{ path: '/terms', changefreq: 'yearly', priority: '0.3' },
	{ path: '/privacy', changefreq: 'yearly', priority: '0.3' }
];

function generateSitemap(baseUrl) {
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

// Get base URL from environment or default
// Can override with SITE_URL env var for different deployments
const baseUrl = process.env.SITE_URL || 'https://rushomon.cc';
const sitemap = generateSitemap(baseUrl);

// Write to build directory
const outputPath = join(process.cwd(), 'build', 'sitemap.xml');
writeFileSync(outputPath, sitemap);

console.log(`✅ Generated sitemap.xml at ${outputPath}`);
console.log(`🌐 Site URL: ${baseUrl}`);
