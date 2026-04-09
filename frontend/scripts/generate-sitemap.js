#!/usr/bin/env node

import { writeFileSync } from "fs";
import { join } from "path";
import { fileURLToPath } from "url";
import { dirname, resolve } from "path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Import PUBLIC_PAGES from the shared location
// We need to read the TypeScript file and extract the constant
const fs = await import("fs");
const pagesFile = fs.readFileSync(
  resolve(__dirname, "../src/lib/seo/pages.ts"),
  "utf8"
);
const pagesMatch = pagesFile.match(
  /export const PUBLIC_PAGES: PublicPage\[\] = \[([\s\S]*?)\];/
);
const pagesContent = pagesMatch ? pagesMatch[1] : "";

// Parse the pages array (simplified - just extract path, changefreq, priority)
const PUBLIC_PAGES = [];
const pageMatches = pagesContent.matchAll(
  /\{ path: '([^']+)', changefreq: '([^']+)', priority: '([^']+)'/g
);
for (const match of pageMatches) {
  PUBLIC_PAGES.push({
    path: match[1],
    changefreq: match[2],
    priority: match[3]
  });
}

function generateSitemap(baseUrl) {
  const lastmod = new Date().toISOString().split("T")[0];

  const urls = PUBLIC_PAGES.map(
    ({ path, changefreq, priority }) => `
  <url>
    <loc>${baseUrl}${path}</loc>
    <lastmod>${lastmod}</lastmod>
    <changefreq>${changefreq}</changefreq>
    <priority>${priority}</priority>
  </url>`
  ).join("");

  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>`;
}

// Get base URL from environment or default
// Can override with PUBLIC_VITE_SITE_URL env var for different deployments
const baseUrl = process.env.PUBLIC_VITE_SITE_URL || "https://rushomon.cc";
const sitemap = generateSitemap(baseUrl);

// Write to build directory
const outputPath = join(process.cwd(), "build", "sitemap.xml");
writeFileSync(outputPath, sitemap);

console.log(`✅ Generated sitemap.xml at ${outputPath}`);
console.log(`🌐 Site URL: ${baseUrl}`);
