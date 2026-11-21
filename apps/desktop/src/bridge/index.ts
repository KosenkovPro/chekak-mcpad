import { browserBridge } from "./browser.ts";
import { tauriBridge } from "./tauri.ts";
import { isTauri } from "./types.ts";
import type { Bridge } from "./types.ts";

/** The bridge that matches the current runtime. */
export const bridge: Bridge = isTauri() ? tauriBridge : browserBridge;

export * from "./types.ts";
