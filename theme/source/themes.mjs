/**
 * Source of truth for the built-in MCPad themes.
 *
 * The generator guarantees that all three themes expose exactly the same token
 * keys, which is what the CI validator checks together with the JSON schema.
 * Run `npm run themes:build` after editing this file, then commit the result.
 */

const TYPOGRAPHY = {
  fontFamily: {
    sans: "Inter, 'Segoe UI', system-ui, -apple-system, sans-serif",
    mono: "'JetBrains Mono', 'Cascadia Code', Consolas, monospace",
    ui: "system-ui, -apple-system, 'Segoe UI', sans-serif",
  },
  fontSize: {
    xs: "11px",
    sm: "12px",
    md: "13px",
    lg: "15px",
    xl: "18px",
    xxl: "22px",
  },
  fontWeight: {
    regular: 400,
    medium: 500,
    semibold: 600,
    bold: 700,
  },
  lineHeight: {
    tight: 1.25,
    normal: 1.5,
    relaxed: 1.7,
  },
  letterSpacing: {
    normal: "0",
    wide: "0.02em",
  },
};

const RADIUS = {
  none: "0",
  xs: "2px",
  sm: "4px",
  md: "6px",
  lg: "10px",
  xl: "16px",
  full: "9999px",
};

const SHADOW = {
  none: "none",
  xs: "0 1px 2px rgba(0, 0, 0, 0.12)",
  sm: "0 2px 6px rgba(0, 0, 0, 0.16)",
  md: "0 8px 24px rgba(0, 0, 0, 0.20)",
  lg: "0 16px 48px rgba(0, 0, 0, 0.28)",
  focus: "0 0 0 2px rgba(37, 99, 235, 0.45)",
};

const DENSITY = {
  scale: 1,
  spacingUnit: "4px",
};

const ICON = {
  set: "mcpad",
  size: {
    xs: "14px",
    sm: "16px",
    md: "18px",
    lg: "22px",
    xl: "28px",
  },
  strokeWidth: {
    thin: 1,
    normal: 1.5,
    bold: 2,
  },
};

const DARK = {
  name: "MCPad Dark",
  version: 1,
  mode: "dark",
  author: "Ivan Kosenkov",
  description: "Default dark theme of MCPad.",
  tokens: {
    color: {
      primitive: {
        gray: { 50: "#f7f8fa", 100: "#eef0f4", 300: "#c9ced8", 500: "#8a93a3", 700: "#4a5261", 900: "#1c2027" },
        blue: { 50: "#eaf2ff", 100: "#d6e6ff", 300: "#8ab4ff", 500: "#3b82f6", 700: "#1d4ed8", 900: "#172554" },
        red: { 50: "#fee2e2", 100: "#fecaca", 300: "#fca5a5", 500: "#ef4444", 700: "#b91c1c", 900: "#7f1d1d" },
        green: { 50: "#dcfce7", 100: "#bbf7d0", 300: "#86efac", 500: "#22c55e", 700: "#15803d", 900: "#14532d" },
        amber: { 50: "#fef3c7", 100: "#fde68a", 300: "#fcd34d", 500: "#f59e0b", 700: "#b45309", 900: "#78350f" },
        violet: { 50: "#ede9fe", 100: "#ddd6fe", 300: "#c4b5fd", 500: "#8b5cf6", 700: "#6d28d9", 900: "#4c1d95" },
      },
      semantic: {
        surface: {
          base: "#1c2027",
          raised: "#232833",
          sunken: "#16191f",
          overlay: "#2b3140",
          hover: "#2a303c",
          active: "#333b4a",
          selected: "#2f3b52",
        },
        text: {
          primary: "#e8ebf0",
          secondary: "#b6bdca",
          muted: "#8a93a3",
          inverse: "#12151a",
          link: "#8ab4ff",
          code: "#c4b5fd",
        },
        border: {
          default: "#333b4a",
          strong: "#4a5261",
          subtle: "#262c37",
          focus: "#3b82f6",
        },
        accent: {
          default: "#2563eb",
          hover: "#3b82f6",
          active: "#1d4ed8",
          subtle: "#1e3a5f",
          on: "#ffffff",
        },
        status: {
          success: "#22c55e",
          warning: "#f59e0b",
          danger: "#ef4444",
          info: "#38bdf8",
        },
        editor: {
          bg: "#1c2027",
          gutterBg: "#16191f",
          gutterText: "#8a93a3",
          activeLine: "#232833",
          selection: "#2f3b52",
          selectionInactive: "#242a36",
          cursor: "#60a5fa",
          matchingBracket: "#fcd34d",
          indentGuide: "#262c37",
          findMatch: "#fcd34d",
          findMatchCurrent: "#f59e0b",
        },
        canvas: {
          bg: "#16191f",
          grid: "#232833",
          gridMajor: "#2b3140",
          nodeBg: "#232833",
          nodeBorder: "#3a4252",
          nodeSelected: "#3b82f6",
          nodeText: "#e8ebf0",
          edge: "#8a93a3",
          edgeLabel: "#b6bdca",
          groupBg: "#1a1e25",
          groupBorder: "#333b4a",
          groupLabel: "#fcd34d",
        },
        diff: {
          added: "#14532d",
          removed: "#7f1d1d",
          modified: "#78350f",
        },
      },
      component: {
        tab: {
          bg: "#16191f",
          activeBg: "#1c2027",
          activeText: "#e8ebf0",
          inactiveText: "#8a93a3",
          hoverBg: "#232833",
        },
        sidebar: { bg: "#16191f", border: "#262c37", text: "#b6bdca" },
        toolbar: { bg: "#1c2027", border: "#262c37", text: "#b6bdca" },
        statusbar: { bg: "#16191f", text: "#8a93a3" },
        modal: { overlay: "#0b0d11", bg: "#232833", border: "#3a4252" },
        mcp: {
          toolRead: "#38bdf8",
          toolWrite: "#f59e0b",
          toolPending: "#8b5cf6",
          badge: "#3b82f6",
        },
      },
    },
    typography: TYPOGRAPHY,
    radius: RADIUS,
    shadow: SHADOW,
    density: DENSITY,
    icon: ICON,
  },
};

