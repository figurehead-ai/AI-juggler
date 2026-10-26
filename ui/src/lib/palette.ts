/**
 * AJ-E1532 — the palette projects and accounts are told apart by: twelve
 * OKLCH swatches, hues 30° apart, all equally light to the eye and of one
 * chroma (as far as sRGB allows), so none shouts and neighbours stay
 * apart. A swatch is the same colour in
 * both themes; its letters are white, AA on every swatch, and every
 * swatch stands out from both themes' surfaces (palette.test.ts checks
 * all three, and the ΔE between neighbouring hues).
 *
 * A project's swatch comes from the backend (`ProjectInfo.mark`); an
 * account's is given here, by id, as the list loads. A project's mark is
 * a rounded square and an account's avatar is round, so the two never
 * read as one.
 */
import { assignProfiles, profileColor } from "./profile-marks";
import type { ProjectInfo } from "./bridge";
import type { ProjectMarkInfo } from "../types/generated/ProjectMarkInfo";
import { luminance, maxChroma, type Oklch } from "./oklch";

/** The swatches' luminance (WCAG Y): one for all, so white letters are AA
 * on each (Y ≤ 0.178) and each stands 3:1 from the dark surfaces
 * (Y ≥ 0.156) — the light ones are further still. Each hue's lightness is
 * solved for it (a blue needs more than a green), and its chroma is as
 * much of `SWATCH_C` as fits in sRGB. */
export const SWATCH_Y = 0.167;
export const SWATCH_C = 0.13;

/** The hues, in the order swatches are given out: the first few far apart
 * on the wheel, and red, orange and amber — the app's error and warning
 * colours — last. The backend's swatch index is a place in this list. */
export const PALETTE_HUES = [265, 145, 325, 205, 295, 115, 235, 175, 355, 55, 85, 25] as const;

/** Each swatch's name, for the mark sheet's colour choice. */
export const PALETTE_NAMES = ["Blue", "Green", "Magenta", "Cyan", "Violet", "Olive", "Azure", "Teal", "Pink", "Orange", "Gold", "Red"] as const;

/** A hue's swatch: the lightness whose colour has `SWATCH_Y`. */
export function swatchOf(h: number): Oklch {
  const at = (l: number): Oklch => ({ l, c: Math.min(SWATCH_C, maxChroma(l, h) * 0.97), h });
  let lo = 0.3;
  let hi = 0.8;
  for (let step = 0; step < 32; step += 1) {
    const mid = (lo + hi) / 2;
    if (luminance(at(mid)) < SWATCH_Y) lo = mid;
    else hi = mid;
  }
  return at(lo);
}

/** The swatches, in the order they are given out. */
export const PALETTE: readonly Oklch[] = PALETTE_HUES.map(swatchOf);

/** A swatch as CSS. */
export function swatchColor(index: number): string {
  const { l, c, h } = PALETTE[((index % PALETTE.length) + PALETTE.length) % PALETTE.length]!;
  return `oklch(${l.toFixed(3)} ${c.toFixed(3)} ${h})`;
}

// --- accounts ---------------------------------------------------------------

/** The user's accounts, each with its own slot (by id, so the same account
 * keeps its colour), and its initials. */
const assigned = new Map<string, number>();
const initials = new Map<string, string>();
let known: { id: string; label: string }[] = [];

/** One or two letters for a name: a two-word name's initials ("WM"),
 * else its first two letters ("Pe"). Words part at spaces, dashes,
 * underscores and dots. */
export function initialsOf(name: string): string {
  return lettersOf(wordsOf(name));
}

function wordsOf(name: string): string[] {
  return name.trim().split(/[\s\-_.]+/).filter(Boolean);
}

function lettersOf(words: string[]): string {
  if (words.length === 0) return "?";
  if (words.length >= 2) return `${[...words[0]!][0]}${[...words[words.length - 1]!][0]}`.toUpperCase();
  const [first = "?", second = ""] = [...words[0]!];
  return `${first.toUpperCase()}${second.toLowerCase()}`;
}

