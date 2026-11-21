import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

/**
 * The canonical themes live in `theme/themes` at the root of the repository.
 * This plugin serves them in development and ships them with the build, so the
 * desktop app never carries a second copy of the theme files.
 */
function themeAssets(): Plugin {
  const themesDir = join(import.meta.dirname, "..", "..", "theme", "themes");

  return {
    name: "mcpad-theme-assets",
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        const match = /^\/themes\/([\w-]+)\.json$/.exec(request.url ?? "");
        if (!match) {
          next();
          return;
        }
        try {
          const body = readFileSync(join(themesDir, `${match[1]}.json`), "utf8");
          response.setHeader("Content-Type", "application/json");
          response.end(body);
        } catch {
          response.statusCode = 404;
          response.end("{}");
        }
      });
    },
    generateBundle() {
      for (const file of readdirSync(themesDir)) {
        if (!file.endsWith(".json")) continue;
        this.emitFile({
          type: "asset",
          fileName: `themes/${file}`,
          source: readFileSync(join(themesDir, file), "utf8"),
        });
      }
    },
  };
}

export default defineConfig({
  plugins: [react(), themeAssets()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "es2022",
    sourcemap: true,
  },
});
