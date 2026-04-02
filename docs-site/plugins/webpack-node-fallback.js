/**
 * Docusaurus plugin to add webpack resolve.fallback for Node.js core modules.
 * Required because postman-code-generators (used by docusaurus-theme-openapi-docs)
 * references `path` in browser-bundled code.
 */
module.exports = function webpackNodeFallback() {
  return {
    name: 'webpack-node-fallback',
    configureWebpack() {
      return {
        resolve: {
          fallback: {
            path: false,
            fs: false,
            os: false,
            crypto: false,
            stream: false,
            buffer: false,
          },
        },
      };
    },
  };
};
