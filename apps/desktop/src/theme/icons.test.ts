import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { ThemeDocument } from "./types.ts";
import {
  BUILTIN_ICONS,
  builtinIconNames,
  customIconPath,
  hasBuiltinIcon,
  resolveIcon,
  sanitizeSvg,
} from "./icons.ts";

const here = dirname(fileURLToPath(import.meta.url));
const themesDir = join(here, "..", "..", "..", "..", "theme", "themes");

function loadTheme(name: string): ThemeDocument {
  return JSON.parse(readFileSync(join(themesDir, `${name}.json`), "utf8")) as ThemeDocument;
}

const dark = loadTheme("dark");

test("the built-in set ships the icons the shell needs", () => {
  const names = builtinIconNames();
  for (const required of ["file", "folder", "search", "settings", "mic", "canvas", "lock", "cloud"]) {
    assert.ok(names.includes(required), `missing icon ${required}`);
  }
});

test("every built-in icon is stroke based geometry on a 24x24 grid", () => {
  for (const [name, icon] of Object.entries(BUILTIN_ICONS)) {
    assert.equal(icon.set, "mcpad", name);
    assert.match(icon.markup, /^(<path|<circle|<rect)/, name);
    assert.ok(!icon.markup.includes("<svg"), `${name} must not contain an svg wrapper`);
  }
});

test("resolving an icon uses the size and stroke tokens of the theme", () => {
  const icon = resolveIcon(dark, "file");
  assert.ok(icon);
  assert.equal(icon.size, "18px");
  assert.equal(icon.strokeWidth, 1.5);
  assert.equal(icon.custom, false);
  assert.equal(icon.markup, BUILTIN_ICONS.file.markup);
});

test("unknown icons resolve to undefined", () => {
  assert.equal(resolveIcon(dark, "does-not-exist"), undefined);
  assert.equal(hasBuiltinIcon("does-not-exist"), false);
});

test("a theme can override an icon with its own svg", () => {
  const theme: ThemeDocument = {
    ...dark,
    icons: { file: "icons/my-file.svg" },
  };

  assert.equal(customIconPath(theme, "file"), "icons/my-file.svg");

  const icon = resolveIcon(theme, "file", '<path d="M4 4h16v16H4z"/>');
  assert.ok(icon);
  assert.equal(icon.custom, true);
  assert.equal(icon.markup, '<path d="M4 4h16v16H4z"/>');
});

test("custom icons are sanitised", () => {
  const dirty = '<script>alert(1)</script><path d="M0 0" onload="alert(2)"/>';
  const clean = sanitizeSvg(dirty);
  assert.ok(!clean.includes("script"), clean);
  assert.ok(!clean.includes("onload"), clean);
  assert.ok(clean.includes("<path"), clean);
});

test("javascript urls are removed from custom icons", () => {
  const clean = sanitizeSvg('<path d="M0 0" fill="javascript:alert(1)"/>');
  assert.ok(!clean.includes("javascript:"), clean);
});
