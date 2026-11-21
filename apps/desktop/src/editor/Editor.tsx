import { forwardRef, useCallback, useEffect, useImperativeHandle, useRef, useState } from "react";
import { Compartment, EditorState } from "@codemirror/state";
import {
  EditorView,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import {
  defaultKeymap,
  history,
  historyKeymap,
  indentWithTab,
} from "@codemirror/commands";
import {
  closeBrackets,
  closeBracketsKeymap,
  completionKeymap,
} from "@codemirror/autocomplete";
import {
  bracketMatching,
  foldGutter,
  foldKeymap,
  indentOnInput,
} from "@codemirror/language";
import { highlightSelectionMatches, selectNextOccurrence } from "@codemirror/search";
import { markdown } from "@codemirror/lang-markdown";

import { mcpadEditorTheme, mcpadHighlighting } from "./editorTheme.ts";
import {
  DEFAULT_SEARCH_OPTIONS,
  buildReplaceAll,
  compilePattern,
  expandReplacement,
  findMatches,
  findNextHit,
} from "./search.ts";
import type { SearchHit, SearchOptions } from "./search.ts";

export interface CursorPosition {
  /** Line of the caret, counted from one. */
  line: number;
  /** Column of the caret, counted from one. */
  column: number;
  /** Number of the selected characters. */
  selected: number;
  /** Number of the lines of the document. */
  lines: number;
  /** Number of the characters of the document. */
  chars: number;
}

/** What the shell can ask of an editor. */
export interface EditorHandle {
  /** Moves the caret to the given line. */
  gotoLine: (line: number) => void;
  /** Opens the search panel. */
  openSearch: (replace?: boolean) => void;
  /** Moves the focus into the editor. */
  focus: () => void;
}

export interface EditorProps {
  /** Content of the document. */
  value: string;
  /** Whether the editor rejects changes. */
  readOnly?: boolean;
  /** Called on every change, with the new content. */
  onChange?: (value: string) => void;
  /** Called when the caret moves. */
  onCursor?: (position: CursorPosition) => void;
  /** Whether the search panel may be opened. */
  searchable?: boolean;
  /** Called when the user asks for the line number field. */
  onRequestGotoLine?: () => void;
}

/**
 * Counts with the right Russian form of the word.
 *
 * `1 совпадение`, `2 совпадения`, `5 совпадений`.
 */
function plural(count: number, one: string, few: string, many: string): string {
  const mod10 = count % 10;
  const mod100 = count % 100;
  if (mod10 === 1 && mod100 !== 11) return `${count} ${one}`;
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 10 || mod100 >= 20)) {
    return `${count} ${few}`;
  }
  return `${count} ${many}`;
}

/**
 * The text editor.
 *
 * The component owns the CodeMirror instance and keeps it in sync with the
 * `value` property. All colours come from the design tokens, the editor itself
 * contains no colour value.
 *
 * The search panel is a part of MCPad instead of the panel of CodeMirror, so
 * it speaks the language of the application and follows the active theme.
 */
