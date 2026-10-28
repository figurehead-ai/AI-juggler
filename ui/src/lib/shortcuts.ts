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
   * terminal, each sheet, a session's row, or the system-wide hotkey. */
  owner: "app" | "home" | "inspector" | "terminal" | "sheets" | "rows" | "system";
  /** Only while Fleet shows (another computer paired): the sheet leaves
   * it out otherwise (the 0.2 walk, F18; D-49). */
  fleet?: true;
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

/** AJ-E1506: a terminal tool's key — ⌘ on a Mac, Ctrl+Shift elsewhere
 * (plain Ctrl+F and Ctrl+- are the shell's), by the key's place. */
function terminalTool(on: (event: KeyLike) => boolean) {
  return (event: KeyLike) => {
    const mod = event.metaKey ? !event.ctrlKey && !event.altKey : event.ctrlKey && event.shiftKey && !event.altKey;
    return mod && on(event);
  };
}

/** ⌥⌘ + a letter, by the key's place (⌥ turns N into "˜" on a Mac). */
function cmdAltCode(code: string, key: string) {
  return (event: KeyLike) =>
    (event.metaKey || event.ctrlKey) && !!event.altKey && !event.shiftKey && (event.code === code || event.key.toLowerCase() === key);
}

/** ⌥⌘↑ or ⌥⌘↓ (Ctrl+Alt elsewhere, as ⌘ is Ctrl there). */
function cmdArrow(event: KeyLike): boolean {
  return (event.metaKey || event.ctrlKey) && event.altKey && !event.shiftKey && (event.key === "ArrowUp" || event.key === "ArrowDown");
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
  // D-50 (the maintainer, 2026-10-28): ⌘N is New session, as deep/11 §3
  // says; New project moved to ⌘⇧N.
  { id: "new-session", owner: "app", area: "Anywhere", keys: ["⌘N"], label: "New session", match: cmd("n") },
  { id: "new-project", owner: "app", area: "Anywhere", keys: ["⌘⇧N"], label: "New project", match: cmd("n", { shift: true }) },
  // AJ-E2503 (§4.14): the notification centre, as the bell opens it. D-50:
  // ⌥⌘N, since New project took ⌘⇧N (by the key's place: ⌥ changes what
  // `key` reads on a Mac).
  { id: "notifications", owner: "app", area: "Anywhere", keys: ["⌥⌘N"], label: "Notifications", match: cmdAltCode("KeyN", "n") },
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
  // A session's row (the list, Home's Needs you): ↩ opens its page here,
  // ⌘↩ goes to it in its app (the row's ↗). The maintainer's review of 0.1.4.
  { id: "row-open", owner: "rows", area: "Anywhere", keys: ["↩"], label: "Open a session's page (on its row)" },
  { id: "row-go-to", owner: "rows", area: "Anywhere", keys: ["⌘↩"], label: "Go to a session in its app (on its row)", match: cmd("enter") },
  // AJ-E2104: ⌘1–9 go to the places in the list (their apps forward), in
  // the list's stable order; the View is ⇧V.
  { id: "go-place", owner: "app", area: "Anywhere", keys: ["⌘1 … ⌘9"], label: "Go to a place in the list (its app forward)", match: cmdDigit([1, 2, 3, 4, 5, 6, 7, 8, 9]) },
  // D-47: Your order, arranged from the keyboard (the drag's twin), on a
  // focused row or place.
  { id: "rail-move", owner: "rows", area: "Anywhere", keys: ["⌥⌘↑", "⌥⌘↓"], label: "Move a row or place up or down the list (Your order)", match: cmdArrow, fieldOwnsIt: true },
  { id: "rail-view", owner: "app", area: "Anywhere", keys: ["⇧V"], label: "The list's View: group, show, only", match: (event: KeyLike) => !event.metaKey && !event.ctrlKey && !event.altKey && event.shiftKey && event.key.toLowerCase() === "v" },
  // Screens (AJ-E2104: ⌘1–4 went to the places)
  { id: "go-home", owner: "app", area: "Screens", keys: ["⌘0"], label: "Home", match: cmdDigit([0]), fieldOwnsIt: true },
  { id: "go-accounts", owner: "app", area: "Screens", keys: ["⌘⇧A"], label: "Accounts", match: cmd("a", { shift: true }) },
  { id: "go-usage", owner: "app", area: "Screens", keys: ["⌘⇧U"], label: "Usage", match: cmd("u", { shift: true }) },
  { id: "go-fleet", owner: "app", area: "Screens", keys: ["⌘⇧F"], label: "Fleet", match: cmd("f", { shift: true }), fleet: true },
  { id: "settings", owner: "app", area: "Screens", keys: ["⌘,"], label: "Settings", match: cmd(",") },
  { id: "processes", owner: "app", area: "Screens", keys: ["⌘⇧C"], label: "Processes", match: cmd("c", { shift: true }) },
  { id: "processes-search", owner: "app", area: "Screens", keys: ["⌘F"], label: "Search Processes (on Processes)", match: cmd("f") },
  // Session
  { id: "stop", owner: "app", area: "Session", keys: ["⌘."], label: "Stop the session (5 s to undo)", match: cmd(".") },
  { id: "handover", owner: "app", area: "Session", keys: ["⌘⇧H"], label: "Continue on another account", match: cmd("h", { shift: true }) },
  {
    id: "inspector-tab",
    owner: "inspector",
    area: "Session",
    keys: ["⌥⌘1 … ⌥⌘6"],
    label: "Inspector tabs",
    match: cmdDigit([1, 2, 3, 4, 5, 6], { alt: true }),
  },
  { id: "remote-back", owner: "inspector", area: "Session", keys: ["⌘["], label: "Back to Fleet (another computer's session)", match: cmd("["), fleet: true },
  { id: "leave-terminal", owner: "terminal", area: "Session", keys: ["⌃Tab", "⌃⇧Tab"], label: "Leave the terminal" },
  // AJ-E1506: the terminal's own tools, read while typing in it (Ctrl+Shift
  // on Windows and Linux, where plain Ctrl is the shell's).
  { id: "terminal-find", owner: "terminal", area: "Session", keys: ["⌘F"], label: "Find in the terminal", match: terminalTool((e) => e.code === "KeyF") },
  { id: "terminal-larger", owner: "terminal", area: "Session", keys: ["⌘+"], label: "Larger terminal text", match: terminalTool((e) => e.code === "Equal" || e.code === "NumpadAdd") },
  { id: "terminal-smaller", owner: "terminal", area: "Session", keys: ["⌘−"], label: "Smaller terminal text", match: terminalTool((e) => e.code === "Minus" || e.code === "NumpadSubtract") },
  { id: "terminal-reset", owner: "terminal", area: "Session", keys: ["⌘0"], label: "Terminal text as set (in the terminal)", match: terminalTool((e) => e.code === "Digit0" || e.code === "Numpad0") },
  { id: "terminal-maximize", owner: "terminal", area: "Session", keys: ["⌘↩"], label: "Fill the window with the terminal, or put it back", match: terminalTool((e) => e.key === "Enter") },
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
    // AJ-E2110: no default key (⌥Space is ChatGPT's); the user picks one.
    keys: ["Your key"],
    label: "Go to a session (choose the key in Settings › Desktop apps)",
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
