import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { ThemeDocument } from "../theme/types.ts";
import type {
  Bridge,
  McpStatus,
  OpenDocument,
  PendingWrite,
  SessionState,
  VaultFile,
} from "./types.ts";

/** Bridge backed by the Rust core through Tauri commands. */
export const tauriBridge: Bridge = {
  kind: "tauri",

  async initialVault(): Promise<string | null> {
    return invoke<string | null>("initial_vault");
  },

  async openVault(root: string): Promise<void> {
    await invoke("open_vault", { root });
  },

  async listFiles(): Promise<VaultFile[]> {
    return invoke<VaultFile[]>("list_files");
  },

  async readFile(path: string): Promise<OpenDocument> {
    return invoke<OpenDocument>("read_file", { path });
  },

  async readFileFull(path: string): Promise<OpenDocument> {
    return invoke<OpenDocument>("read_file_full", { path });
  },

  async writeFile(
    path: string,
    text: string,
    encoding?: string,
    lineEnding?: string,
    complete = true,
  ): Promise<void> {
    await invoke("write_file", { path, text, encoding, lineEnding, complete });
  },

  async convertFile(path: string, encoding?: string, lineEnding?: string): Promise<void> {
    await invoke("convert_file", { path, encoding, lineEnding });
  },

  async loadSession(): Promise<SessionState> {
    return invoke<SessionState>("load_session");
  },

  async saveSession(session: SessionState): Promise<void> {
    await invoke("save_session", { session });
  },

  async saveDraft(path: string, text: string): Promise<void> {
    await invoke("save_draft", { path, text });
  },

  async loadDraft(path: string): Promise<string | null> {
    return invoke<string | null>("load_draft", { path });
  },

  async discardDraft(path: string): Promise<void> {
    await invoke("discard_draft", { path });
  },

  async listThemes(): Promise<string[]> {
    return invoke<string[]>("list_themes");
  },

  async loadTheme(name: string): Promise<ThemeDocument> {
    return invoke<ThemeDocument>("load_theme", { name });
  },

  async mcpStatus(): Promise<McpStatus> {
    return invoke<McpStatus>("mcp_status");
  },

  async mcpPendingWrites(): Promise<PendingWrite[]> {
    return invoke<PendingWrite[]>("mcp_pending_writes");
  },

  async mcpResolveWrite(id: string, allow: boolean): Promise<boolean> {
    return invoke<boolean>("mcp_resolve_write", { id, allow });
  },

  onMcpPending(handler: (request: PendingWrite) => void): () => void {
    return watch("mcp://pending-write", handler);
  },

  onMcpSettled(handler: (id: string) => void): () => void {
    return watch<string>("mcp://write-settled", handler);
  },
};

/**
 * Subscribes to an event of the shell.
 *
 * The subscription is set up in the background, therefore the cleanup has to
 * wait for it: a listener that is disposed right away must not stay behind.
 */
function watch<T>(event: string, handler: (payload: T) => void): () => void {
  let stopped = false;
  let unlisten: (() => void) | undefined;

  void listen<T>(event, (message) => handler(message.payload)).then((dispose) => {
    if (stopped) {
      dispose();
    } else {
      unlisten = dispose;
    }
  });

  return () => {
    stopped = true;
    unlisten?.();
  };
}
