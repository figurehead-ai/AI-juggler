/**
 * AJ-E1504 — every keyboard shortcut, in one registry. The App, Home and
 * the inspector match keys through `matches`; the `?` sheet is rendered
 * from `SHORTCUTS`, so the sheet can't drift from what the keys do (a
 * test reads both).
 *
 * The typing rule (Req.3; AJ-E403's Ctrl+D rule, made general): while
 * typing in a field or the terminal, plain keys and Ctrl-combos belong to
 * what is typed into (Ctrl+K kills a line in the shell, Ctrl+D ends
 * input), so no shortcut takes them. ⌘ shortcuts still work there,
 * except the ones a field has its own use for (⌘Z); so do Ctrl+Shift
 * ones on Windows and Linux, as ⌘⇧ does on a Mac, except Ctrl+Shift+C
 * and V (a terminal's copy and paste there).
 */
import { isTypingTarget } from "./density";

export interface KeyLike {
  key: string;
  code?: string;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  target?: EventTarget | null;
}

export type ShortcutArea = "Anywhere" | "Screens" | "Session" | "Home" | "From any app";

/** The sheet's sections, in order. */
export const SHORTCUT_AREAS: ShortcutArea[] = ["Anywhere", "Screens", "Session", "Home", "From any app"];

export interface Shortcut {
  id: string;
  /** What it does, in a few words. */
  label: string;
  /** How it reads on the sheet. */
  keys: readonly string[];
  area: ShortcutArea;
  /** The key test (modifiers exact). Absent where the key is handled
   * natively (a row's button, Esc in a sheet, the terminal's Ctrl+Tab,
   * the system-wide hotkey). */
  match?: (event: KeyLike, inApp: boolean) => boolean;
  /** A ⌘ shortcut a text field has its own use for: not while typing. */
  fieldOwnsIt?: boolean;
  /** Who handles it: App's one listener, Home, the inspector, the
   * terminal, each sheet, or the system-wide hotkey. */
  owner: "app" | "home" | "inspector" | "terminal" | "sheets" | "system";
}

/** ⌘ (or Ctrl) + `key`, with exactly these other modifiers. */
function cmd(key: string, opts: { shift?: boolean; alt?: boolean } = {}) {
  return (event: KeyLike) =>
    (event.metaKey || event.ctrlKey) &&
    !!event.shiftKey === !!opts.shift &&
    !!event.altKey === !!opts.alt &&
    event.key.toLowerCase() === key;
}

/** A plain key (Shift allowed where the key needs it, as for "?"). */
function plain(...keys: string[]) {
  return (event: KeyLike) => !event.metaKey && !event.ctrlKey && !event.altKey && keys.includes(event.key);
}

/** ⌘ + a digit, by the key's place (⌥ changes what `key` reads). */
function cmdDigit(digits: number[], opts: { alt?: boolean } = {}) {
  return (event: KeyLike) => {
    if (!(event.metaKey || event.ctrlKey) || event.shiftKey || !!event.altKey !== !!opts.alt) return false;
    const digit = event.code?.startsWith("Digit") ? Number(event.code.slice(5)) : Number(event.key);
    return digits.includes(digit);
  };
}