export const Editor = forwardRef<EditorHandle, EditorProps>(function Editor(
  { value, readOnly = false, onChange, onCursor, searchable = true, onRequestGotoLine },
  ref,
) {
  const host = useRef<HTMLDivElement | null>(null);
  const view = useRef<EditorView | null>(null);
  const readOnlyCompartment = useRef(new Compartment()).current;

  const onChangeRef = useRef(onChange);
  const onCursorRef = useRef(onCursor);
  const gotoRequestRef = useRef(onRequestGotoLine);
  const searchableRef = useRef(searchable);
  onChangeRef.current = onChange;
  onCursorRef.current = onCursor;
  gotoRequestRef.current = onRequestGotoLine;
  searchableRef.current = searchable;

  const [searchOpen, setSearchOpen] = useState(false);
  const [pattern, setPattern] = useState("");
  const [replacement, setReplacement] = useState("");
  const [options, setOptions] = useState<SearchOptions>(DEFAULT_SEARCH_OPTIONS);
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [broken, setBroken] = useState(false);
  const [message, setMessage] = useState("");
  const [docVersion, setDocVersion] = useState(0);

  const searchInput = useRef<HTMLInputElement | null>(null);
  const replaceInput = useRef<HTMLInputElement | null>(null);

  const patternRef = useRef(pattern);
  const replacementRef = useRef(replacement);
  const optionsRef = useRef(options);
  patternRef.current = pattern;
  replacementRef.current = replacement;
  optionsRef.current = options;

  /** Collects the matches of the current pattern in the given state. */
  const collect = useCallback((state: EditorState) => {
    const compiled = compilePattern(patternRef.current, optionsRef.current);
    if (!compiled) {
      return { hits: [] as SearchHit[], broken: patternRef.current.length > 0 };
    }
    return { hits: findMatches(state.doc.toString(), compiled), broken: false };
  }, []);

  /** Moves the caret to the next or to the previous match. */
  const runSearch = useCallback(
    (backwards: boolean) => {
      const instance = view.current;
      if (!instance) return;

      const found = collect(instance.state);
      if (found.broken || !found.hits.length) return;

      const head = instance.state.selection.main.head;
      const hit = findNextHit(found.hits, head, backwards);
      if (!hit) return;

      instance.dispatch({
        selection: { anchor: hit.from, head: hit.to },
        scrollIntoView: true,
        userEvent: "select.search",
      });
      instance.focus();
    },
    [collect],
  );

  /** Replaces the match under the caret and moves to the next one. */
  const replaceCurrent = useCallback(() => {
    const instance = view.current;
    if (!instance) return;

    const selection = instance.state.selection.main;
    const found = collect(instance.state);
    if (found.broken) return;

    const hit = found.hits.find(
      (item) => item.from === selection.from && item.to === selection.to,
    );
    if (!hit) {
      runSearch(false);
      return;
    }

    const insert = optionsRef.current.regexp
      ? expandReplacement(replacementRef.current, hit)
      : replacementRef.current;

    instance.dispatch({ changes: { from: hit.from, to: hit.to, insert } });
    setMessage("Заменено");

    const next = collect(instance.state);
    const following = findNextHit(next.hits, hit.from + insert.length);
    if (following) {
      instance.dispatch({
        selection: { anchor: following.from, head: following.to },
        scrollIntoView: true,
      });
    }
  }, [collect, runSearch]);

  /** Replaces every match at once. */
  const replaceAll = useCallback(() => {
    const instance = view.current;
    if (!instance) return;

    const found = collect(instance.state);
    if (found.broken || !found.hits.length) return;

    const change = buildReplaceAll(
      instance.state.doc.toString(),
      found.hits,
      replacementRef.current,
      optionsRef.current.regexp,
    );
    if (!change) return;

    instance.dispatch({ changes: { from: change.from, to: change.to, insert: change.insert } });
    setMessage(`Заменено: ${found.hits.length}`);
  }, [collect]);

  /** Opens the search panel and moves the focus into the right field. */
  const openSearch = useCallback((replace?: boolean) => {
    if (!searchableRef.current) return;
    setSearchOpen(true);
    window.setTimeout(() => {
      if (replace) replaceInput.current?.focus();
      else searchInput.current?.focus();
    }, 0);
  }, []);

  /** Closes the search panel and returns the focus to the document. */
  const closeSearch = useCallback(() => {
    setSearchOpen(false);
    view.current?.focus();
  }, []);

  /** Moves the caret to the given line. */
  const gotoLine = useCallback((line: number) => {
    const instance = view.current;
    if (!instance) return;

    const total = instance.state.doc.lines;
    const target = Math.min(Math.max(1, Math.trunc(line)), total);
    const position = instance.state.doc.line(target);

    instance.dispatch({ selection: { anchor: position.from }, scrollIntoView: true });
    instance.focus();
  }, []);

  useImperativeHandle(
    ref,
    () => ({
      gotoLine,
      openSearch,
      focus: () => view.current?.focus(),
    }),
    [gotoLine, openSearch],
  );

  useEffect(() => {
    if (!host.current) return;

    const state = EditorState.create({
      doc: value,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        foldGutter(),
        drawSelection(),
        dropCursor(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        bracketMatching(),
        closeBrackets(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        EditorView.lineWrapping,
        markdown(),
        mcpadEditorTheme,
        mcpadHighlighting,
        // The shortcuts of MCPad come first, so they win over the defaults.
        keymap.of([
          { key: "Mod-f", run: () => (openSearch(false), true) },
          { key: "Mod-h", run: () => (openSearch(true), true) },
          { key: "Mod-g", run: () => (gotoRequestRef.current?.(), true) },
          { key: "F3", run: () => (runSearch(false), true) },
          { key: "Shift-F3", run: () => (runSearch(true), true) },
          { key: "Mod-d", run: selectNextOccurrence },
        ]),
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...historyKeymap,
          ...foldKeymap,
          ...completionKeymap,
          indentWithTab,
        ]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onChangeRef.current?.(update.state.doc.toString());
            setDocVersion((version) => version + 1);
          }
          if (update.selectionSet || update.docChanged) {
            const head = update.state.selection.main.head;
            const line = update.state.doc.lineAt(head);
            const selection = update.state.selection.main;
            onCursorRef.current?.({
              line: line.number,
              column: head - line.from + 1,
              selected: selection.to - selection.from,
              lines: update.state.doc.lines,
              chars: update.state.doc.length,
            });
          }
        }),
        readOnlyCompartment.of(EditorState.readOnly.of(readOnly)),
      ],
    });

    const instance = new EditorView({ state, parent: host.current });
    view.current = instance;

    // The state bar needs the size of the document right away, before the
    // first move of the caret.
    onCursorRef.current?.({
      line: 1,
      column: 1,
      selected: 0,
      lines: instance.state.doc.lines,
      chars: instance.state.doc.length,
    });

    return () => {
      instance.destroy();
      view.current = null;
    };
    // The editor is created once and updated through the effect below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const instance = view.current;
    if (!instance) return;
    const current = instance.state.doc.toString();
    if (current === value) return;
    instance.dispatch({
      changes: { from: 0, to: current.length, insert: value },
    });
  }, [value]);

  useEffect(() => {
    view.current?.dispatch({
      effects: readOnlyCompartment.reconfigure(EditorState.readOnly.of(readOnly)),
    });
  }, [readOnly, readOnlyCompartment]);

  // The matches are counted a moment after the pattern changes, so typing
  // stays smooth even in a big document.
  useEffect(() => {
    if (!searchOpen) return;
    const timer = window.setTimeout(() => {
      const instance = view.current;
      if (!instance) return;
      const found = collect(instance.state);
      setHits(found.hits);
      setBroken(found.broken);
    }, 150);
    return () => window.clearTimeout(timer);
  }, [pattern, options, docVersion, searchOpen, collect]);

  return (
    <div className="editor">
      {searchOpen && (
        <div className="editor-search">
          <div className="editor-search-row">
            <input
              className="editor-search-input"
              ref={searchInput}
              value={pattern}
              placeholder="Найти"
              spellCheck={false}
              onChange={(event) => setPattern(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  runSearch(event.shiftKey);
                }
                if (event.key === "Escape") {
                  event.preventDefault();
                  closeSearch();
                }
              }}
            />
            <label className="editor-search-flag">
              <input
                type="checkbox"
                checked={options.regexp}
                onChange={(event) =>
                  setOptions((current) => ({ ...current, regexp: event.target.checked }))
                }
              />
              .*
            </label>
            <label className="editor-search-flag">
              <input
                type="checkbox"
                checked={options.caseSensitive}
                onChange={(event) =>
                  setOptions((current) => ({
                    ...current,
                    caseSensitive: event.target.checked,
                  }))
                }
              />
              Учитывать регистр
            </label>
            <label className="editor-search-flag">
              <input
                type="checkbox"
                checked={options.wholeWord}
                disabled={options.regexp}
                onChange={(event) =>
                  setOptions((current) => ({
                    ...current,
                    wholeWord: event.target.checked,
                  }))
                }
              />
              Целые слова
            </label>
            <span className="editor-search-count">
              {broken
                ? "Неверное выражение"
                : pattern
                  ? plural(hits.length, "совпадение", "совпадения", "совпадений")
                  : ""}
            </span>
            <button type="button" onClick={() => runSearch(true)} title="Найти раньше">
              ↑
            </button>
            <button type="button" onClick={() => runSearch(false)} title="Найти далее">
              ↓
            </button>
            <button type="button" onClick={closeSearch} title="Закрыть">
              ✕
            </button>
          </div>
          <div className="editor-search-row">
            <input
              className="editor-search-input"
              ref={replaceInput}
              value={replacement}
              placeholder="Заменить на"
              spellCheck={false}
              onChange={(event) => setReplacement(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  replaceCurrent();
                }
                if (event.key === "Escape") {
                  event.preventDefault();
                  closeSearch();
                }
              }}
            />
            <button type="button" onClick={replaceCurrent}>
              Заменить
            </button>
            <button type="button" onClick={replaceAll}>
              Заменить всё
            </button>
            <span className="editor-search-message">{message}</span>
          </div>
        </div>
      )}
      <div className="editor-host" ref={host} />
    </div>
  );
});
