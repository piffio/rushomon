import adapterStatic from "@sveltejs/adapter-static";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  kit: {
    adapter: adapterStatic({
      pages: "build",
      assets: "build",
      fallback: "fallback.html", // Use different name to preserve index.html
      precompress: true,
      strict: false // Allow dynamic routes with fallback
    }),
    prerender: {
      origin: process.env.PUBLIC_VITE_SITE_URL || "https://rushomon.cc",
      entries: ["/", "/pricing", "/terms", "/privacy", "/report", "/login"], // Explicitly prerender key routes
      handleUnseenRoutes: "ignore" // Don't fail if routes are not found during crawling
    }
  }
};

export default config;
