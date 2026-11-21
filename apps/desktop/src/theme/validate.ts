/**
 * Runtime validation of a theme document.
 *
 * A user theme is merged over the built-in default. Every token that is
 * missing or malformed falls back to the default value and is reported as an
 * error, so a broken theme file can never break the editor.
 */

import type { ThemeDocument } from "./types.ts";

const HEX_COLOUR = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/;

/**
 * Paths that accept additional keys. Everything else follows the contract of
 * the built-in theme and rejects unknown tokens.
 */
const EXTENSIBLE_PATHS = [
  "color.primitive",
  "color.component",
  "typography",
  "radius",
  "shadow",
  "icon.size",
  "icon.strokeWidth",
];

export interface ValidationResult {
  theme: ThemeDocument;
  errors: string[];
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isExtensible(path: string): boolean {
  // Token paths are reported with the `tokens.` prefix, the rules are not.
  const normalized = path.startsWith("tokens.") ? path.slice("tokens.".length) : path;
  return EXTENSIBLE_PATHS.some(
    (candidate) => normalized === candidate || normalized.startsWith(`${candidate}.`),
  );
}

/**
 * Merges `override` over `base`.
 *
 * The type of the base value decides what is accepted: colours must be hex
 * strings, weights and line heights must be numbers, and so on. Unknown keys
 * are only kept on extensible paths.
 */
function mergeValue(
  override: unknown,
  base: unknown,
  path: string,
  errors: string[],
): unknown {
  if (override === undefined) {
    return base;
  }

  if (isPlainObject(base)) {
    if (!isPlainObject(override)) {
      errors.push(`${path}: expected an object`);
      return base;
    }
    const result: Record<string, unknown> = { ...base };
    for (const [key, value] of Object.entries(override)) {
      const childPath = path ? `${path}.${key}` : key;
      if (!(key in base)) {
        if (isExtensible(path)) {
          result[key] = value;
        } else {
          errors.push(`${childPath}: unknown token`);
        }
        continue;
      }
      result[key] = mergeValue(value, base[key], childPath, errors);
    }
    return result;
  }

  if (typeof base === "number") {
    if (typeof override !== "number" || !Number.isFinite(override)) {
      errors.push(`${path}: expected a number`);
      return base;
    }
    return override;
  }

  if (typeof base === "string") {
    if (typeof override !== "string") {
      errors.push(`${path}: expected a string`);
      return base;
    }
    if (base.startsWith("#") && !HEX_COLOUR.test(override)) {
      errors.push(`${path}: "${override}" is not a hex colour`);
      return base;
    }
    return override;
  }

  errors.push(`${path}: unsupported token type`);
  return base;
}

/**
 * Validates a theme document against the default theme.
 *
 * @param input   the parsed user theme, or `undefined`
 * @param fallback the built-in theme used for every missing or invalid token
 */
export function validateTheme(
  input: unknown,
  fallback: ThemeDocument,
): ValidationResult {
  const errors: string[] = [];

  if (!isPlainObject(input)) {
    return {
      theme: fallback,
      errors: ["theme: the document is not an object"],
    };
  }

  const name = typeof input.name === "string" ? input.name : fallback.name;
  if (typeof input.name !== "string") {
    errors.push("name: expected a string");
  }

  const version = typeof input.version === "number" ? input.version : fallback.version;
  if (typeof input.version !== "number") {
    errors.push("version: expected a number");
  }

  const mode =
    input.mode === "light" || input.mode === "dark" || input.mode === "high-contrast"
      ? input.mode
      : fallback.mode;
  if (input.mode !== mode) {
    errors.push("mode: expected light, dark or high-contrast");
  }

  const icons = isPlainObject(input.icons)
    ? (input.icons as Record<string, string>)
    : fallback.icons;

  const tokens = mergeValue(input.tokens, fallback.tokens, "tokens", errors) as ThemeDocument["tokens"];

  return {
    theme: {
      name,
      version,
      mode,
      ...(input.author !== undefined ? { author: String(input.author) } : {}),
      ...(input.description !== undefined ? { description: String(input.description) } : {}),
      ...(icons ? { icons } : {}),
      tokens,
    },
    errors,
  };
}

/**
 * Parses a theme file.
 *
 * A broken JSON document falls back to the default theme instead of throwing:
 * a broken theme must never prevent the editor from starting.
 */
export function parseTheme(
  raw: string,
  fallback: ThemeDocument,
): ValidationResult {
  try {
    return validateTheme(JSON.parse(raw), fallback);
  } catch (error) {
    return {
      theme: fallback,
      errors: [`theme: invalid JSON (${(error as Error).message})`],
    };
  }
}
