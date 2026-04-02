import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import type * as OpenApiPlugin from 'docusaurus-plugin-openapi-docs';
import {existsSync, readFileSync} from 'fs';
import {resolve} from 'path';

const DOCS_SITE_URL = process.env.DOCS_SITE_URL || 'https://doc.rushomon.cc';

// Check for stable spec files (committed versioned specs like v0.6.2.json)
// This enables the stable version in the dropdown when any vX.Y.Z.json spec exists
const OPENAPI_DIR = resolve(__dirname, '../docs/openapi');
const stableSpecFiles = existsSync(OPENAPI_DIR)
  ? require('fs').readdirSync(OPENAPI_DIR).filter((f: string) => /^v\d+\.\d+\.\d+\.json$/.test(f))
  : [];
const latestStableVersion = stableSpecFiles.length > 0
  ? stableSpecFiles.sort().reverse()[0].replace('.json', '')
  : null;

const config: Config = {
  title: 'Rushomon API Docs',
  tagline: 'API reference for the Rushomon URL shortener',
  favicon: 'img/favicon.svg',

  future: {
    v4: true,
  },

  url: DOCS_SITE_URL,
  baseUrl: '/',

  organizationName: 'piffio',
  projectName: 'rushomon',

  onBrokenLinks: 'warn',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          docItemComponent: '@theme/ApiItem',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  plugins: [
    require.resolve('./plugins/webpack-node-fallback'),
    [
      'docusaurus-plugin-openapi-docs',
      {
        id: 'rushomon-api',
        docsPluginId: 'classic',
        config: {
          rushomon: {
            specPath: '../docs/openapi/main.json',
            outputDir: 'docs/api',
            showSchemas: true,
            sidebarOptions: {
              groupPathsBy: 'tag',
              categoryLinkSource: 'tag',
            },
          } satisfies OpenApiPlugin.Options,
          ...(latestStableVersion ? {
            rushomonStable: {
              specPath: `../docs/openapi/${latestStableVersion}.json`,
              outputDir: 'docs/api-stable',
              showSchemas: true,
              label: latestStableVersion,
              sidebarOptions: {
                groupPathsBy: 'tag',
                categoryLinkSource: 'tag',
              },
            } satisfies OpenApiPlugin.Options,
          } : {}),
        },
      },
    ],
  ],

  themes: ['docusaurus-theme-openapi-docs'],

  themeConfig: {
    image: 'img/rushomon-social-card.png',
    colorMode: {
      defaultMode: 'light',
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Rushomon Docs',
      logo: {
        alt: 'Rushomon Logo',
        src: 'img/logo.svg',
        href: '/',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'defaultSidebar',
          position: 'left',
          label: 'API Reference',
        },
        ...(latestStableVersion ? [{
          type: 'dropdown' as const,
          label: 'Version',
          position: 'left' as const,
          items: [
            {
              label: 'main (unreleased)',
              to: '/docs/api/rushomon-url-shortener-api',
            },
            {
              label: `${latestStableVersion} (stable)`,
              to: '/docs/api-stable/rushomon-url-shortener-api',
            },
          ],
        }] : []),
        {
          href: 'https://rushomon.cc',
          label: 'rushomon.cc',
          position: 'right',
        },
        {
          href: 'https://github.com/piffio/rushomon',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Documentation',
          items: [
            {
              label: 'API Reference',
              to: '/docs/api',
            },
            {
              label: 'Self-Hosting Guide',
              href: 'https://github.com/piffio/rushomon/blob/main/docs/SELF_HOSTING.md',
            },
          ],
        },
        {
          title: 'Rushomon',
          items: [
            {
              label: 'Website',
              href: 'https://rushomon.cc',
            },
            {
              label: 'GitHub',
              href: 'https://github.com/piffio/rushomon',
            },
            {
              label: 'Issues',
              href: 'https://github.com/piffio/rushomon/issues',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Rushomon. Licensed under AGPL-3.0.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['bash', 'json', 'rust'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
