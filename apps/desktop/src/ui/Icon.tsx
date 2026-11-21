/**
 * The icon component.
 *
 * An icon is addressed by the name of MCPad. The artwork of the installed icon
 * set wins; the hand written set of the theme is the fallback, so the
 * interface works even before the icons are downloaded.
 *
 * The size and the thickness come from the design tokens of the active theme,
 * the component itself contains no colour and no size.
 */

import { BUILTIN_ICONS, sanitizeSvg } from "../theme/icons.ts";
import { VENDOR_ICONS } from "../theme/vendor.ts";

export interface IconProps {
  /** Name of the icon, for example `search`. */
  name: string;
  /** Additional class of the element. */
  className?: string;
  /** Accessible name of the icon. */
  title?: string;
}

/**
 * Renders an icon.
 *
 * The markup is inserted as it is, because it comes either from the own set
 * of the theme or from a file whose checksum was verified by the installer.
 */
export function Icon({ name, className, title }: IconProps) {
  const vendor = VENDOR_ICONS[name];
  const builtin = BUILTIN_ICONS[name];
  if (!vendor && !builtin) return null;

  const markup = sanitizeSvg(vendor ?? builtin.markup);

  return (
    <svg
      className={className ? `app-icon ${className}` : "app-icon"}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
      style={{
        width: "var(--mcpad-icon-size-md)",
        height: "var(--mcpad-icon-size-md)",
        strokeWidth: "var(--mcpad-icon-stroke-width-normal)",
      }}
      role={title ? "img" : "presentation"}
      aria-label={title}
      aria-hidden={title ? undefined : "true"}
      dangerouslySetInnerHTML={{ __html: markup }}
    />
  );
}
