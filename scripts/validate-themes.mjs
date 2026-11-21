/**
 * Validates the built-in MCPad themes.
 *
 * Four checks, all of them run in CI:
 *
 *   1. every theme matches `theme/schema/theme.schema.json`;
 *   2. every theme exposes exactly the same token keys;
 *   3. the committed theme files match the generator output;
 *   4. the important colour pairs reach the WCAG contrast thresholds.
 *
 * The script has no npm dependencies: the JSON schema is validated with a small
 * built-in validator so the schema stays the single source of truth.
 *
 * Usage: npm run themes:validate
 */

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { THEMES, serializeTheme } from "../theme/source/themes.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const themesDir = join(root, "theme", "themes");
const schemaPath = join(root, "theme", "schema", "theme.schema.json");

const failures = [];

/* ------------------------------------------------------------------ *
 * Minimal JSON Schema validator (the subset used by our schema)
 * ------------------------------------------------------------------ */

function jsonType(value) {
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  if (Number.isInteger(value)) return "integer";
  const type = typeof value;
  if (type === "number") return "number";
  if (type === "object") return "object";
  return type;
}

function resolveRef(ref, rootSchema) {
  if (!ref.startsWith("#/")) {
    throw new Error(`unsupported $ref: ${ref}`);
  }
  let target = rootSchema;
  for (const part of ref.slice(2).split("/")) {
    target = target[part];
  }
  return target;
}

function validate(schema, data, rootSchema, path = "$", errors = []) {
  if (schema.$ref) {
    return validate(resolveRef(schema.$ref, rootSchema), data, rootSchema, path, errors);
  }

  if (schema.type) {
    const expected = Array.isArray(schema.type) ? schema.type : [schema.type];
    const actual = jsonType(data);
    const ok = expected.some((candidate) =>
      candidate === actual || (candidate === "number" && actual === "integer"),
    );
    if (!ok) {
      errors.push(`${path}: expected ${expected.join("|")}, got ${actual}`);
      return errors;
    }
  }

  if (schema.enum && !schema.enum.includes(data)) {
    errors.push(`${path}: ${JSON.stringify(data)} is not one of ${JSON.stringify(schema.enum)}`);
  }

  if (schema.const !== undefined && data !== schema.const) {
    errors.push(`${path}: expected ${JSON.stringify(schema.const)}`);
  }

  if (typeof data === "string") {
    if (schema.minLength !== undefined && data.length < schema.minLength) {
      errors.push(`${path}: shorter than ${schema.minLength}`);
    }
    if (schema.maxLength !== undefined && data.length > schema.maxLength) {
      errors.push(`${path}: longer than ${schema.maxLength}`);
    }
    if (schema.pattern && !new RegExp(schema.pattern).test(data)) {
      errors.push(`${path}: does not match ${schema.pattern}`);
    }
  }

  if (typeof data === "number") {
    if (schema.minimum !== undefined && data < schema.minimum) {
      errors.push(`${path}: below minimum ${schema.minimum}`);
    }
    if (schema.maximum !== undefined && data > schema.maximum) {
      errors.push(`${path}: above maximum ${schema.maximum}`);
    }
    if (schema.exclusiveMinimum !== undefined && data <= schema.exclusiveMinimum) {
      errors.push(`${path}: not greater than ${schema.exclusiveMinimum}`);
    }
  }

  if (Array.isArray(data)) {
    if (schema.minItems !== undefined && data.length < schema.minItems) {
      errors.push(`${path}: fewer than ${schema.minItems} items`);
    }
    if (schema.items) {
      data.forEach((item, index) =>
        validate(schema.items, item, rootSchema, `${path}[${index}]`, errors),
      );
    }
  }

  if (data && typeof data === "object" && !Array.isArray(data)) {
    for (const key of schema.required ?? []) {
      if (!(key in data)) {
        errors.push(`${path}: missing required property "${key}"`);
      }
    }
    const properties = schema.properties ?? {};
    for (const [key, value] of Object.entries(data)) {
      if (properties[key]) {
        validate(properties[key], value, rootSchema, `${path}.${key}`, errors);
      } else if (schema.additionalProperties === false) {
        errors.push(`${path}: unexpected property "${key}"`);
      } else if (schema.additionalProperties && typeof schema.additionalProperties === "object") {
        validate(schema.additionalProperties, value, rootSchema, `${path}.${key}`, errors);
      }
    }
  }

  return errors;
}

/* ------------------------------------------------------------------ *
 * Helpers
 * ------------------------------------------------------------------ */

function collectKeys(value, prefix = "") {
  const keys = new Set();
  for (const [key, child] of Object.entries(value)) {
    const path = prefix ? `${prefix}.${key}` : key;
    keys.add(path);
    if (child && typeof child === "object" && !Array.isArray(child)) {
      for (const nested of collectKeys(child, path)) {
        keys.add(nested);
      }
    }
  }
  return keys;
}

