/**
 * The MCPad theme engine.
 *
 * A theme document is flattened into CSS custom properties (`--mcpad-*`) and
 * applied to the document root. Components never hardcode a colour: they read
 * `var(--mcpad-color-semantic-text-primary)` and friends. CodeMirror and
 * React Flow are themed through the very same variables, so a single theme
 * file covers the editor, the canvas and the shell.
 */

import type { StyleTarget, ThemeDocument } from "./types.ts";
import { validateTheme, type ValidationResult } from "./validate.ts";

export { parseTheme, validateTheme } from "./validate.ts";
export type { ValidationResult } from "./validate.ts";

const CSS_PREFIX = "--mcpad";

/**
 * Short aliases for the tokens that are used everywhere.
 * They keep the CSS short without hiding the canonical names.
 */
export const TOKEN_ALIASES: Record<string, string> = {
  bg: "color.semantic.surface.base",
  "bg-raised": "color.semantic.surface.raised",
  "bg-sunken": "color.semantic.surface.sunken",
  fg: "color.semantic.text.primary",
  "fg-secondary": "color.semantic.text.secondary",
  "fg-muted": "color.semantic.text.muted",
  border: "color.semantic.border.default",
  "border-strong": "color.semantic.border.strong",
  accent: "color.semantic.accent.default",
  "accent-hover": "color.semantic.accent.hover",
  "accent-fg": "color.semantic.accent.on",
  danger: "color.semantic.status.danger",
  warning: "color.semantic.status.warning",
  success: "color.semantic.status.success",
  "editor-bg": "color.semantic.editor.bg",
  "editor-fg": "color.semantic.text.primary",
  "editor-selection": "color.semantic.editor.selection",
  "editor-cursor": "color.semantic.editor.cursor",
  "editor-gutter": "color.semantic.editor.gutterBg",
  "canvas-bg": "color.semantic.canvas.bg",
  "canvas-node-bg": "color.semantic.canvas.nodeBg",
  "canvas-edge": "color.semantic.canvas.edge",
};

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** `surface.base` becomes `surface-base`, `gutterBg` becomes `gutter-bg`. */
function toKebab(value: string): string {
  return value.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();
}

/** The CSS custom property of a token path. */
export function tokenVariable(path: string): string {
  return `${CSS_PREFIX}-${path.split(".").map(toKebab).join("-")}`;
}

/** Flattens a theme into CSS custom properties. */
export function flattenTheme(theme: ThemeDocument): Record<string, string> {
  const variables: Record<string, string> = {};

  const walk = (node: unknown, path: string[]): void => {
    if (isPlainObject(node)) {
      for (const [key, value] of Object.entries(node)) {
        walk(value, [...path, key]);
      }
      return;
    }
    if (path.length === 0 || node === undefined || node === null) {
      return;
    }
    variables[tokenVariable(path.join("."))] = String(node);
  };

  walk(theme.tokens, []);
  return variables;
}

/** The alias variables derived from the canonical tokens. */
export function aliasVariables(theme: ThemeDocument): Record<string, string> {
  const canonical = flattenTheme(theme);
  const aliases: Record<string, string> = {};

  for (const [alias, path] of Object.entries(TOKEN_ALIASES)) {
    const value = canonical[tokenVariable(path)];
    if (value !== undefined) {
      aliases[`${CSS_PREFIX}-${alias}`] = value;
    }
  }

  return aliases;
}

/**
 * Applies a theme to a style target.
 *
 * @returns the names of the variables that were set, so they can be removed
 *          again when the user switches to another theme.
 */
export function applyTheme(
  theme: ThemeDocument,
  target: StyleTarget,
): string[] {
  const variables = { ...flattenTheme(theme), ...aliasVariables(theme) };

  for (const [name, value] of Object.entries(variables)) {
    target.setProperty(name, value);
  }
  target.setAttribute?.("data-theme-mode", theme.mode);
  target.setAttribute?.("data-theme-name", theme.name);

  return Object.keys(variables);
}

/** Removes previously applied theme variables. */
export function clearTheme(target: StyleTarget, names: string[]): void {
  for (const name of names) {
    target.removeProperty(name);
  }
}

/** Renders a theme as a CSS block, used for exports and for printing. */
export function themeStyle(theme: ThemeDocument, selector = ":root"): string {
  const variables = { ...flattenTheme(theme), ...aliasVariables(theme) };
  const body = Object.entries(variables)
    .map(([name, value]) => `  ${name}: ${value};`)
    .join("\n");
  return `${selector} {\n${body}\n}\n`;
}

/** Merges a user theme over the built-in default. */
export function resolveTheme(
  override: unknown,
  fallback: ThemeDocument,
): ValidationResult {
  return validateTheme(override, fallback);
}

/** The path of a user supplied icon, if the theme overrides it. */
export function iconSource(theme: ThemeDocument, name: string): string | undefined {
  return theme.icons?.[name];
}
