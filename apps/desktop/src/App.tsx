import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { bridge } from "./bridge/index.ts";
import type {
  McpStatus,
  OpenDocument,
  PendingWrite,
  SessionState,
  VaultFile,
} from "./bridge/index.ts";
import { Editor } from "./editor/Editor.tsx";
import type { CursorPosition, EditorHandle } from "./editor/Editor.tsx";
import { Icon } from "./ui/Icon.tsx";
import { McpPanel } from "./ui/McpPanel.tsx";
import { applyTheme } from "./theme/apply.ts";

const AUTOSAVE_DELAY_MS = 1500;
const DRAFT_DELAY_MS = 600;

const ENCODINGS = [
  "UTF-8",
  "UTF-8 with BOM",
  "UTF-16 LE",
  "UTF-16 BE",
  "Windows-1251",
  "KOI8-R",
];

const LINE_ENDINGS = ["LF", "CRLF"];

/** The icon that stands for a kind of file. */
function iconForKind(kind: string): string {
  switch (kind) {
    case "canvas":
      return "canvas";
    case "note":
      return "note";
    default:
      return "file";
  }
}

/** Groups the files of the vault by their directory. */
function buildTree(files: VaultFile[]): { directory: string; files: VaultFile[] }[] {
  const groups = new Map<string, VaultFile[]>();
  for (const file of files) {
    const directory = file.path.includes("/")
      ? file.path.slice(0, file.path.lastIndexOf("/"))
      : "";
    const bucket = groups.get(directory) ?? [];
    bucket.push(file);
    groups.set(directory, bucket);
  }
  return [...groups.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([directory, entries]) => ({
      directory,
      files: entries.sort((a, b) => a.name.localeCompare(b.name)),
    }));
}