function hexToRgb(hex) {
  let value = hex.replace("#", "");
  if (value.length === 3 || value.length === 4) {
    value = value
      .split("")
      .map((char) => char + char)
      .join("");
  }
  return [
    parseInt(value.slice(0, 2), 16),
    parseInt(value.slice(2, 4), 16),
    parseInt(value.slice(4, 6), 16),
  ];
}

function relativeLuminance(hex) {
  const [r, g, b] = hexToRgb(hex).map((channel) => {
    const c = channel / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrastRatio(foreground, background) {
  const a = relativeLuminance(foreground);
  const b = relativeLuminance(background);
  const lighter = Math.max(a, b);
  const darker = Math.min(a, b);
  return (lighter + 0.05) / (darker + 0.05);
}

/* ------------------------------------------------------------------ *
 * Checks
 * ------------------------------------------------------------------ */

const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
const files = readdirSync(themesDir)
  .filter((file) => file.endsWith(".json"))
  .sort();

if (files.length === 0) {
  failures.push("no theme files found in theme/themes");
}

const themes = new Map();

for (const file of files) {
  const name = file.replace(/\.json$/, "");
  const raw = readFileSync(join(themesDir, file), "utf8");

  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    failures.push(`${file}: invalid JSON (${error.message})`);
    continue;
  }
  themes.set(name, parsed);

  for (const error of validate(schema, parsed, schema)) {
    failures.push(`${file}: ${error}`);
  }
}

// Every theme must expose the same tokens.
if (themes.size > 1) {
  const referenceName = [...themes.keys()][0];
  const reference = collectKeys(themes.get(referenceName).tokens);
  for (const [name, theme] of themes) {
    if (name === referenceName) continue;
    const keys = collectKeys(theme.tokens);
    for (const key of reference) {
      if (!keys.has(key)) {
        failures.push(`${name}.json: missing token "${key}" (present in ${referenceName}.json)`);
      }
    }
    for (const key of keys) {
      if (!reference.has(key)) {
        failures.push(`${name}.json: unknown token "${key}" (not in ${referenceName}.json)`);
      }
    }
  }
}

// The committed files must match the generator output.
for (const [name, theme] of Object.entries(THEMES)) {
  const target = join(themesDir, `${name}.json`);
  let raw = "";
  try {
    raw = readFileSync(target, "utf8");
  } catch {
    failures.push(`${name}.json: file is missing, run "npm run themes:build"`);
    continue;
  }
  if (raw !== serializeTheme(theme)) {
    failures.push(`${name}.json: out of sync with theme/source/themes.mjs, run "npm run themes:build"`);
  }
}

// Contrast of the pairs that matter for readability.
const CONTRAST_PAIRS = [
  ["color.semantic.text.primary", "color.semantic.surface.base", 4.5],
  ["color.semantic.text.secondary", "color.semantic.surface.base", 4.5],
  ["color.semantic.text.muted", "color.semantic.surface.base", 3],
  ["color.semantic.text.link", "color.semantic.surface.base", 4.5],
  ["color.semantic.accent.on", "color.semantic.accent.default", 4.5],
  ["color.semantic.status.success", "color.semantic.surface.base", 3],
  ["color.semantic.status.warning", "color.semantic.surface.base", 3],
  ["color.semantic.status.danger", "color.semantic.surface.base", 3],
  ["color.semantic.status.info", "color.semantic.surface.base", 3],
  ["color.semantic.editor.gutterText", "color.semantic.editor.gutterBg", 4.5],
  ["color.semantic.editor.cursor", "color.semantic.editor.bg", 3],
  ["color.semantic.canvas.nodeText", "color.semantic.canvas.nodeBg", 4.5],
  ["color.semantic.canvas.edgeLabel", "color.semantic.canvas.bg", 4.5],
];

/** Looks a token up by its path relative to `tokens`. */
function lookup(theme, dottedPath) {
  return dottedPath
    .split(".")
    .reduce((node, key) => (node ? node[key] : undefined), theme.tokens);
}

for (const [name, theme] of themes) {
  for (const [foregroundPath, backgroundPath, threshold] of CONTRAST_PAIRS) {
    const foreground = lookup(theme, foregroundPath);
    const background = lookup(theme, backgroundPath);
    if (typeof foreground !== "string" || typeof background !== "string") {
      failures.push(`${name}.json: missing colour pair ${foregroundPath} / ${backgroundPath}`);
      continue;
    }
    const ratio = contrastRatio(foreground, background);
    if (ratio < threshold) {
      failures.push(
        `${name}.json: ${foregroundPath} on ${backgroundPath} has contrast ${ratio.toFixed(2)}:1, needs ${threshold}:1`,
      );
    }
  }
}

/* ------------------------------------------------------------------ *
 * Report
 * ------------------------------------------------------------------ */

if (failures.length > 0) {
  console.error("Theme validation failed:");
  for (const failure of failures) {
    console.error(`  - ${failure}`);
  }
  process.exit(1);
}

console.log(`Theme validation passed for ${themes.size} theme(s): ${[...themes.keys()].join(", ")}`);
