/**
 * Types of the MCPad theme document.
 *
 * The runtime shape is a mirror of `theme/schema/theme.schema.json`. The Rust
 * core keeps the same shape in `mcpad_core::theme`, so a theme file can be
 * written once and used by both sides.
 */

export type ThemeMode = "light" | "dark" | "high-contrast";

export type ColorScale = Record<string, string>;

export interface SurfaceTokens {
  base: string;
  raised: string;
  sunken: string;
  overlay: string;
  hover: string;
  active: string;
  selected: string;
}

export interface TextTokens {
  primary: string;
  secondary: string;
  muted: string;
  inverse: string;
  link: string;
  code: string;
}

export interface BorderTokens {
  default: string;
  strong: string;
  subtle: string;
  focus: string;
}

export interface AccentTokens {
  default: string;
  hover: string;
  active: string;
  subtle: string;
  on: string;
}

export interface StatusTokens {
  success: string;
  warning: string;
  danger: string;
  info: string;
}

export interface EditorTokens {
  bg: string;
  gutterBg: string;
  gutterText: string;
  activeLine: string;
  selection: string;
  selectionInactive: string;
  cursor: string;
  matchingBracket: string;
  indentGuide: string;
  findMatch: string;
  findMatchCurrent: string;
}

export interface CanvasTokens {
  bg: string;
  grid: string;
  gridMajor: string;
  nodeBg: string;
  nodeBorder: string;
  nodeSelected: string;
  nodeText: string;
  edge: string;
  edgeLabel: string;
  groupBg: string;
  groupBorder: string;
  groupLabel: string;
}

export interface DiffTokens {
  added: string;
  removed: string;
  modified: string;
}

export interface SemanticColorTokens {
  surface: SurfaceTokens;
  text: TextTokens;
  border: BorderTokens;
  accent: AccentTokens;
  status: StatusTokens;
  editor: EditorTokens;
  canvas: CanvasTokens;
  diff: DiffTokens;
}

export interface ColorTokens {
  primitive: Record<string, ColorScale>;
  semantic: SemanticColorTokens;
  component: Record<string, Record<string, string>>;
}

export interface TypographyTokens {
  fontFamily: Record<string, string>;
  fontSize: Record<string, string>;
  fontWeight: Record<string, number>;
  lineHeight: Record<string, number>;
  letterSpacing: Record<string, string>;
}

export interface DensityTokens {
  scale: number;
  spacingUnit: string;
}

export interface IconTokens {
  set: string;
  size: Record<string, string>;
  strokeWidth: Record<string, number>;
}

export interface ThemeTokens {
  color: ColorTokens;
  typography: TypographyTokens;
  radius: Record<string, string>;
  shadow: Record<string, string>;
  density: DensityTokens;
  icon: IconTokens;
}

export interface ThemeDocument {
  name: string;
  version: number;
  mode: ThemeMode;
  author?: string;
  description?: string;
  icons?: Record<string, string>;
  tokens: ThemeTokens;
}

/** Minimal style target, so the engine works with the DOM and with tests. */
export interface StyleTarget {
  setProperty(name: string, value: string): void;
  removeProperty(name: string): void;
  setAttribute?(name: string, value: string): void;
}
