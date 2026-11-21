/**
 * The icons that come from an icon set.
 *
 * The artwork belongs to its authors and lives outside the repository: the
 * installer `scripts/install-icons.mjs` downloads it from the source
 * repository and checks the checksum. When the files are missing, the
 * interface falls back to the hand written set of the theme, so a fresh clone
 * still works.
 *
 * The names of the icons are the names of MCPad, not the names of the source
 * set. The mapping lives in `scripts/icons.manifest.json`.
 */

/** The raw content of every installed icon, by the name of MCPad. */
const modules = import.meta.glob("../icons/vendor/*.svg", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/**
 * The inner markup of an SVG file.
 *
 * The comment in front of the `<svg>` element carries the tags of the icon
 * and is of no use inside the application.
 */
export function innerMarkup(svg: string): string {
  const start = svg.indexOf("<svg");
  if (start < 0) return "";
  const open = svg.indexOf(">", start);
  const end = svg.lastIndexOf("</svg>");
  if (open < 0 || end < 0) return "";
  return svg.slice(open + 1, end);
}

/** The installed icons, by the name of MCPad. */
export const VENDOR_ICONS: Record<string, string> = Object.fromEntries(
  Object.entries(modules).map(([path, svg]) => {
    const name = path.slice(path.lastIndexOf("/") + 1).replace(/\.svg$/, "");
    return [name, innerMarkup(svg)];
  }),
);

/** Whether an icon of the set is installed. */
export function hasVendorIcon(name: string): boolean {
  return Object.prototype.hasOwnProperty.call(VENDOR_ICONS, name);
}
