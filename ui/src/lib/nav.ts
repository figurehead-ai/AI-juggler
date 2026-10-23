/**
 * The app's destinations: the header's (D-25), the ones under More, and
 * every global screen's id. Moved out of App.tsx (its size budget).
 */

export const GLOBAL_NAV = [
  // D-25 amendment (AJ-E1021): Triage and Overview folded into Home.
  // AJ-E1007: every ended Instance — search, archive, replay.
  { id: "history", label: "History" },
  { id: "accounts", label: "Accounts" },
  { id: "usage", label: "Usage" },
  { id: "fleet", label: "Fleet" },
  // AJ-E506: the Cleanup Centre is a GLOBAL surface (ADR-0007) —
  // never a rail peer of Projects/Instances/Sessions.
  { id: "cleanup", label: "Cleanup" },
  // AJ-E1203: Appearance + the optional Account & Cloud (⌘,).
  { id: "settings", label: "Settings" },
  // AJ-E1528: what Home's This Mac panel sums up, in full.
  { id: "thismac", label: "This Mac" },
  // AJ-E1518: what AI Juggler reads, changes and sends, each line tested.
  { id: "privacy", label: "Privacy" },
] as const;

export type GlobalId = (typeof GLOBAL_NAV)[number]["id"];

/** `id` as a screen's id, or `null` when no screen has it (an
 * `aij:open-screen` event can carry anything). */
export function globalId(id: unknown): GlobalId | null {
  return GLOBAL_NAV.find((entry) => entry.id === id)?.id ?? null;
}

/** D-25: the header's four destinations; the rest are one click away
 * under More (Fleet joins the header once a Mac is paired; Cleanup shows
 * as a pill while something is left behind). */
export const PRIMARY_NAV: Array<{ id: GlobalId | "home"; label: string }> = [
  { id: "home", label: "Home" },
  { id: "accounts", label: "Accounts" },
  { id: "history", label: "History" },
  { id: "settings", label: "Settings" },
];
export const MORE_NAV: Array<{ id: GlobalId; label: string; hint: string }> = [
  { id: "usage", label: "Usage", hint: "Limits and cost for each account" },
  { id: "thismac", label: "This Mac", hint: "What your AI tools use on this Mac" },
  { id: "fleet", label: "Fleet", hint: "Your other Macs" },
  { id: "cleanup", label: "Cleanup", hint: "Processes left behind, and the audit log" },
  { id: "privacy", label: "Privacy", hint: "What AI Juggler reads, changes and sends" },
];
