import assert from "node:assert/strict";
import { test } from "node:test";

import {
  DEFAULT_SEARCH_OPTIONS,
  buildReplaceAll,
  compilePattern,
  escapePattern,
  expandReplacement,
  findMatches,
  findNextHit,
} from "./search.ts";

/** Compiles a pattern and finds the matches in one step. */
function matches(pattern: string, text: string, options = DEFAULT_SEARCH_OPTIONS) {
  const compiled = compilePattern(pattern, options);
  assert.ok(compiled, `the pattern ${pattern} must compile`);
  return findMatches(text, compiled);
}

test("the characters of a regular expression are escaped", () => {
  assert.equal(escapePattern("a.b*c"), "a\\.b\\*c");
  assert.equal(escapePattern("(x|y)"), "\\(x\\|y\\)");
});

test("an empty pattern and a broken expression do not compile", () => {
  assert.equal(compilePattern("", DEFAULT_SEARCH_OPTIONS), null);
  assert.equal(compilePattern("(", { ...DEFAULT_SEARCH_OPTIONS, regexp: true }), null);
});

test("a plain search finds every occurrence", () => {
  const hits = matches("кот", "кот и кот, потом кот");
  assert.equal(hits.length, 3);
  assert.equal(hits[0].from, 0);
  assert.equal(hits[0].to, 3);
  assert.equal(hits[1].from, 6);
});

test("the case is ignored unless it is asked for", () => {
  assert.equal(matches("Кот", "кот КОТ").length, 2);
  assert.equal(
    matches("Кот", "кот КОТ", { ...DEFAULT_SEARCH_OPTIONS, caseSensitive: true }).length,
    0,
  );
  assert.equal(
    matches("кот", "кот КОТ", { ...DEFAULT_SEARCH_OPTIONS, caseSensitive: true }).length,
    1,
  );
});

test("a whole word search ignores the words around it", () => {
  const hits = matches("кот", "кот котик подкот", {
    ...DEFAULT_SEARCH_OPTIONS,
    wholeWord: true,
  });
  assert.equal(hits.length, 1);
  assert.equal(hits[0].from, 0);
});

test("a whole word search knows cyrillic letters", () => {
  const hits = matches("дом", "домик дом надом", {
    ...DEFAULT_SEARCH_OPTIONS,
    wholeWord: true,
  });
  assert.equal(hits.length, 1);
  assert.equal(hits[0].from, 6);
});

test("a regular expression finds the groups", () => {
  const hits = matches("(\\w+)@(\\w+)", "почта user@example", {
    ...DEFAULT_SEARCH_OPTIONS,
    regexp: true,
  });
  assert.equal(hits.length, 1);
  assert.deepEqual(hits[0].groups, ["user", "example"]);
});

test("an expression that matches nothing does not hang", () => {
  const hits = matches("a*", "ббб", { ...DEFAULT_SEARCH_OPTIONS, regexp: true });
  assert.equal(hits.length, 4);
  assert.equal(hits[3].from, 3);
});

test("the group references of a replacement are expanded", () => {
  const hits = matches("(\\w+)@(\\w+)", "user@example", {
    ...DEFAULT_SEARCH_OPTIONS,
    regexp: true,
  });
  assert.equal(expandReplacement("$2:$1", hits[0]), "example:user");
  assert.equal(expandReplacement("$$1", hits[0]), "$1");
});

test("the next match is found and the search wraps around", () => {
  const hits = matches("a", "a b a b a");
  assert.equal(findNextHit(hits, 0)?.from, 0);
  assert.equal(findNextHit(hits, 1)?.from, 4);
  assert.equal(findNextHit(hits, 99)?.from, 0);
  assert.equal(findNextHit(hits, 4, true)?.from, 0);
  assert.equal(findNextHit(hits, 99, true)?.from, 8);
});

test("every match is replaced at once", () => {
  const text = "кот и кот";
  const hits = matches("кот", text);
  const change = buildReplaceAll(text, hits, "собака", false);
  assert.ok(change);
  assert.equal(
    text.slice(0, change.from) + change.insert + text.slice(change.to),
    "собака и собака",
  );
});

test("the change of a replacement covers only the matches", () => {
  const text = "aXbXc";
  const hits = matches("X", text);
  const change = buildReplaceAll(text, hits, "YYY", false);
  assert.ok(change);
  assert.equal(change.from, 1);
  assert.equal(change.to, 4);
  // The text between the matches is kept, the text around them is not touched.
  assert.equal(change.insert, "YYYbYYY");
  assert.equal(
    text.slice(0, change.from) + change.insert + text.slice(change.to),
    "aYYYbYYYc",
  );
});

test("a regular expression keeps its groups while replacing", () => {
  const text = "a=1 b=2";
  const hits = matches("(\\w)=(\\w)", text, { ...DEFAULT_SEARCH_OPTIONS, regexp: true });
  const change = buildReplaceAll(text, hits, "$2=$1", true);
  assert.ok(change);
  assert.equal(
    text.slice(0, change.from) + change.insert + text.slice(change.to),
    "1=a 2=b",
  );
});

test("a group that does not exist becomes an empty text", () => {
  const hits = matches("MCP(ad)", "MCPad", { ...DEFAULT_SEARCH_OPTIONS, regexp: true });
  assert.equal(expandReplacement("$2-$1", hits[0]), "-ad");
});