export const SHORTCUTS = [
  // Anywhere
  { id: "help", owner: "app", area: "Anywhere", keys: ["?"], label: "Show these shortcuts", match: plain("?") },
  { id: "palette", owner: "app", area: "Anywhere", keys: ["⌘K"], label: "Search and go to a session", match: cmd("k") },
  { id: "new-project", owner: "app", area: "Anywhere", keys: ["⌘N"], label: "New project", match: cmd("n") },
  { id: "toggle-rail", owner: "app", area: "Anywhere", keys: ["⌘B"], label: "Show or hide the list", match: cmd("b") },
  { id: "inspector", owner: "app", area: "Anywhere", keys: ["⌘⇧I"], label: "Fold or unfold the inspector", match: cmd("i", { shift: true }) },
  {
    id: "density",
    owner: "app",
    area: "Anywhere",
    keys: ["⌃D"],
    label: "Change how much each row shows",
    // AJ-E403 Req.4: Ctrl+D everywhere; ⌘D only in the app (the browser
    // preview keeps its bookmark key).
    match: (event: KeyLike, inApp: boolean) =>
      event.key.toLowerCase() === "d" && !event.shiftKey && !event.altKey && (event.ctrlKey || (event.metaKey && inApp)),
  },
  { id: "theme", owner: "app", area: "Anywhere", keys: ["⌘⇧L"], label: "Light, dark or match the Mac", match: cmd("l", { shift: true }) },
  { id: "undo", owner: "app", area: "Anywhere", keys: ["⌘Z"], label: "Undo what a toast offers", match: cmd("z"), fieldOwnsIt: true },
  { id: "close", owner: "sheets", area: "Anywhere", keys: ["Esc"], label: "Close a sheet, menu or search" },
  // AJ-E1537: a focused row's menu, the same one a right-click opens.
  { id: "row-menu", owner: "sheets", area: "Anywhere", keys: ["⇧F10"], label: "Open a row's menu" },
  // Screens
  { id: "go-home", owner: "app", area: "Screens", keys: ["⌘1"], label: "Home", match: cmdDigit([1]) },
  { id: "go-accounts", owner: "app", area: "Screens", keys: ["⌘2"], label: "Accounts", match: cmdDigit([2]) },
  { id: "go-usage", owner: "app", area: "Screens", keys: ["⌘3"], label: "Usage", match: cmdDigit([3]) },
  {
    id: "go-fleet",
    owner: "app",
    area: "Screens",
    keys: ["⌘4", "⌘⇧F"],
    label: "Fleet",
    match: (event: KeyLike) => cmdDigit([4])(event) || cmd("f", { shift: true })(event),
  },
  { id: "settings", owner: "app", area: "Screens", keys: ["⌘,"], label: "Settings", match: cmd(",") },
  { id: "cleanup", owner: "app", area: "Screens", keys: ["⌘⇧C"], label: "Cleanup", match: cmd("c", { shift: true }) },
  { id: "cleanup-filter", owner: "app", area: "Screens", keys: ["⌘F"], label: "Filter Cleanup (on Cleanup)", match: cmd("f") },
  // Session
  { id: "stop", owner: "app", area: "Session", keys: ["⌘."], label: "Stop the session (5 s to undo)", match: cmd(".") },
  { id: "handover", owner: "app", area: "Session", keys: ["⌘⇧H"], label: "Switch account", match: cmd("h", { shift: true }) },
  {
    id: "inspector-tab",
    owner: "inspector",
    area: "Session",
    keys: ["⌥⌘1 … ⌥⌘6"],
    label: "Inspector tabs",
    match: cmdDigit([1, 2, 3, 4, 5, 6], { alt: true }),
  },
  { id: "remote-back", owner: "inspector", area: "Session", keys: ["⌘["], label: "Back to Fleet (another Mac's session)", match: cmd("[") },
  { id: "leave-terminal", owner: "terminal", area: "Session", keys: ["⌃Tab", "⌃⇧Tab"], label: "Leave the terminal" },
  // Home
  {
    id: "home-next",
    owner: "home",
    area: "Home",
    keys: ["J", "↓"],
    label: "Next row",
    match: (event: KeyLike) => plain("j", "ArrowDown")(event),
  },
  {
    id: "home-prev",
    owner: "home",
    area: "Home",
    keys: ["K", "↑"],
    label: "Previous row",
    match: (event: KeyLike) => plain("k", "ArrowUp")(event),
  },
  { id: "home-open", owner: "home", area: "Home", keys: ["Enter"], label: "Open the row" },
  { id: "home-action", owner: "home", area: "Home", keys: ["A"], label: "Run the row's action", match: plain("a", "A") },
  // From any app
  {
    id: "go-to",
    owner: "system",
    area: "From any app",
    keys: ["⌥Space"],
    label: "Go to a session (change the key in ⌘K)",
  },
] as const satisfies readonly Shortcut[];

export type ShortcutId = (typeof SHORTCUTS)[number]["id"];

/** The shortcuts App's listener handles (its handler map must cover them). */
export type AppShortcutId = Extract<(typeof SHORTCUTS)[number], { owner: "app" }>["id"];

const BY_ID = new Map<string, Shortcut>(SHORTCUTS.map((entry) => [entry.id, entry as Shortcut]));

/**
 * True when `event` is shortcut `id` and may fire here: never a plain
 * key or a Ctrl-combo while typing, never a ⌘ key a field owns.
 */
export function matches(id: ShortcutId, event: KeyLike, inApp = false): boolean {
  const entry = BY_ID.get(id);
  if (!entry?.match || !entry.match(event, inApp)) return false;
  if (!isTypingTarget(event.target ?? null)) return true;
  if (entry.fieldOwnsIt) return false;
  if (event.metaKey) return true;
  // Windows and Linux: Ctrl+Shift+letter is the app's while typing, as
  // ⌘⇧ is on a Mac (a terminal reads Ctrl+Shift+H as Ctrl+H, so nothing
  // it needs is lost), except Ctrl+Shift+C and V, a terminal's copy and
  // paste there. Linux CI: ⌘⇧H from the session's terminal never fired.
  return !!event.ctrlKey && !!event.shiftKey && !["c", "v"].includes(event.key.toLowerCase());
}

/** ⌘./Ctrl+. — Stop the selected live Instance (deep 02 SP-04). */
export function isStopShortcut(event: KeyLike): boolean {
  return matches("stop", event);
}

/** ⌘⇧H/Ctrl+Shift+H — focus the hand-over section. */
export function isHandoverShortcut(event: KeyLike): boolean {
  return matches("handover", event);
}

/** ⌘Z/Ctrl+Z outside a text field — undoes the newest toast (AJ-E1503). */
export function isUndoShortcut(event: KeyLike): boolean {
  return matches("undo", event);
}

// --- AJ-E1504: the inspector folded (⌘⇧I), remembered per window --------

/** This window's key: one window today; a pop-out (AJ-E1506) gets its own. */
function foldKey(): string {
  const label = (window as unknown as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } })
    .__TAURI_INTERNALS__?.metadata?.currentWindow?.label;
  return `aij.inspector.folded.${label ?? "main"}`;
}

export function readInspectorFolded(): boolean {
  try {
    return localStorage.getItem(foldKey()) === "1";
  } catch {
    return false;
  }
}

export function storeInspectorFolded(folded: boolean): void {
  try {
    localStorage.setItem(foldKey(), folded ? "1" : "0");
  } catch {
    // A blocked store only forgets the fold.
  }
}