/** Initials that tell the user's accounts apart: labels that begin with
 * the same words ("claude-work", "claude-personal") are known by the
 * words after them ("Wo", "Pe"), not by the letter they share. Where two
 * would still meet ("claude-work", "codex-work"), the later one (in the
 * list's order) takes its next: its plain initials, then its first word's
 * letters. */
export function distinctInitials(labels: string[]): string[] {
  const split = labels.map(wordsOf);
  const lower = split.map((words) => words.map((word) => word.toLowerCase()));
  const taken = new Set<string>();
  return split.map((words, index) => {
    let start = 0;
    const others = lower.filter((_, other) => other !== index);
    while (
      start < words.length - 1 &&
      others.some((other) => other.length > start && lower[index]!.slice(0, start + 1).every((word, n) => other[n] === word))
    ) {
      start += 1;
    }
    const first = [...(words[0] ?? "?")];
    const candidates = [
      lettersOf(words.slice(start)),
      lettersOf(words),
      ...first.slice(1).map((letter) => `${first[0]!.toUpperCase()}${letter.toLowerCase()}`),
    ];
    const picked = candidates.find((candidate) => !taken.has(candidate.toLowerCase())) ?? candidates[0]!;
    taken.add(picked.toLowerCase());
    return picked;
  });
}

/** Give each of the user's accounts its slot and initials (call when the
 * list loads; cheap and idempotent). */
export function assignPalette(accounts: { id: string; label: string }[]): void {
  const sorted = [...new Map(accounts.map((account) => [account.id, account])).values()].sort((a, b) => (a.id < b.id ? -1 : 1));
  known = sorted.map(({ id, label }) => ({ id, label }));
  assigned.clear();
  sorted.forEach((account, index) => assigned.set(account.id, index % PALETTE.length));
  // One colour per profile: its name joins the badges' set.
  assignProfiles(sorted.map((account) => account.label));
  initials.clear();
  const letters = distinctInitials(sorted.map((account) => account.label));
  sorted.forEach((account, index) => initials.set(account.id, letters[index]!));
}

/** The user's accounts as last loaded (id and label). */
export function knownAccounts(): { id: string; label: string }[] {
  return known;
}

/** A stable index into the palette for `key`: its assigned slot, else a
 * hash (a key the list doesn't know yet). */
export function paletteIndex(key: string | null): number {
  const slot = key === null ? undefined : assigned.get(key);
  if (slot !== undefined) return slot;
  const text = key ?? "unknown";
  let hash = 0;
  for (let i = 0; i < text.length; i += 1) {
    hash = (hash * 31 + text.charCodeAt(i)) % 100_000;
  }
  return hash % PALETTE.length;
}

/** A profile's colour by its id, as CSS: its badge's (`profileColor` of
 * its name), so a chart by profile, its avatar and its badge agree. */
export function accountColor(accountId: string | null): string {
  const label = accountId === null ? null : (known.find((account) => account.id === accountId)?.label ?? null);
  return profileColor(label ?? accountId ?? "unknown");
}

/** An account's initials: told apart from the user's other accounts when
 * the list has loaded, else from its label alone. */
export function accountInitials(accountId: string | null, label: string): string {
  return (accountId !== null ? initials.get(accountId) : undefined) ?? initialsOf(label);
}

/** The user's accounts, as loaded: each given its slot on the way in. */
export function withPalette<T extends { id: string; label: string }>(accounts: T[]): T[] {
  assignPalette(accounts);
  return accounts;
}

// --- projects ---------------------------------------------------------------

const marks = new Map<string, ProjectMarkInfo>();

/** The user's projects, as loaded: each one's mark kept for the rows
 * that only know a project's id. */
export function withMarks(projects: ProjectInfo[]): ProjectInfo[] {
  marks.clear();
  for (const project of projects) if (project.mark) marks.set(project.id, project.mark);
  return projects;
}

/** A project's mark, when the list has it. */
export function markOf(projectId: string | null | undefined): ProjectMarkInfo | null {
  return projectId ? (marks.get(projectId) ?? null) : null;
}