const LIGHT = {
  name: "MCPad Light",
  version: 1,
  mode: "light",
  author: "Ivan Kosenkov",
  description: "Default light theme of MCPad.",
  tokens: {
    color: {
      primitive: {
        gray: { 50: "#f7f8fa", 100: "#eef0f4", 300: "#c9ced8", 500: "#6b7280", 700: "#374151", 900: "#111827" },
        blue: { 50: "#eff6ff", 100: "#dbeafe", 300: "#93c5fd", 500: "#3b82f6", 700: "#1d4ed8", 900: "#1e3a8a" },
        red: { 50: "#fef2f2", 100: "#fee2e2", 300: "#fca5a5", 500: "#dc2626", 700: "#b91c1c", 900: "#7f1d1d" },
        green: { 50: "#f0fdf4", 100: "#dcfce7", 300: "#86efac", 500: "#16a34a", 700: "#15803d", 900: "#14532d" },
        amber: { 50: "#fffbeb", 100: "#fef3c7", 300: "#fcd34d", 500: "#d97706", 700: "#b45309", 900: "#78350f" },
        violet: { 50: "#f5f3ff", 100: "#ede9fe", 300: "#c4b5fd", 500: "#7c3aed", 700: "#6d28d9", 900: "#4c1d95" },
      },
      semantic: {
        surface: {
          base: "#ffffff",
          raised: "#f7f8fa",
          sunken: "#eef0f4",
          overlay: "#ffffff",
          hover: "#f1f5f9",
          active: "#e2e8f0",
          selected: "#dbeafe",
        },
        text: {
          primary: "#111827",
          secondary: "#374151",
          muted: "#6b7280",
          inverse: "#ffffff",
          link: "#1d4ed8",
          code: "#6d28d9",
        },
        border: {
          default: "#c9ced8",
          strong: "#9aa3b2",
          subtle: "#e2e8f0",
          focus: "#3b82f6",
        },
        accent: {
          default: "#2563eb",
          hover: "#1d4ed8",
          active: "#1e40af",
          subtle: "#dbeafe",
          on: "#ffffff",
        },
        status: {
          success: "#16a34a",
          warning: "#d97706",
          danger: "#dc2626",
          info: "#0284c7",
        },
        editor: {
          bg: "#ffffff",
          gutterBg: "#f7f8fa",
          gutterText: "#6b7280",
          activeLine: "#f1f5f9",
          selection: "#bfdbfe",
          selectionInactive: "#e0e7ff",
          cursor: "#1d4ed8",
          matchingBracket: "#d97706",
          indentGuide: "#e2e8f0",
          findMatch: "#fde68a",
          findMatchCurrent: "#fbbf24",
        },
        canvas: {
          bg: "#f7f8fa",
          grid: "#e2e8f0",
          gridMajor: "#c9ced8",
          nodeBg: "#ffffff",
          nodeBorder: "#c9ced8",
          nodeSelected: "#2563eb",
          nodeText: "#111827",
          edge: "#6b7280",
          edgeLabel: "#374151",
          groupBg: "#f1f5f9",
          groupBorder: "#9aa3b2",
          groupLabel: "#b45309",
        },
        diff: {
          added: "#dcfce7",
          removed: "#fee2e2",
          modified: "#fef3c7",
        },
      },
      component: {
        tab: {
          bg: "#eef0f4",
          activeBg: "#ffffff",
          activeText: "#111827",
          inactiveText: "#6b7280",
          hoverBg: "#e2e8f0",
        },
        sidebar: { bg: "#f7f8fa", border: "#e2e8f0", text: "#374151" },
        toolbar: { bg: "#ffffff", border: "#e2e8f0", text: "#374151" },
        statusbar: { bg: "#f7f8fa", text: "#6b7280" },
        modal: { overlay: "#11182766", bg: "#ffffff", border: "#c9ced8" },
        mcp: {
          toolRead: "#0284c7",
          toolWrite: "#d97706",
          toolPending: "#7c3aed",
          badge: "#2563eb",
        },
      },
    },
    typography: TYPOGRAPHY,
    radius: RADIUS,
    shadow: SHADOW,
    density: DENSITY,
    icon: ICON,
  },
};

