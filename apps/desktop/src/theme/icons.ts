/**
 * Icon registry.
 *
 * Icons are addressed by name (`icon.file-plus`) and rendered with
 * `currentColor`, so an icon always follows the semantic colour tokens. A
 * theme may replace any icon with its own SVG file; the path is resolved from
 * the `icons` map of the theme document.
 *
 * The built-in set is hand written geometry on a 24x24 grid, stroke based and
 * free of third party assets.
 */

import type { ThemeDocument } from "./types.ts";

export interface IconDefinition {
  /** Name of the icon set, e.g. `mcpad`. */
  readonly set: string;
  /** Inner markup of the SVG element, without the `<svg>` wrapper. */
  readonly markup: string;
}

export interface ResolvedIcon {
  /** Name of the icon. */
  readonly name: string;
  /** Inner markup of the SVG element. */
  readonly markup: string;
  /** Size, taken from the icon size tokens of the theme. */
  readonly size: string;
  /** Stroke width, taken from the icon stroke width tokens of the theme. */
  readonly strokeWidth: number;
  /** Whether the icon comes from the theme instead of the built-in set. */
  readonly custom: boolean;
}

const ICON_SIZE_DEFAULT = "18px";
const ICON_STROKE_DEFAULT = 1.5;

/** The built-in icon set. */
export const BUILTIN_ICONS: Record<string, IconDefinition> = {
  file: {
    set: "mcpad",
    markup: '<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v4h4"/>',
  },
  "file-plus": {
    set: "mcpad",
    markup:
      '<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v4h4"/><path d="M12 11v6M9 14h6"/>',
  },
  folder: {
    set: "mcpad",
    markup: '<path d="M3 7h6l2 2h10v10H3z"/>',
  },
  search: {
    set: "mcpad",
    markup: '<circle cx="11" cy="11" r="6"/><path d="M15.5 15.5L20 20"/>',
  },
  settings: {
    set: "mcpad",
    markup:
      '<circle cx="12" cy="12" r="3"/><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M18.4 5.6l-2.1 2.1M7.7 16.3l-2.1 2.1"/>',
  },
  mic: {
    set: "mcpad",
    markup:
      '<rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/>',
  },
  canvas: {
    set: "mcpad",
    markup:
      '<rect x="4" y="5" width="16" height="14" rx="2"/><rect x="7" y="8" width="5" height="4" rx="1"/><rect x="14" y="13" width="3" height="3" rx="1"/>',
  },
  note: {
    set: "mcpad",
    markup: '<rect x="5" y="4" width="14" height="16" rx="2"/><path d="M8 9h8M8 13h8M8 17h5"/>',
  },
  "chevron-right": {
    set: "mcpad",
    markup: '<path d="M9 5l7 7-7 7"/>',
  },
  check: {
    set: "mcpad",
    markup: '<path d="M5 13l4 4 10-10"/>',
  },
  close: {
    set: "mcpad",
    markup: '<path d="M6 6l12 12M18 6L6 18"/>',
  },
  lock: {
    set: "mcpad",
    markup: '<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
  },
  cloud: {
    set: "mcpad",
    markup: '<path d="M7 18a4 4 0 0 1 .6-7.96A6 6 0 0 1 19.4 11 3.5 3.5 0 0 1 18.5 18z"/>',
  },
  sync: {
    set: "mcpad",
    markup:
      '<path d="M4 12a8 8 0 0 1 13.7-5.6L20 8"/><path d="M20 12a8 8 0 0 1-13.7 5.6L4 16"/><path d="M20 4v4h-4M4 20v-4h4"/>',
  },
  trash: {
    set: "mcpad",
    markup:
      '<path d="M4 7h16M9 7V5h6v2M6 7l1 13h10l1-13M10 11v6M14 11v6"/>',
  },
  edit: {
    set: "mcpad",
    markup: '<path d="M4 20h4L18 10l-4-4L4 16z"/><path d="M14 6l4 4"/>',
  },
  plus: {
    set: "mcpad",
    markup: '<path d="M12 5v14M5 12h14"/>',
  },
  copy: {
    set: "mcpad",
    markup:
      '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/>',
  },
  robot: {
    set: "mcpad",
    markup:
      '<rect x="4" y="8" width="16" height="12" rx="2"/><path d="M12 8V5M9 13h.01M15 13h.01M9 16h6M12 3v2"/>',
  },
};

/**
 * Removes dangerous parts from an SVG fragment.
 *
 * This is a defensive first layer for user supplied icons, not a complete
 * sanitizer. When the frontend gets its dependency tree, this should be
 * replaced by a dedicated sanitizer such as DOMPurify.
 */
export function sanitizeSvg(markup: string): string {
  return markup
    .replace(/<\s*script[\s\S]*?<\s*\/\s*script\s*>/gi, "")
    .replace(/<\s*script[^>]*\/?>/gi, "")
    .replace(/\son\w+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)/gi, "")
    .replace(/javascript\s*:/gi, "");
}

/** Whether an icon name is provided by the built-in set. */
export function hasBuiltinIcon(name: string): boolean {
  return Object.prototype.hasOwnProperty.call(BUILTIN_ICONS, name);
}

/**
 * Resolves an icon for a theme.
 *
 * A theme may override an icon with the path of an SVG file. The file is
 * loaded by the caller and passed in through `customMarkup`.
 */
export function resolveIcon(
  theme: ThemeDocument,
  name: string,
  customMarkup?: string,
): ResolvedIcon | undefined {
  const size = theme.tokens.icon.size.md ?? ICON_SIZE_DEFAULT;
  const strokeWidth = theme.tokens.icon.strokeWidth.normal ?? ICON_STROKE_DEFAULT;

  if (customMarkup) {
    return {
      name,
      markup: sanitizeSvg(customMarkup),
      size,
      strokeWidth,
      custom: true,
    };
  }

  const builtin = BUILTIN_ICONS[name];
  if (!builtin) {
    return undefined;
  }

  return {
    name,
    markup: builtin.markup,
    size,
    strokeWidth,
    custom: false,
  };
}

/**
 * The path of a user supplied icon for this theme, if there is one.
 */
export function customIconPath(theme: ThemeDocument, name: string): string | undefined {
  return theme.icons?.[name];
}

/** All names of the built-in icon set. */
export function builtinIconNames(): string[] {
  return Object.keys(BUILTIN_ICONS).sort();
}
