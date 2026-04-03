import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';
import type * as OpenApiPlugin from 'docusaurus-plugin-openapi-docs';
import {existsSync, readFileSync} from 'fs';
import {resolve} from 'path';

const DOCS_SITE_URL = process.env.DOCS_SITE_URL || 'https://doc.rushomon.cc';

// Read versions from the Docusaurus versions.json (created by `docusaurus docs:version`)
const VERSIONS_JSON = resolve(__dirname, 'versions.json');
const docusaurusVersions: string[] = existsSync(VERSIONS_JSON)
  ? JSON.parse(readFileSync(VERSIONS_JSON, 'utf8'))
  : [];
const latestStableVersion = docusaurusVersions.length > 0 ? docusaurusVersions[0] : null;

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
          lastVersion: latestStableVersion ?? 'current',
          versions: {
            current: {
              label: 'main (unreleased)',
              path: 'next',
              banner: 'none',
            },
            ...(latestStableVersion ? {
              [latestStableVersion]: {
                label: `${latestStableVersion} (stable)`,
                banner: 'none',
              },
            } : {}),
          },
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
            outputDir: 'docs',
            showSchemas: true,
            sidebarOptions: {
              groupPathsBy: 'tag',
              categoryLinkSource: 'tag',
            },
          } satisfies OpenApiPlugin.Options,
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
          to: '/docs/api',
          position: 'left',
          label: 'API Reference',
        },
        ...(latestStableVersion ? [{
          type: 'docsVersionDropdown' as const,
          position: 'left' as const,
          dropdownActiveClassDisabled: true,
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