const HIGH_CONTRAST = {
  name: "MCPad High Contrast",
  version: 1,
  mode: "high-contrast",
  author: "Ivan Kosenkov",
  description: "Maximum contrast theme for bright rooms and low vision.",
  tokens: {
    color: {
      primitive: {
        gray: { 50: "#ffffff", 100: "#f2f2f2", 300: "#bfbfbf", 500: "#8a8a8a", 700: "#3f3f3f", 900: "#000000" },
        blue: { 50: "#e8f1ff", 100: "#cfe3ff", 300: "#7fb0ff", 500: "#0057ff", 700: "#003fb3", 900: "#00246b" },
        red: { 50: "#ffecec", 100: "#ffd5d5", 300: "#ff9c9c", 500: "#d40000", 700: "#990000", 900: "#5c0000" },
        green: { 50: "#e8ffe8", 100: "#d0ffd0", 300: "#8cff8c", 500: "#008000", 700: "#005c00", 900: "#003d00" },
        amber: { 50: "#fff6d6", 100: "#ffeaa0", 300: "#ffd94d", 500: "#a06000", 700: "#704300", 900: "#4a2d00" },
        violet: { 50: "#f2eaff", 100: "#e2d4ff", 300: "#bfa0ff", 500: "#6b00ff", 700: "#4c00b3", 900: "#2e0070" },
      },
      semantic: {
        surface: {
          base: "#000000",
          raised: "#0a0a0a",
          sunken: "#000000",
          overlay: "#141414",
          hover: "#1c1c1c",
          active: "#262626",
          selected: "#003fb3",
        },
        text: {
          primary: "#ffffff",
          secondary: "#e8e8e8",
          muted: "#c0c0c0",
          inverse: "#000000",
          link: "#7fb0ff",
          code: "#bfa0ff",
        },
        border: {
          default: "#8a8a8a",
          strong: "#ffffff",
          subtle: "#4a4a4a",
          focus: "#ffd94d",
        },
        accent: {
          default: "#0057ff",
          hover: "#7fb0ff",
          active: "#003fb3",
          subtle: "#00246b",
          on: "#ffffff",
        },
        status: {
          success: "#00c000",
          warning: "#ffd94d",
          danger: "#ff4d4d",
          info: "#7fb0ff",
        },
        editor: {
          bg: "#000000",
          gutterBg: "#000000",
          gutterText: "#c0c0c0",
          activeLine: "#141414",
          selection: "#003fb3",
          selectionInactive: "#262626",
          cursor: "#ffd94d",
          matchingBracket: "#ffd94d",
          indentGuide: "#262626",
          findMatch: "#a06000",
          findMatchCurrent: "#ffd94d",
        },
        canvas: {
          bg: "#000000",
          grid: "#1c1c1c",
          gridMajor: "#262626",
          nodeBg: "#0a0a0a",
          nodeBorder: "#c0c0c0",
          nodeSelected: "#ffd94d",
          nodeText: "#ffffff",
          edge: "#e8e8e8",
          edgeLabel: "#ffffff",
          groupBg: "#141414",
          groupBorder: "#8a8a8a",
          groupLabel: "#ffd94d",
        },
        diff: {
          added: "#005c00",
          removed: "#990000",
          modified: "#704300",
        },
      },
      component: {
        tab: {
          bg: "#000000",
          activeBg: "#141414",
          activeText: "#ffffff",
          inactiveText: "#c0c0c0",
          hoverBg: "#1c1c1c",
        },
        sidebar: { bg: "#000000", border: "#4a4a4a", text: "#e8e8e8" },
        toolbar: { bg: "#0a0a0a", border: "#4a4a4a", text: "#e8e8e8" },
        statusbar: { bg: "#000000", text: "#c0c0c0" },
        modal: { overlay: "#000000b3", bg: "#0a0a0a", border: "#ffffff" },
        mcp: {
          toolRead: "#7fb0ff",
          toolWrite: "#ffd94d",
          toolPending: "#bfa0ff",
          badge: "#0057ff",
        },
      },
    },
    typography: TYPOGRAPHY,
    radius: RADIUS,
    shadow: SHADOW,
    density: DENSITY,
    icon: ICON,
  },
};

/** All built-in themes, keyed by file name. */
export const THEMES = {
  dark: DARK,
  light: LIGHT,
  "high-contrast": HIGH_CONTRAST,
};

/** Serialises a theme exactly the way the committed files are written. */
export function serializeTheme(theme) {
  return `${JSON.stringify(theme, null, 2)}\n`;
}
