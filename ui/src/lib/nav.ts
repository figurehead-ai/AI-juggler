/**
 * The app's destinations: the header's (D-25), the ones under More, and
 * every global screen's id. Moved out of App.tsx (its size budget).
 */
import { useEffect } from "react";

export const GLOBAL_NAV = [
  // D-25 amendment (AJ-E1021): Triage and Overview folded into Home.
  // AJ-E1007: every ended Instance — search, archive, replay.
  { id: "history", label: "History" },
  { id: "accounts", label: "Accounts" },
  { id: "usage", label: "Usage" },
  { id: "fleet", label: "Fleet" },
  // AJ-E506: the Cleanup Centre is a GLOBAL surface (ADR-0007) —
  // never a rail peer of Projects/Instances/Sessions.
  { id: "cleanup", label: "Processes" },
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

/** The screens that are a Settings pane: their id opens it there. */
export const IN_SETTINGS: Partial<Record<GlobalId, string>> = { privacy: "privacy" };

/** One place for Privacy (the review of 0.1.4): its screen id opens
 * Settings' pane, however it's asked for (More, a link, an event). The
 * tool page folded into Settings › Accounts: the returned call opens it
 * there (a session's tool chip, Open tool). */
export function useSettingsLinks(global: GlobalId | null, setGlobal: (id: GlobalId) => void, setSection: (section: string) => void, beforeOpen: () => void): () => void {
  useEffect(() => {
    const pane = global ? IN_SETTINGS[global] : undefined;
    if (!pane) return;
    beforeOpen();
    setSection(pane);
    setGlobal("settings");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [global]);
  return () => {
    beforeOpen();
    setSection("tools");
    setGlobal("settings");
  };
}

/** D-25: the header's destinations; the rest are one click away
 * under More (Fleet joins the header once a Mac is paired; Cleanup shows
 * as a pill while something is left behind). */
export const PRIMARY_NAV: Array<{ id: GlobalId | "home"; label: string }> = [
  { id: "home", label: "Home" },
  { id: "accounts", label: "Accounts" },
  // The review of 0.1.4: the page with the most to show, out of More.
  { id: "usage", label: "Usage" },
  { id: "history", label: "History" },
  { id: "settings", label: "Settings" },
];
export const MORE_NAV: Array<{ id: GlobalId; label: string; hint: string }> = [
  { id: "thismac", label: "This Mac", hint: "What your AI tools use on this Mac" },
  { id: "fleet", label: "Fleet", hint: "Your other Macs" },
  { id: "cleanup", label: "Processes", hint: "What your sessions run and left behind, and the cleanup history" },
  // One place for Privacy: this opens Settings › Privacy.
  { id: "privacy", label: "Privacy", hint: "In Settings: what AI Juggler reads, changes and sends" },
];
