/**
 * The CodeMirror theme of MCPad.
 *
 * The theme only refers to CSS custom properties. Nothing here contains a
 * colour value, so switching the application theme repaints the editor without
 * recreating the editor state.
 */

import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";

/** The syntax highlighting of MCPad. */
export const mcpadHighlightStyle = HighlightStyle.define([
  { tag: tags.heading, color: "var(--mcpad-color-semantic-text-primary)", fontWeight: "600" },
  { tag: tags.heading1, color: "var(--mcpad-color-semantic-text-primary)", fontWeight: "700" },
  { tag: tags.heading2, color: "var(--mcpad-color-semantic-text-primary)", fontWeight: "600" },
  { tag: tags.emphasis, fontStyle: "italic" },
  { tag: tags.strong, fontWeight: "700" },
  { tag: tags.link, color: "var(--mcpad-color-semantic-text-link)" },
  { tag: tags.url, color: "var(--mcpad-color-semantic-text-link)" },
  { tag: tags.monospace, color: "var(--mcpad-color-semantic-text-code)" },
  { tag: tags.quote, color: "var(--mcpad-color-semantic-text-secondary)", fontStyle: "italic" },
  { tag: tags.list, color: "var(--mcpad-color-semantic-text-muted)" },
  { tag: tags.comment, color: "var(--mcpad-color-semantic-text-muted)", fontStyle: "italic" },
  { tag: tags.keyword, color: "var(--mcpad-color-semantic-accent-default)" },
  { tag: tags.string, color: "var(--mcpad-color-semantic-status-success)" },
  { tag: tags.number, color: "var(--mcpad-color-semantic-status-warning)" },
  { tag: tags.bool, color: "var(--mcpad-color-semantic-status-info)" },
  { tag: tags.tagName, color: "var(--mcpad-color-semantic-status-danger)" },
]);

/** The editor theme, driven by the design tokens of the active theme. */
export const mcpadEditorTheme = EditorView.theme(
  {
    "&": {
      color: "var(--mcpad-editor-fg)",
      backgroundColor: "var(--mcpad-editor-bg)",
      height: "100%",
      fontSize: "var(--mcpad-typography-font-size-md)",
    },
    "&.cm-focused": {
      outline: "none",
    },
    ".cm-content": {
      caretColor: "var(--mcpad-editor-cursor)",
      fontFamily: "var(--mcpad-typography-font-family-mono)",
      lineHeight: "var(--mcpad-typography-line-height-normal)",
      padding: "var(--mcpad-spacing-md, 8px) 0",
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "var(--mcpad-editor-cursor)",
      borderLeftWidth: "2px",
    },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
      backgroundColor: "var(--mcpad-editor-selection)",
    },
    ".cm-gutters": {
      backgroundColor: "var(--mcpad-editor-gutter)",
      color: "var(--mcpad-color-semantic-editor-gutter-text)",
      border: "none",
      fontFamily: "var(--mcpad-typography-font-family-mono)",
    },
    ".cm-activeLine": {
      backgroundColor: "var(--mcpad-color-semantic-editor-active-line)",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "var(--mcpad-color-semantic-editor-active-line)",
    },
    ".cm-matchingBracket, .cm-nonmatchingBracket": {
      backgroundColor: "var(--mcpad-color-semantic-editor-matching-bracket)",
      outline: "none",
    },
    ".cm-selectionMatch": {
      backgroundColor: "var(--mcpad-color-semantic-editor-find-match)",
    },
    ".cm-panels": {
      backgroundColor: "var(--mcpad-bg-raised)",
      color: "var(--mcpad-fg)",
    },
    ".cm-searchMatch": {
      backgroundColor: "var(--mcpad-color-semantic-editor-find-match)",
    },
    ".cm-searchMatch.cm-searchMatch-selected": {
      backgroundColor: "var(--mcpad-color-semantic-editor-find-match-current)",
    },
  },
  { dark: true },
);

/** The highlighter extension, built from the token driven style. */
export const mcpadHighlighting = syntaxHighlighting(mcpadHighlightStyle);
