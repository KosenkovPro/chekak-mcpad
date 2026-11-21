/**
 * Access to the Rust core.
 *
 * The frontend never touches the file system directly. It calls the bridge,
 * which is either the Tauri implementation (real vault, real files) or the
 * browser implementation used for UI work without a compiled backend.
 */

import type { ThemeDocument } from "../theme/types.ts";

export type FileKind = "note" | "canvas" | "other";

export interface VaultFile {
  /** Path relative to the vault root. */
  path: string;
  /** File name including the extension. */
  name: string;
  /** Kind of the file. */
  kind: FileKind;
  /** Size in bytes. */
  size: number;
}

export interface OpenDocument {
  /** Path relative to the vault root. */
  path: string;
  /** Content of the file. */
  text: string;
  /** Encoding label, for example `UTF-8`. */
  encoding: string;
  /** Line ending label, `LF` or `CRLF`. */
  lineEnding: string;
  /** Whether the file was opened in lazy mode because it is huge. */
  large: boolean;
}

/** One open tab, mirrored from the session of the core. */
export interface TabState {
  /** Path relative to the vault root. */
  path: string;
  /** Caret position as line and column. */
  caret?: [number, number] | null;
  /** Vertical scroll offset. */
  scroll?: number;
  /** Whether the tab is pinned. */
  pinned?: boolean;
}

/** The persisted editor session. */
export interface SessionState {
  /** Open tabs, in the order they were opened. */
  tabs: TabState[];
  /** Path of the active tab. */
  active?: string | null;
  /** Splits of the editor layout. */
  splits?: string[];
}

/** The state of the MCP server of the application. */
export interface McpStatus {
  /** Whether the server listens. */
  enabled: boolean;
  /** The address of the protocol. */
  url: string | null;
  /** The port of the server. */
  port: number | null;
  /** The token a client has to send. */
  token: string | null;
  /** The number of the changes that wait for the answer of the user. */
  pending: number;
}

/** A change of an AI agent that waits for the answer of the user. */
export interface PendingWrite {
  /** The identifier of the request. */
  id: string;
  /** The tool that asks for the change. */
  tool: string;
  /** Path of the file, relative to the vault. */
  path: string;
  /** A short description of the change. */
  summary: string;
  /** The beginning of the new content. */
  preview: string | null;
}

export interface Bridge {
  /** Which implementation is active. */
  readonly kind: "tauri" | "browser";
  /** The vault the application was started with, if there is one. */
  initialVault(): Promise<string | null>;
  /** Opens a vault directory. */
  openVault(root: string): Promise<void>;
  /** Lists the files of the open vault. */
  listFiles(): Promise<VaultFile[]>;
  /** Reads a file of the open vault. */
  readFile(path: string): Promise<OpenDocument>;
  /** Reads a file of the open vault completely, no matter how big it is. */
  readFileFull(path: string): Promise<OpenDocument>;
  /** Writes a file of the open vault, atomically. */
  writeFile(
    path: string,
    text: string,
    encoding?: string,
    lineEnding?: string,
    complete?: boolean,
  ): Promise<void>;
  /** Converts a file to another encoding or line ending, in place. */
  convertFile(path: string, encoding?: string, lineEnding?: string): Promise<void>;
  /** Reads the persisted session. */
  loadSession(): Promise<SessionState>;
  /** Writes the persisted session. */
  saveSession(session: SessionState): Promise<void>;
  /** Stores the unsaved content of a file. */
  saveDraft(path: string, text: string): Promise<void>;
  /** Reads the unsaved content of a file. */
  loadDraft(path: string): Promise<string | null>;
  /** Removes the unsaved content of a file. */
  discardDraft(path: string): Promise<void>;
  /** Names of the available themes. */
  listThemes(): Promise<string[]>;
  /** Loads a theme by name. */
  loadTheme(name: string): Promise<ThemeDocument>;
  /** The state of the MCP server. */
  mcpStatus(): Promise<McpStatus>;
  /** The changes of the AI agents that wait for the answer of the user. */
  mcpPendingWrites(): Promise<PendingWrite[]>;
  /** Answers a change of an AI agent. */
  mcpResolveWrite(id: string, allow: boolean): Promise<boolean>;
  /** Watches for the changes that wait for the answer of the user. */
  onMcpPending(handler: (request: PendingWrite) => void): () => void;
  /** Watches for the changes that were answered. */
  onMcpSettled(handler: (id: string) => void): () => void;
}

/** Decides which implementation to use. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
