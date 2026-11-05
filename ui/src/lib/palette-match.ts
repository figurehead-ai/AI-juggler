/**
 * AJ-E2901: ⌘K's loose matching, shared with the rail's search so the two
 * agree. Each typed word has to match one of an item's fields: at its
 * start (best), at the start of one of its words, inside one, or as its
 * letters in order within one word ("wrk" finds "Work", "bil" "billing").
 * Ranked: the title starting with everything typed, then by how well each
 * word matched (the title a little ahead of the other fields), then by
 * recency. The title's matched letters come back as ranges, drawn bold
 * (never by colour alone, WCAG 1.4.1).
 */

/** `[start, end)` in the title. */
export type Range = readonly [number, number];

export interface Matched {
  score: number;
  ranges: Range[];
}

const PREFIX = 4;
const WORD_START = 3;
const INSIDE = 2;
const LOOSE = 1;

const isWordChar = (char: string | undefined) => char !== undefined && /[\p{L}\p{N}]/u.test(char);

/** Where each word of `text` starts. */
function wordStarts(text: string): number[] {
  const starts: number[] = [];
  for (let at = 0; at < text.length; at++) if (isWordChar(text[at]) && !isWordChar(text[at - 1])) starts.push(at);
  return starts;
}

/** `word`'s letters in order inside the one word of `text` starting at `from`. */
function inOneWord(word: string, text: string, from: number): Range[] | null {
  const ranges: [number, number][] = [];
  let at = from;
  for (const letter of word) {
    while (at < text.length && isWordChar(text[at]) && text[at] !== letter) at++;
    if (at >= text.length || text[at] !== letter) return null;
    const last = ranges[ranges.length - 1];
    if (last && last[1] === at) last[1] = at + 1;
    else ranges.push([at, at + 1]);
    at++;
  }
  return ranges;
}

/** How well one typed word (lower case) matches `text`; null when not at all. */
export function matchWord(word: string, text: string): Matched | null {
  const lower = text.toLowerCase();
  if (!word) return { score: PREFIX, ranges: [] };
  if (lower.startsWith(word)) return { score: PREFIX, ranges: [[0, word.length]] };
  const starts = wordStarts(lower);
  for (const at of starts) if (lower.startsWith(word, at)) return { score: WORD_START, ranges: [[at, at + word.length]] };
  const inside = lower.indexOf(word);
  if (inside >= 0) return { score: INSIDE, ranges: [[inside, inside + word.length]] };
  for (const at of starts) {
    if (lower[at] !== word[0]) continue;
    const ranges = inOneWord(word, lower, at);
    if (ranges) return { score: LOOSE, ranges };
  }
  return null;
}

/** Every typed word against the title or one of `other`; null when one
 * matches nowhere. */
export function fuzzyMatch(query: string, title: string, other: readonly string[] = []): Matched | null {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return { score: 0, ranges: [] };
  let score = title.toLowerCase().startsWith(words.join(" ")) ? 100 : 0;
  const ranges: Range[] = [];
  for (const word of words) {
    const inTitle = matchWord(word, title);
    let best = inTitle ? inTitle.score + 0.5 : 0;
    for (const field of other) {
      if (best >= PREFIX) break;
      const found = matchWord(word, field);
      if (found && found.score > best) best = found.score;
    }
    if (best === 0) return null;
    if (inTitle) ranges.push(...inTitle.ranges);
    score += best;
  }
  return { score, ranges: merge(ranges) };
}

function merge(ranges: Range[]): Range[] {
  const sorted = [...ranges].sort((a, b) => a[0] - b[0]);
  const out: [number, number][] = [];
  for (const [start, end] of sorted) {
    const last = out[out.length - 1];
    if (last && start <= last[1]) last[1] = Math.max(last[1], end);
    else out.push([start, end]);
  }
  return out;
}

/** `items` that match `query`, best first, then the most recent. */
export function rank<T>(items: readonly T[], query: string, fields: (item: T) => { title: string; other?: readonly string[]; recency?: number | null }): { item: T; ranges: Range[] }[] {
  const scored = items.flatMap((item) => {
    const { title, other = [], recency = 0 } = fields(item);
    const found = fuzzyMatch(query, title, other);
    return found ? [{ item, ranges: found.ranges, score: found.score, recency: recency ?? 0 }] : [];
  });
  scored.sort((a, b) => b.score - a.score || b.recency - a.recency);
  return scored.map(({ item, ranges }) => ({ item, ranges }));
}

/** `text` cut at `ranges`: the matched parts flagged. */
export function highlight(text: string, ranges: readonly Range[]): { text: string; hit: boolean }[] {
  const parts: { text: string; hit: boolean }[] = [];
  let at = 0;
  for (const [start, end] of ranges) {
    if (start > at) parts.push({ text: text.slice(at, start), hit: false });
    parts.push({ text: text.slice(start, end), hit: true });
    at = end;
  }
  if (at < text.length) parts.push({ text: text.slice(at), hit: false });
  return parts;
}
