import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';
import { existsSync } from 'fs';
import { resolve } from 'path';

// Import the generated OpenAPI sidebar items (exported as array directly)
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const apiItems: any[] = require('./docs/api/sidebar.ts');

// Stable sidebar — only available after a versioned release has been deployed
const stableSidebarPath = resolve(__dirname, 'docs/api-stable/sidebar.ts');
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const apiStableItems: any[] = existsSync(stableSidebarPath)
  ? require('./docs/api-stable/sidebar.ts')
  : [];

const sidebars: SidebarsConfig = {
  defaultSidebar: [
    {
      type: 'doc',
      id: 'intro',
      label: 'Getting Started',
    },
    {
      type: 'category',
      label: 'API',
      link: {
        type: 'doc',
        id: 'api/rushomon-url-shortener-api',
      },
      items: apiItems.slice(1),
    },
  ],
  ...(apiStableItems.length > 0 ? {
    stableSidebar: [
      {
        type: 'doc',
        id: 'intro',
        label: 'Getting Started',
      },
      {
        type: 'category',
        label: 'API',
        link: {
          type: 'doc',
          id: 'api-stable/rushomon-url-shortener-api',
        },
        items: apiStableItems.slice(1),
      },
    ],
  } : {}),
};

export default sidebars;
