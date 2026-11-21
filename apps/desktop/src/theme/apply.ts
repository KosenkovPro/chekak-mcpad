import { applyTheme as applyThemeVariables } from "./engine.ts";
import type { ThemeDocument } from "./types.ts";

/**
 * Applies a theme to the document root.
 *
 * Every component reads `var(--mcpad-*)`, so applying the theme is enough to
 * repaint the whole application, including CodeMirror and React Flow.
 */
export function applyTheme(theme: ThemeDocument): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  applyThemeVariables(theme, {
    setProperty: (name, value) => root.style.setProperty(name, value),
    removeProperty: (name) => root.style.removeProperty(name),
    setAttribute: (name, value) => root.setAttribute(name, value),
  });
}
