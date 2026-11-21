import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { StyleTarget, ThemeDocument } from "./types.ts";
import {
  aliasVariables,
  applyTheme,
  clearTheme,
  flattenTheme,
  parseTheme,
  resolveTheme,
  themeStyle,
  tokenVariable,
} from "./engine.ts";

const here = dirname(fileURLToPath(import.meta.url));
const themesDir = join(here, "..", "..", "..", "..", "theme", "themes");

function loadTheme(name: string): ThemeDocument {
  return JSON.parse(readFileSync(join(themesDir, `${name}.json`), "utf8")) as ThemeDocument;
}

class FakeStyleTarget implements StyleTarget {
  readonly properties = new Map<string, string>();
  readonly attributes = new Map<string, string>();

  setProperty(name: string, value: string): void {
    this.properties.set(name, value);
  }

  removeProperty(name: string): void {
    this.properties.delete(name);
  }

  setAttribute(name: string, value: string): void {
    this.attributes.set(name, value);
  }
}

const dark = loadTheme("dark");
const light = loadTheme("light");

test("token paths become kebab case css variables", () => {
  assert.equal(
    tokenVariable("color.semantic.editor.gutterBg"),
    "--mcpad-color-semantic-editor-gutter-bg",
  );
  assert.equal(
    tokenVariable("typography.fontSize.md"),
    "--mcpad-typography-font-size-md",
  );
});

test("flattening covers every leaf of the token tree", () => {
  const variables = flattenTheme(dark);
  assert.equal(variables["--mcpad-color-semantic-surface-base"], "#1c2027");
  assert.equal(variables["--mcpad-color-primitive-gray-900"], "#1c2027");
  assert.equal(variables["--mcpad-typography-font-size-md"], "13px");
  assert.equal(variables["--mcpad-radius-md"], "6px");
  assert.equal(variables["--mcpad-density-scale"], "1");
  assert.equal(variables["--mcpad-icon-stroke-width-normal"], "1.5");
});

test("aliases point at the canonical tokens", () => {
  const aliases = aliasVariables(dark);
  assert.equal(aliases["--mcpad-bg"], "#1c2027");
  assert.equal(aliases["--mcpad-fg"], "#e8ebf0");
  assert.equal(aliases["--mcpad-accent"], "#2563eb");
});

test("applying a theme writes css variables and the mode attribute", () => {
  const target = new FakeStyleTarget();
  const names = applyTheme(dark, target);

  assert.ok(names.includes("--mcpad-color-semantic-surface-base"));
  assert.equal(target.properties.get("--mcpad-color-semantic-surface-base"), "#1c2027");
  assert.equal(target.attributes.get("data-theme-mode"), "dark");
  assert.equal(target.attributes.get("data-theme-name"), "MCPad Dark");
});

test("switching themes removes the variables of the previous one", () => {
  const target = new FakeStyleTarget();
  const previous = applyTheme(dark, target);
  clearTheme(target, previous);
  assert.equal(target.properties.size, 0);
});

test("a partial user theme inherits every missing token", () => {
  const result = resolveTheme(
    {
      name: "My theme",
      version: 1,
      mode: "dark",
      tokens: {
        color: {
          semantic: {
            surface: { base: "#101010" },
          },
        },
      },
    },
    dark,
  );

  assert.deepEqual(result.errors, []);
  assert.equal(result.theme.name, "My theme");
  assert.equal(result.theme.tokens.color.semantic.surface.base, "#101010");
  assert.equal(result.theme.tokens.color.semantic.surface.raised, "#232833");
  assert.equal(result.theme.tokens.color.semantic.text.primary, "#e8ebf0");
});

test("an invalid colour falls back to the default value", () => {
  const result = resolveTheme(
    {
      name: "Broken",
      version: 1,
      mode: "dark",
      tokens: { color: { semantic: { surface: { base: "not-a-colour" } } } },
    },
    dark,
  );

  assert.equal(result.errors.length, 1);
  assert.match(result.errors[0], /not a hex colour/);
  assert.equal(result.theme.tokens.color.semantic.surface.base, "#1c2027");
});

test("unknown tokens of a fixed section are rejected", () => {
  const result = resolveTheme(
    {
      name: "Extra",
      version: 1,
      mode: "dark",
      tokens: { color: { semantic: { surface: { nope: "#ffffff" } } } },
    },
    dark,
  );

  assert.equal(result.errors.length, 1);
  assert.match(result.errors[0], /unknown token/);
});

test("extensible sections accept additional keys", () => {
  const result = resolveTheme(
    {
      name: "Extended",
      version: 1,
      mode: "dark",
      tokens: {
        color: {
          primitive: { teal: { 500: "#008080" } },
          component: { toolbar: { bg: "#000000" } },
        },
      },
    },
    dark,
  );

  assert.deepEqual(result.errors, []);
  assert.equal(result.theme.tokens.color.primitive.teal[500], "#008080");
  assert.equal(result.theme.tokens.color.component.toolbar.bg, "#000000");
});

test("a wrong type never replaces the default", () => {
  const result = resolveTheme(
    {
      name: "Wrong type",
      version: 1,
      mode: "dark",
      tokens: { typography: { fontWeight: { regular: "bold" } } },
    },
    dark,
  );

  assert.equal(result.errors.length, 1);
  assert.equal(result.theme.tokens.typography.fontWeight.regular, 400);
});

test("broken json falls back to the default theme", () => {
  const result = parseTheme("{ not json", dark);
  assert.equal(result.theme, dark);
  assert.match(result.errors[0], /invalid JSON/);
});

test("an invalid mode falls back to the mode of the default theme", () => {
  const result = resolveTheme({ name: "x", version: 1, mode: "neon" }, dark);
  assert.equal(result.theme.mode, "dark");
  assert.equal(result.errors.length, 1);
});

test("the css block contains every variable", () => {
  const css = themeStyle(light);
  assert.match(css, /^:root \{/);
  assert.match(css, /--mcpad-color-semantic-surface-base: #ffffff;/);
  assert.match(css, /--mcpad-fg: #111827;/);
});