/** The application shell of MCPad. */
export function App() {
  const [vaultPath, setVaultPath] = useState("");
  const [files, setFiles] = useState<VaultFile[]>([]);
  const [openPaths, setOpenPaths] = useState<string[]>([]);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [splitPath, setSplitPath] = useState<string | null>(null);
  const [documents, setDocuments] = useState<Record<string, OpenDocument>>({});
  const [dirty, setDirty] = useState<Record<string, boolean>>({});
  const [cursor, setCursor] = useState<CursorPosition>({
    line: 1,
    column: 1,
    selected: 0,
    lines: 0,
    chars: 0,
  });
  const [themeNames, setThemeNames] = useState<string[]>([]);
  const [themeName, setThemeName] = useState("dark");
  const [status, setStatus] = useState("Готово");
  const [gotoLine, setGotoLine] = useState("");
  const [mcpStatus, setMcpStatus] = useState<McpStatus>({
    enabled: false,
    url: null,
    port: null,
    token: null,
    pending: 0,
  });
  const [mcpWrites, setMcpWrites] = useState<PendingWrite[]>([]);
  const [mcpOpen, setMcpOpen] = useState(false);

  const saveTimers = useRef<Record<string, number>>({});
  const draftTimers = useRef<Record<string, number>>({});
  const restored = useRef(false);
  const editor = useRef<EditorHandle | null>(null);
  const gotoInput = useRef<HTMLInputElement | null>(null);

  const tree = useMemo(() => buildTree(files), [files]);

  const refreshFiles = useCallback(async () => {
    try {
      setFiles(await bridge.listFiles());
    } catch (error) {
      setStatus(`Не удалось прочитать Vault: ${(error as Error).message}`);
    }
  }, []);

  // Startup: theme, files and the session of the previous run.
  useEffect(() => {
    void (async () => {
      try {
        const themes = await bridge.listThemes();
        setThemeNames(themes);
        applyTheme(await bridge.loadTheme(themeName));

        // The vault of the previous run, or the one given on the command line,
        // is already opened by the shell. The interface only has to notice it.
        const initial = await bridge.initialVault();
        if (initial) {
          await bridge.openVault(initial);
          setVaultPath(initial);
        }

        await refreshFiles();

        let session: SessionState = { tabs: [], active: null, splits: [] };
        try {
          session = await bridge.loadSession();
        } catch {
          // A vault that is not open yet simply has no session to restore.
        }

        for (const tab of session.tabs) {
          try {
            const document = await bridge.readFile(tab.path);
            const draft = await bridge.loadDraft(tab.path);
            setDocuments((current) => ({
              ...current,
              [tab.path]: draft ? { ...document, text: draft } : document,
            }));
            if (draft) {
              setDirty((current) => ({ ...current, [tab.path]: true }));
            }
          } catch {
            // A file that vanished since the last run is simply skipped.
          }
        }
        setOpenPaths(session.tabs.map((tab) => tab.path));
        setActivePath(session.active ?? session.tabs[0]?.path ?? null);
        restored.current = true;
      } catch (error) {
        setStatus(`Ошибка запуска: ${(error as Error).message}`);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The session is written whenever the layout changes.
  useEffect(() => {
    if (!restored.current) return;
    const session: SessionState = {
      tabs: openPaths.map((path) => ({ path })),
      active: activePath,
      splits: splitPath ? [splitPath] : [],
    };
    void bridge.saveSession(session).catch(() => undefined);
  }, [openPaths, activePath, splitPath]);

  // The MCP server of the application and the changes that wait for the user.
  useEffect(() => {
    let alive = true;

    void (async () => {
      try {
        const [current, waiting] = await Promise.all([
          bridge.mcpStatus(),
          bridge.mcpPendingWrites(),
        ]);
        if (!alive) return;
        setMcpStatus(current);
        setMcpWrites(waiting);
      } catch {
        // A shell without the MCP server simply has nothing to show.
      }
    })();

    const stopPending = bridge.onMcpPending((request) => {
      setMcpWrites((current) =>
        current.some((item) => item.id === request.id) ? current : [...current, request],
      );
      setMcpStatus((current) => ({ ...current, pending: current.pending + 1 }));
      setMcpOpen(true);
    });

    const stopSettled = bridge.onMcpSettled((id) => {
      setMcpWrites((current) => current.filter((item) => item.id !== id));
      setMcpStatus((current) => ({
        ...current,
        pending: Math.max(0, current.pending - 1),
      }));
    });

    return () => {
      alive = false;
      stopPending();
      stopSettled();
    };
  }, []);

  const openFile = useCallback(async (path: string) => {
    try {
      const document = await bridge.readFile(path);
      setDocuments((current) => ({ ...current, [path]: document }));
      setOpenPaths((current) => (current.includes(path) ? current : [...current, path]));
      setActivePath(path);
      setStatus(`${path} · ${document.encoding} · ${document.lineEnding}`);
    } catch (error) {
      setStatus(`Не удалось открыть: ${(error as Error).message}`);
    }
  }, []);

  const closeFile = useCallback(
    (path: string) => {
      setOpenPaths((current) => current.filter((item) => item !== path));
      setDocuments((current) => {
        const next = { ...current };
        delete next[path];
        return next;
      });
      setDirty((current) => {
        const next = { ...current };
        delete next[path];
        return next;
      });
      setActivePath((current) => (current === path ? null : current));
      if (splitPath === path) setSplitPath(null);
      void bridge.discardDraft(path).catch(() => undefined);
    },
    [splitPath],
  );

  /** Saves a file and drops its draft. */
  const persist = useCallback(async (path: string, text: string) => {
    const document = documents[path];
    try {
      await bridge.writeFile(
        path,
        text,
        document?.encoding,
        document?.lineEnding,
        !document?.large,
      );
      await bridge.discardDraft(path);
      setDirty((current) => ({ ...current, [path]: false }));
      setStatus(`Сохранено: ${path}`);
    } catch (error) {
      setStatus(`Не удалось сохранить: ${(error as Error).message}`);
    }
  }, [documents]);

  /** Loads a huge file completely, so it can be edited without a loss. */
  const loadFull = useCallback(async (path: string) => {
    try {
      const document = await bridge.readFileFull(path);
      setDocuments((current) => ({ ...current, [path]: document }));
      setStatus(`Файл загружен целиком: ${path}`);
    } catch (error) {
      setStatus(`Не удалось загрузить файл: ${(error as Error).message}`);
    }
  }, []);

  /** Moves the caret to the line the user typed in the status bar. */
  const jumpToLine = useCallback(() => {
    const line = Number.parseInt(gotoLine, 10);
    if (Number.isNaN(line)) return;
    editor.current?.gotoLine(line);
    setGotoLine("");
  }, [gotoLine]);

  /** Called on every keystroke: schedules the autosave and keeps a draft. */
  const change = useCallback(
    (path: string, text: string) => {
      setDocuments((current) => ({ ...current, [path]: { ...current[path], text } }));
      setDirty((current) => ({ ...current, [path]: true }));

      window.clearTimeout(saveTimers.current[path]);
      saveTimers.current[path] = window.setTimeout(() => {
        void persist(path, text);
      }, AUTOSAVE_DELAY_MS);

      window.clearTimeout(draftTimers.current[path]);
      draftTimers.current[path] = window.setTimeout(() => {
        void bridge.saveDraft(path, text).catch(() => undefined);
      }, DRAFT_DELAY_MS);
    },
    [persist],
  );

  const selectTheme = useCallback(async (name: string) => {
    try {
      applyTheme(await bridge.loadTheme(name));
      setThemeName(name);
      setStatus(`Тема: ${name}`);
    } catch (error) {
      setStatus(`Тема не найдена: ${(error as Error).message}`);
    }
  }, []);

  const convert = useCallback(
    async (path: string, encoding?: string, lineEnding?: string) => {
      try {
        await bridge.convertFile(path, encoding, lineEnding);
        const document = await bridge.readFile(path);
        setDocuments((current) => ({ ...current, [path]: document }));
        setStatus(`Конвертировано: ${path} → ${document.encoding} / ${document.lineEnding}`);
      } catch (error) {
        setStatus(`Не удалось конвертировать: ${(error as Error).message}`);
      }
    },
    [],
  );

  const openVault = useCallback(async () => {
    if (!vaultPath) return;
    try {
      await bridge.openVault(vaultPath);
      await refreshFiles();
      setStatus(`Vault открыт: ${vaultPath}`);
    } catch (error) {
      setStatus(`Не удалось открыть Vault: ${(error as Error).message}`);
    }
  }, [vaultPath, refreshFiles]);

  /** Answers a change an AI agent asks for. */
  const resolveWrite = useCallback(
    async (id: string, allow: boolean) => {
      const request = mcpWrites.find((item) => item.id === id);
      try {
        const answered = await bridge.mcpResolveWrite(id, allow);
        if (!answered) {
          setMcpWrites((current) => current.filter((item) => item.id !== id));
          setStatus("Запрос уже не актуален");
          return;
        }

        if (allow && request) {
          await refreshFiles();
          // The file of the agent may be open in a tab: it has to show the new text.
          if (openPaths.includes(request.path) && !dirty[request.path]) {
            try {
              const document = await bridge.readFile(request.path);
              setDocuments((current) => ({ ...current, [request.path]: document }));
            } catch {
              // The file may be gone already.
            }
          }
        }

        setStatus(
          allow
            ? `Изменение разрешено: ${request?.path ?? id}`
            : `Изменение отклонено: ${request?.path ?? id}`,
        );
      } catch (error) {
        setStatus(`Не удалось ответить на запрос: ${(error as Error).message}`);
      }
    },
    [dirty, mcpWrites, openPaths, refreshFiles],
  );

  const active = activePath ? documents[activePath] : undefined;
  const split = splitPath ? documents[splitPath] : undefined;

  return (
    <div className="app">
      <header className="app-toolbar">
        <span className="app-brand">MCPad</span>
        <div className="app-vault">
          <input
            type="text"
            value={vaultPath}
            placeholder="C:\путь\к\vault"
            onChange={(event) => setVaultPath(event.target.value)}
          />
          <button type="button" onClick={() => void openVault()}>
            Открыть Vault
          </button>
          <button
            type="button"
            onClick={() =>
              setSplitPath((current) => (current ? null : (activePath ?? null)))
            }
            disabled={!activePath}
          >
            {splitPath ? "Убрать сплит" : "Сплит"}
          </button>
          <button
            type="button"
            onClick={() => editor.current?.openSearch()}
            disabled={!activePath}
          >
            Поиск
          </button>
          <button
            type="button"
            className={mcpOpen ? "is-active" : ""}
            onClick={() => setMcpOpen((current) => !current)}
            title="MCP-сервер"
          >
            <Icon name="robot" />
            MCP
            {mcpWrites.length > 0 && (
              <span className="app-mcp-badge">{mcpWrites.length}</span>
            )}
          </button>
        </div>
        <div className="app-theme">
          {themeNames.map((name) => (
            <button
              key={name}
              type="button"
              className={name === themeName ? "is-active" : ""}
              onClick={() => void selectTheme(name)}
            >
              {name}
            </button>
          ))}
        </div>
      </header>

      <div className="app-body">
        <aside className="app-sidebar">
          <div className="app-sidebar-title">Vault</div>
          <ul className="app-tree">
            {tree.map((group) => (
              <li key={group.directory || "root"}>
                {group.directory && (
                  <div className="app-tree-dir">
                    <Icon name="folder" />
                    {group.directory}/
                  </div>
                )}
                <ul className="app-tree-files">
                  {group.files.map((file) => (
                    <li key={file.path}>
                      <button
                        type="button"
                        className={file.path === activePath ? "is-active" : ""}
                        onClick={() => void openFile(file.path)}
                        title={file.path}
                      >
                        <Icon name={iconForKind(file.kind)} title={file.kind} />
                        {file.name}
                      </button>
                    </li>
                  ))}
                </ul>
              </li>
            ))}
          </ul>
        </aside>

        <main className="app-main">
          <nav className="app-tabs">
            {openPaths.map((path) => (
              <span
                key={path}
                className={`app-tab${path === activePath ? " is-active" : ""}`}
              >
                <button
                  type="button"
                  onClick={() => setActivePath(path)}
                  title={path}
                >
                  {dirty[path] ? "• " : ""}
                  {path.split("/").pop()}
                </button>
                <button
                  type="button"
                  className="app-tab-close"
                  onClick={() => closeFile(path)}
                  aria-label={`Закрыть ${path}`}
                >
                  ×
                </button>
              </span>
            ))}
          </nav>

          <section className="app-editor">
            {active ? (
              <div className={split ? "app-panes" : "app-pane"}>
                <div className="app-pane">
                  {active.large && (
                    <div className="app-large-banner">
                      <span>
                        Файл большой: показан только начало. Редактирование отключено, чтобы
                        не потерять остальное.
                      </span>
                      <button type="button" onClick={() => void loadFull(active.path)}>
                        Загрузить целиком
                      </button>
                    </div>
                  )}
                  <Editor
                    ref={editor}
                    value={active.text}
                    readOnly={active.large}
                    onChange={(text) => change(active.path, text)}
                    onCursor={setCursor}
                    onRequestGotoLine={() => gotoInput.current?.focus()}
                  />
                </div>
                {split && (
                  <div className="app-pane">
                    <div className="app-pane-title">{splitPath}</div>
                    <Editor value={split.text} readOnly />
                  </div>
                )}
              </div>
            ) : (
              <div className="app-empty">Выберите файл в дереве слева</div>
            )}
          </section>
        </main>
      </div>

      {mcpOpen && (
        <McpPanel
          status={mcpStatus}
          pending={mcpWrites}
          onResolve={(id, allow) => void resolveWrite(id, allow)}
          onClose={() => setMcpOpen(false)}
        />
      )}

      <footer className="app-statusbar">
        <span>{status}</span>
        <span className="app-status-right">
          {active && (
            <>
              <label className="app-goto">
                Строка
                <input
                  ref={gotoInput}
                  type="number"
                  min={1}
                  value={gotoLine}
                  placeholder={String(cursor.line)}
                  onChange={(event) => setGotoLine(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      jumpToLine();
                    }
                  }}
                />
              </label>
              <label>
                Кодировка
                <select
                  value={active.encoding}
                  onChange={(event) => void convert(active.path, event.target.value)}
                >
                  {ENCODINGS.map((label) => (
                    <option key={label} value={label}>
                      {label}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Переводы строк
                <select
                  value={active.lineEnding}
                  onChange={(event) => void convert(active.path, undefined, event.target.value)}
                >
                  {LINE_ENDINGS.map((label) => (
                    <option key={label} value={label}>
                      {label}
                    </option>
                  ))}
                </select>
              </label>
              <span>
                Стр {cursor.line}, кол {cursor.column}
              </span>
              {cursor.selected > 0 && <span>выделено {cursor.selected}</span>}
              <span>
                {cursor.lines} строк, {cursor.chars} символов
              </span>
              {dirty[active.path] && <span className="app-dirty">изменено</span>}
            </>
          )}
        </span>
      </footer>
    </div>
  );
}
