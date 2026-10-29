/**
 * ⌘K's own actions (deep/11 §4.17; E2106 Req.4): besides the selected
 * item's (the action registry's), the app's — go to a screen, a new
 * session or project, the notifications, the shortcuts, the list, and a
 * search of History for what's typed. The 0.2 walk (F7) found none:
 * typing "switch" found only folders with the word.
 *
 * Pure: App hands in what each one does; the palette filters them by
 * what's typed and draws them with their keys from the shortcut registry.
 */
import { GLOBAL_NAV, type GlobalId } from "./nav";
import { SHORTCUTS, type ShortcutId } from "./shortcuts";

export type PaletteCommandIcon = "home" | "screen" | "accounts" | "usage" | "history" | "processes" | "computer" | "settings" | "privacy" | "fleet" | "new" | "project" | "bell" | "keyboard" | "list" | "search";

export interface PaletteCommand {
  id: string;
  label: string;
  icon: PaletteCommandIcon;
  /** Other words it is found by ("switch" finds Accounts). */
  words?: string;
  /** Its key, as the ? sheet shows it. */
  keys?: string;
  /** Shown before any typing (with the selected item's actions). */
  top?: boolean;
  run: () => void;
}

/** What App lets the palette do. */
export interface AppDoings {
  openScreen: (id: GlobalId | "home") => void;
  newSession: () => void;
  newProject: () => void;
  notifications: () => void;
  shortcuts: () => void;
  toggleRail: () => void;
  /** History, searched for these words. */
  searchHistory: (words: string) => void;
  /** Fleet shows (another computer is paired). */
  fleet: boolean;
}

const SCREEN_ICON: Record<GlobalId | "home", PaletteCommandIcon> = {
  home: "home",
  history: "history",
  pools: "screen",
  accounts: "accounts",
  usage: "usage",
  fleet: "fleet",
  cleanup: "processes",
  settings: "settings",
  thismac: "computer",
  privacy: "privacy",
};

const SCREEN_KEY: Partial<Record<GlobalId | "home", ShortcutId>> = { home: "go-home", accounts: "go-accounts", usage: "go-usage", fleet: "go-fleet", settings: "settings", cleanup: "processes" };

const SCREEN_WORDS: Partial<Record<GlobalId | "home", string>> = {
  home: "needs you inbox start",
  accounts: "profiles subscriptions limits switch sign in",
  usage: "tokens cost limits charts",
  history: "ended past sessions transcripts search",
  pools: "agents work together canvas lead members",
  cleanup: "processes stop kill leftovers cleanup",
  thismac: "memory processor computer machine",
  settings: "preferences options",
  privacy: "data what is read sent",
};

/** The keys a shortcut reads as on the ? sheet. */
export function keysOf(id: ShortcutId | undefined): string | undefined {
  return id ? SHORTCUTS.find((entry) => entry.id === id)?.keys[0] : undefined;
}

/** Every one of App's actions for ⌘K, in the order the palette lists them. */
export function appCommands(app: AppDoings): PaletteCommand[] {
  const screens: (GlobalId | "home")[] = ["home", ...GLOBAL_NAV.map((entry) => entry.id).filter((id) => id !== "fleet" || app.fleet)];
  return [
    { id: "new-session", label: "New session…", icon: "new", words: "start launch run", keys: keysOf("new-session"), top: true, run: app.newSession },
    { id: "new-project", label: "New project…", icon: "project", words: "folder add", keys: keysOf("new-project"), run: app.newProject },
    ...screens.map((id): PaletteCommand => ({
      id: `go:${id}`,
      label: `Go to ${id === "home" ? "Home" : GLOBAL_NAV.find((entry) => entry.id === id)!.label}`,
      icon: SCREEN_ICON[id],
      words: SCREEN_WORDS[id],
      keys: keysOf(SCREEN_KEY[id]),
      run: () => app.openScreen(id),
    })),
    { id: "notifications", label: "Notifications", icon: "bell", words: "needs you alerts bell", keys: keysOf("notifications"), run: app.notifications },
    { id: "shortcuts", label: "Keyboard shortcuts", icon: "keyboard", words: "keys help", keys: keysOf("help"), run: app.shortcuts },
    { id: "toggle-rail", label: "Show or hide the list", icon: "list", words: "rail sidebar icons fold", keys: keysOf("toggle-rail"), run: app.toggleRail },
  ];
}

/** Does every word typed match the command's label or its other words? */
export function commandMatches(command: PaletteCommand, needle: string): boolean {
  const hay = `${command.label} ${command.words ?? ""}`.toLowerCase();
  return needle
    .split(/\s+/)
    .filter(Boolean)
    .every((word) => hay.includes(word));
}

/** The commands for what's typed: the top ones before any typing, the
 * matching ones after, and always a search of History for the words. */
export function commandsFor(all: PaletteCommand[], needle: string, app: Pick<AppDoings, "searchHistory">): PaletteCommand[] {
  const words = needle.trim();
  if (!words) return all.filter((command) => command.top);
  return [
    ...all.filter((command) => commandMatches(command, words)),
    { id: "search-history", label: `Search History for “${words}”`, icon: "search", run: () => app.searchHistory(words) },
  ];
}
