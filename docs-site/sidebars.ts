import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

// Import the generated OpenAPI sidebar items (exported as array directly)
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const apiItems: any[] = require('./docs/sidebar.ts');

const sidebars: SidebarsConfig = {
  defaultSidebar: [
    {
      type: 'doc',
      id: 'intro',
      label: 'Getting Started',
    },
    {
      type: 'doc',
      id: 'permissions',
      label: 'Permissions & Tiers',
    },
    {
      type: 'category',
      label: 'API',
      link: {
        type: 'doc',
        id: 'rushomon-url-shortener-api',
      },
      items: apiItems.slice(1),
    },
  ],
};

export default sidebars;
