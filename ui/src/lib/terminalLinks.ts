/**
 * AJ-E1506 — the file paths in a terminal line, for clickable links: what
 * agents and compilers print (`src/auth/session.test.ts:42`, `./a.rs`,
 * `~/notes.md`, `/Users/me/app/main.go:3:7`). Web addresses are the
 * web-links addon's; a path inside one isn't a path. A relative path needs
 * a folder and an extension (`a/b` alone is too often not a file); the
 * backend opens only what is really there.
 */

export type PathLink = { start: number; end: number; text: string };

// Absolute, ~/, ./ or ../ paths; or a relative one with a folder and an
// extension. Then an optional :line or :line:column.
const PATH =
  /(?:(?:~|\.{1,2})?\/[\w@+%-][\w.@+%~-]*(?:\/[\w.@+%~-]+)*\/?|[\w@+-][\w.@+-]*(?:\/[\w.@+%~-]+)+\.[A-Za-z][A-Za-z0-9]{0,9})(?::\d+(?::\d+)?)?/g;
const URL = /[a-z][a-z0-9+.-]*:\/\/\S+/gi;
// A path never starts in the middle of a word or another path.
const BEFORE = /[\w/.~@:-]/;
// Sentence punctuation after a path isn't part of it.
const TRAILING = /[.,;:!?'")\]}]+$/;

export function findPathLinks(line: string): PathLink[] {
  const urls = [...line.matchAll(URL)].map((match) => [match.index, match.index + match[0].length] as const);
  const links: PathLink[] = [];
  for (const match of line.matchAll(PATH)) {
    const start = match.index;
    if (start > 0 && BEFORE.test(line[start - 1]!)) continue;
    const text = match[0].replace(TRAILING, "");
    const end = start + text.length;
    if (text.length < 2 || text === "//" || urls.some(([from, to]) => start < to && end > from)) continue;
    // A lone "/" or a bare "~/" says nothing.
    if (/^(?:~|\.{1,2})?\/$/.test(text)) continue;
    links.push({ start, end, text });
  }
  return links;
}
