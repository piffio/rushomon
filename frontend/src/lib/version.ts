import { version } from '../../package.json';

export const APP_VERSION = version || 'dev';

export function getVersionInfo() {
  return {
    version: APP_VERSION,
    name: 'Rushomon',
    isProduction: import.meta.env.PROD,
  };
}
