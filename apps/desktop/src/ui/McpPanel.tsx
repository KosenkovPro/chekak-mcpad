import { useState } from "react";

import type { McpStatus, PendingWrite } from "../bridge/index.ts";
import { Icon } from "./Icon.tsx";

interface McpPanelProps {
  /** The state of the server. */
  status: McpStatus;
  /** The changes that wait for the answer of the user. */
  pending: PendingWrite[];
  /** Answers a change. */
  onResolve: (id: string, allow: boolean) => void;
  /** Closes the panel. */
  onClose: () => void;
}

/** The name of a tool, in the language of the interface. */
function toolTitle(tool: string): string {
  switch (tool) {
    case "write_file":
      return "Запись файла";
    case "delete_file":
      return "Удаление файла";
    default:
      return tool;
  }
}

/**
 * The panel of the MCP server.
 *
 * It shows where the clients have to connect and every change an AI agent
 * asks for. Nothing is written to the vault before the user agrees here.
 */
export function McpPanel({ status, pending, onResolve, onClose }: McpPanelProps) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    if (!status.url || !status.token) return;
    try {
      await navigator.clipboard.writeText(
        `${status.url}\nAuthorization: Bearer ${status.token}`,
      );
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      // The browser refused the clipboard, the text stays visible anyway.
    }
  };

  return (
    <aside className="app-mcp" role="dialog" aria-label="MCP-сервер">
      <header className="app-mcp-header">
        <span>MCP-сервер</span>
        <button type="button" onClick={onClose} aria-label="Закрыть панель MCP">
          ×
        </button>
      </header>

      {status.enabled ? (
        <div className="app-mcp-status">
          <div className="app-mcp-row">
            <span className="app-mcp-key">Адрес</span>
            <code>{status.url}</code>
          </div>
          <div className="app-mcp-row">
            <span className="app-mcp-key">Токен</span>
            <code className="app-mcp-token">{status.token}</code>
          </div>
          <div className="app-mcp-row">
            <span className="app-mcp-key">Режим</span>
            <span>только чтение, запись — после подтверждения</span>
          </div>
          <button type="button" className="app-mcp-copy" onClick={() => void copy()}>
            <Icon name="copy" />
            {copied ? "Скопировано" : "Скопировать настройку"}
          </button>
          <p className="app-mcp-hint">
            Сервер слушает только 127.0.0.1. В настройках клиента укажите адрес и
            заголовок <code>Authorization: Bearer &lt;токен&gt;</code>.
          </p>
        </div>
      ) : (
        <div className="app-mcp-status">
          <p className="app-mcp-hint">
            MCP-сервер работает только в настольном приложении.
          </p>
        </div>
      )}

      <div className="app-mcp-pending">
        <div className="app-mcp-title">
          Запросы на изменение
          {pending.length > 0 && <span className="app-mcp-badge">{pending.length}</span>}
        </div>

        {pending.length === 0 ? (
          <p className="app-mcp-hint">Пока никто ничего не просит изменить.</p>
        ) : (
          pending.map((request) => (
            <div key={request.id} className="app-mcp-card">
              <div className="app-mcp-card-title">{toolTitle(request.tool)}</div>
              <div className="app-mcp-card-path">{request.path}</div>
              <div className="app-mcp-card-summary">{request.summary}</div>
              {request.preview && (
                <pre className="app-mcp-preview">{request.preview}</pre>
              )}
              <div className="app-mcp-card-actions">
                <button
                  type="button"
                  className="app-mcp-allow"
                  onClick={() => onResolve(request.id, true)}
                >
                  Разрешить
                </button>
                <button type="button" onClick={() => onResolve(request.id, false)}>
                  Отклонить
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </aside>
  );
}
