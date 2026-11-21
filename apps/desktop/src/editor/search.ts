/**
 * The search engine of the editor.
 *
 * The module knows nothing about the editor itself: it compiles a pattern,
 * finds the matches inside a text and builds the replacement. Keeping it pure
 * makes the behaviour testable without a window and keeps the editor
 * component small.
 */

/** How a pattern is interpreted. */
export interface SearchOptions {
  /** The pattern is a regular expression. */
  regexp: boolean;
  /** The pattern is matched with the case taken into account. */
  caseSensitive: boolean;
  /** Only whole words are matched. */
  wholeWord: boolean;
}

/** One match inside a text. */
export interface SearchHit {
  /** Start of the match. */
  from: number;
  /** End of the match, exclusive. */
  to: number;
  /** The matched text. */
  text: string;
  /** The captured groups of the match. */
  groups: (string | undefined)[];
}

/** The default options of the search panel. */
export const DEFAULT_SEARCH_OPTIONS: SearchOptions = {
  regexp: false,
  caseSensitive: false,
  wholeWord: false,
};

/** The biggest number of matches that are counted and highlighted. */
export const MATCH_LIMIT = 10000;

/** Escapes the characters that have a meaning inside a regular expression. */
export function escapePattern(pattern: string): string {
  return pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Compiles a pattern into a regular expression.
 *
 * Returns `null` when the pattern is empty or is not a valid regular
 * expression, so the interface can tell the user that the expression is
 * broken instead of failing silently.
 */
export function compilePattern(
  pattern: string,
  options: SearchOptions,
): RegExp | null {
  if (!pattern) return null;

  const flags = `g${options.caseSensitive ? "" : "i"}`;
  try {
    if (options.regexp) {
      return new RegExp(pattern, flags);
    }

    const escaped = escapePattern(pattern);
    if (options.wholeWord) {
      // The word boundary of JavaScript only knows latin letters, so the
      // lookaround is written by hand and works with cyrillic as well.
      return new RegExp(
        `(?<![\\p{L}\\p{N}_])${escaped}(?![\\p{L}\\p{N}_])`,
        `${flags}u`,
      );
    }
    return new RegExp(escaped, flags);
  } catch {
    return null;
  }
}

/** Finds every match of the pattern inside the text. */
export function findMatches(
  text: string,
  pattern: RegExp,
  limit = MATCH_LIMIT,
): SearchHit[] {
  const hits: SearchHit[] = [];
  const scanner = new RegExp(
    pattern.source,
    pattern.flags.includes("g") ? pattern.flags : `${pattern.flags}g`,
  );

  let match = scanner.exec(text);
  while (match !== null) {
    hits.push({
      from: match.index,
      to: match.index + match[0].length,
      text: match[0],
      groups: match.slice(1),
    });
    if (hits.length >= limit) break;

    // An expression that matches an empty string would loop forever.
    if (match[0].length === 0) scanner.lastIndex += 1;
    match = scanner.exec(text);
  }

  return hits;
}

/**
 * Finds the match that follows the given position.
 *
 * The search wraps around: when nothing follows the position, the first match
 * of the document is used, exactly like in a normal text editor.
 */
export function findNextHit(
  hits: SearchHit[],
  from: number,
  backwards = false,
): SearchHit | null {
  if (!hits.length) return null;

  if (backwards) {
    for (let index = hits.length - 1; index >= 0; index -= 1) {
      if (hits[index].to <= from) return hits[index];
    }
    return hits[hits.length - 1];
  }

  for (const hit of hits) {
    if (hit.from >= from) return hit;
  }
  return hits[0];
}

/** Expands the group references of a replacement, like `$1` or `$&`. */
export function expandReplacement(replacement: string, hit: SearchHit): string {
  return replacement.replace(/\$(\$|&|\d{1,2})/g, (_whole, reference: string) => {
    if (reference === "$") return "$";
    if (reference === "&") return hit.text;
    const index = Number(reference) - 1;
    return hit.groups[index] ?? "";
  });
}

/** The change that replaces every match at once. */
export interface ReplaceChange {
  /** Start of the replaced range. */
  from: number;
  /** End of the replaced range, exclusive. */
  to: number;
  /** The new content of the range. */
  insert: string;
}

/**
 * Builds the change that replaces every match at once.
 *
 * The change covers the range from the first match to the end of the last one,
 * so the text around the matches stays untouched. A regular expression keeps
 * its group references, a plain search replaces the matches literally.
 */
export function buildReplaceAll(
  text: string,
  hits: SearchHit[],
  replacement: string,
  regexp: boolean,
): ReplaceChange | null {
  if (!hits.length) return null;

  const from = hits[0].from;
  let insert = "";
  let cursor = from;

  for (const hit of hits) {
    insert += text.slice(cursor, hit.from);
    insert += regexp ? expandReplacement(replacement, hit) : replacement;
    cursor = hit.to;
  }

  return { from, to: cursor, insert };
}
