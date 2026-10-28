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

/** D-25: the header's destinations (Fleet joins them once a Mac is
 * paired; Cleanup shows as a pill while something is left behind).
 * AJ-E2507: Settings, This Mac, Processes, your other computers and
 * Privacy are one click away in your menu, behind the avatar. */
export const PRIMARY_NAV: Array<{ id: GlobalId | "home"; label: string }> = [
  { id: "home", label: "Home" },
  { id: "accounts", label: "Accounts" },
  // The review of 0.1.4: the page with the most to show, out of More.
  { id: "usage", label: "Usage" },
  { id: "history", label: "History" },
];

/**
 * A screen opened from a menu, a notice or the palette puts the keyboard
 * on that screen's page (§2.5): the app's `<main>` takes focus once the
 * screen has rendered and the menu that opened it has given focus back.
 */
export function focusPageSoon(): void {
  requestAnimationFrame(() => requestAnimationFrame(() => focusPage()));
}

/** The page's own place for the keyboard: the app's `<main>`. */
export function focusPage(): void {
  document.querySelector<HTMLElement>("main")?.focus({ preventScroll: true });
}

/**
 * A page opened however (a row on Home, the palette, a notice): when what
 * had focus went with the old page, the keyboard lands on the new page,
 * never on the window's body where the next Tab starts at the rail again
 * (the 0.2 walk, F11). Focus that is still somewhere (a rail row, the
 * terminal taking it) stays where it is.
 */
export function focusPageIfLost(): void {
  requestAnimationFrame(() =>
    requestAnimationFrame(() => {
      const active = document.activeElement;
      if (!active || active === document.body || !active.isConnected) focusPage();
    }),
  );
}
