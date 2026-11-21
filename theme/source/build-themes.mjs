/**
 * Regenerates the built-in theme files from `theme/source/themes.mjs`.
 *
 * Usage: npm run themes:build
 */

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { THEMES, serializeTheme } from "./themes.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const outputDir = join(here, "..", "themes");

mkdirSync(outputDir, { recursive: true });

for (const [name, theme] of Object.entries(THEMES)) {
  const target = join(outputDir, `${name}.json`);
  writeFileSync(target, serializeTheme(theme), "utf8");
  console.log(`written ${target}`);
}
