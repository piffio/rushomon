import Critters from "critters";
import fs from "fs/promises";
import path from "path";

const critters = new Critters({
  path: "build",
  publicPath: "/",
  inlineThreshold: 10240, // Inline CSS smaller than 10KB
  minimumExternalSize: 512, // Don't inline if external file is smaller than 512B
  pruneSource: true, // Remove inlined styles from external CSS
  mergeStylesheets: true, // Merge stylesheets for better compression
  preloadAsync: true // Use <link rel="preload"> for async CSS loading
});

async function processHtmlFiles(dir) {
  const entries = await fs.readdir(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);

    if (entry.isDirectory()) {
      await processHtmlFiles(fullPath);
    } else if (entry.name.endsWith(".html")) {
      console.log(`Processing ${fullPath}...`);

      try {
        const html = await fs.readFile(fullPath, "utf-8");
        const inlined = await critters.process(html);
        await fs.writeFile(fullPath, inlined, "utf-8");
        console.log(`✓ Inlined critical CSS in ${fullPath}`);
      } catch (err) {
        console.error(`✗ Error processing ${fullPath}:`, err.message);
      }
    }
  }
}

// Main execution
console.log("Inlining critical CSS...");
await processHtmlFiles("build");
console.log("Done!");
