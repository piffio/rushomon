#!/usr/bin/env node
/**
 * configure-versions.mjs
 *
 * Run during production docs deployment to register the newly released
 * version in the Docusaurus OpenAPI plugin config.
 *
 * Usage: node scripts/configure-versions.mjs v0.7.0
 *
 * What it does:
 *   1. Scans docs/openapi/ for all vX.Y.Z.json files
 *   2. Updates docs-site/versions.json with the full list of released versions
 *
 * The docusaurus.config.ts reads versions.json at build time to add
 * version dropdown entries. Each versioned spec is served from
 * docs/openapi/<version>.json.
 */

import {readFileSync, writeFileSync, existsSync, readdirSync} from 'fs';
import {resolve, dirname} from 'path';
import {fileURLToPath} from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SITE_ROOT = resolve(__dirname, '..');
const REPO_ROOT = resolve(SITE_ROOT, '..');
const OPENAPI_DIR = resolve(REPO_ROOT, 'docs', 'openapi');
const VERSIONS_FILE = resolve(SITE_ROOT, 'versions.json');

const newTag = process.argv[2];
if (!newTag) {
  console.error('Usage: configure-versions.mjs <tag>  (e.g. v0.7.0)');
  process.exit(1);
}

// Collect all vX.Y.Z.json files from docs/openapi/
const files = readdirSync(OPENAPI_DIR).filter(f => /^v\d+\.\d+\.\d+\.json$/.test(f));
const versions = files
  .map(f => f.replace('.json', ''))
  .sort((a, b) => {
    const [aMaj, aMin, aPatch] = a.replace('v', '').split('.').map(Number);
    const [bMaj, bMin, bPatch] = b.replace('v', '').split('.').map(Number);
    if (bMaj !== aMaj) return bMaj - aMaj;
    if (bMin !== aMin) return bMin - aMin;
    return bPatch - aPatch;
  });

console.log(`📋 Found ${versions.length} versioned spec(s): ${versions.join(', ')}`);

writeFileSync(VERSIONS_FILE, JSON.stringify(versions, null, 2) + '\n');
console.log(`✅ Wrote ${VERSIONS_FILE}`);
