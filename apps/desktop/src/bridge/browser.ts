import type { ThemeDocument } from "../theme/types.ts";
import type {
  Bridge,
  McpStatus,
  OpenDocument,
  PendingWrite,
  SessionState,
  VaultFile,
} from "./types.ts";

const SAMPLE_FILES: Record<string, string> = {
  "notes/welcome.md": [
    "# Добро пожаловать в MCPad",
    "",
    "Этот файл живёт в памяти браузера. Когда ядро на Rust будет собрано,",
    "тот же интерфейс будет читать настоящий Vault.",
    "",
    "- обычный Markdown",
    "- без проприетарных форматов",
    "- с живым доступом агентов по MCP",
  ].join("\n"),
  "notes/architecture.md": [
    "# Архитектура",
    "",
    "| Слой | Технология |",
    "| --- | --- |",
    "| Ядро | Rust |",
    "| Интерфейс | Tauri v2 + CodeMirror 6 |",
    "| Холст | React Flow |",
  ].join("\n"),
  "notes/regex.md": [
    "# Проверка поиска",
    "",
    "первая строка",
    "вторая строка",
    "третья строка",
    "чётвертая строка",
  ].join("\n"),
  "board/idea.canvas": JSON.stringify(
    {
      nodes: [
        {
          id: "1",
          type: "text",
          text: "# Идея\n\nОписание узла",
          x: 0,
          y: 0,
          width: 320,
          height: 180,
        },
      ],
      edges: [],
    },
    null,
    2,
  ),
};

const drafts = new Map<string, string>();
let session: SessionState = { tabs: [], active: null, splits: [] };

/** The path of the sample file that pretends to be huge. */
const BIG_PATH = "notes/big.md";

/** The number of lines the huge sample file shows without loading it fully. */
const BIG_PREVIEW_LINES = 5000;

/** The content of the huge sample file. */
const BIG_TEXT = Array.from(
  { length: BIG_PREVIEW_LINES },
  (_, index) => `строка ${index + 1} большого файла`,
).join("\n");

/**
 * Bridge used while the desktop shell runs in a plain browser.
 *
 * It keeps the vault in memory and serves the real theme files, so the whole
 * interface can be developed and reviewed without a compiled backend.
 */
export const browserBridge: Bridge = {
  kind: "browser",

  async initialVault(): Promise<string | null> {
    return null;
  },

  async openVault(): Promise<void> {
    // Nothing to do, the browser bridge always uses the sample vault.
  },

  async listFiles(): Promise<VaultFile[]> {
    const files: VaultFile[] = Object.entries(SAMPLE_FILES).map(([path, text]) => ({
      path,
      name: path.split("/").pop() ?? path,
      kind: path.endsWith(".md") ? "note" : path.endsWith(".canvas") ? "canvas" : "other",
      size: new TextEncoder().encode(text).length,
    }));

    files.push({
      path: BIG_PATH,
      name: "big.md",
      kind: "note",
      size: new TextEncoder().encode(BIG_TEXT).length,
    });

    return files;
  },

  async readFile(path: string): Promise<OpenDocument> {
    if (path === BIG_PATH) {
      return {
        path,
        text: BIG_TEXT.split("\n").slice(0, BIG_PREVIEW_LINES).join("\n"),
        encoding: "UTF-8",
        lineEnding: "LF",
        large: true,
      };
    }

    const text = SAMPLE_FILES[path];
    if (text === undefined) {
      throw new Error(`Файл не найден: ${path}`);
    }
    return {
      path,
      text,
      encoding: "UTF-8",
      lineEnding: "LF",
      large: false,
    };
  },

  async readFileFull(path: string): Promise<OpenDocument> {
    if (path === BIG_PATH) {
      return {
        path,
        text: BIG_TEXT,
        encoding: "UTF-8",
        lineEnding: "LF",
        large: false,
      };
    }
    return browserBridge.readFile(path);
  },

  async writeFile(
    path: string,
    text: string,
    _encoding?: string,
    _lineEnding?: string,
    _complete?: boolean,
  ): Promise<void> {
    SAMPLE_FILES[path] = text;
  },

  async convertFile(): Promise<void> {
    // The browser bridge has no encoding conversion.
  },

  async loadSession(): Promise<SessionState> {
    return session;
  },

  async saveSession(next: SessionState): Promise<void> {
    session = next;
  },

  async saveDraft(path: string, text: string): Promise<void> {
    drafts.set(path, text);
  },

  async loadDraft(path: string): Promise<string | null> {
    return drafts.get(path) ?? null;
  },

  async discardDraft(path: string): Promise<void> {
    drafts.delete(path);
  },

  async listThemes(): Promise<string[]> {
    return ["dark", "light", "high-contrast"];
  },

  async loadTheme(name: string): Promise<ThemeDocument> {
    const response = await fetch(`/themes/${name}.json`);
    if (!response.ok) {
      throw new Error(`Тема не найдена: ${name}`);
    }
    return (await response.json()) as ThemeDocument;
  },

  async mcpStatus(): Promise<McpStatus> {
    // The browser has no server of its own, the panel shows that plainly.
    return {
      enabled: false,
      url: null,
      port: null,
      token: null,
      pending: 0,
    };
  },

  async mcpPendingWrites(): Promise<PendingWrite[]> {
    return [];
  },

  async mcpResolveWrite(): Promise<boolean> {
    return false;
  },

  onMcpPending(): () => void {
    return () => undefined;
  },

  onMcpSettled(): () => void {
    return () => undefined;
  },
};
