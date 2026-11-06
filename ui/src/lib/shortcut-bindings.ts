/**
 * AJ-E2905: shortcuts you can rebind. A binding is data: a primary
 * modifier (⌘ on a Mac, Ctrl elsewhere, so it moves between systems),
 * ⇧, ⌥, and the key's place (`code`: ⌥ changes what `key` reads on a
 * Mac). Each rebindable shortcut has its default here; a change is kept
 * per Mac in AI Juggler's settings, applies at once, and is checked: never
 * a key another shortcut uses, one the system or a text field keeps, or a
 * plain key outside Home's rows. A stored change a newer version makes
 * clash falls back to the default and says so once.
 */
import { appSettingGet, appSettingSet } from "./ipc/settings";
import type { KeyLike, ShortcutId } from "./shortcuts";

export interface Binding {
  primary: boolean;
  shift: boolean;
  alt: boolean;
  /** The key's place (`KeyP`, `Comma`, `Digit0`, `Enter`…). */
  code: string;
}

const b = (code: string, mods: { primary?: boolean; shift?: boolean; alt?: boolean } = {}): Binding => ({
  primary: mods.primary ?? true,
  shift: mods.shift ?? false,
  alt: mods.alt ?? false,
  code,
});

/** The rebindable shortcuts and their keys as shipped. */
export const DEFAULT_BINDINGS: Partial<Record<ShortcutId, Binding>> = {
  help: b("Slash", { primary: false, shift: true }),
  palette: b("KeyK"),
  "new-session": b("KeyN"),
  "new-project": b("KeyN", { shift: true }),
  notifications: b("KeyN", { alt: true }),
  "toggle-rail": b("KeyB"),
  inspector: b("KeyI", { shift: true }),
  theme: b("KeyL", { shift: true }),
  undo: b("KeyZ"),
  "rail-view": b("KeyV", { primary: false, shift: true }),
  "go-home": b("Digit0"),
  "go-projects": b("KeyP", { shift: true }),
  "go-pools": b("KeyO", { shift: true }),
  "go-tasks": b("KeyT", { shift: true }),
  "go-accounts": b("KeyA", { shift: true }),
  "go-usage": b("KeyU", { shift: true }),
  "go-history": b("KeyY", { shift: true }),
  "go-fleet": b("KeyF", { shift: true }),
  settings: b("Comma"),
  processes: b("KeyC", { shift: true }),
  "processes-search": b("KeyF"),
  stop: b("Period"),
  handover: b("KeyH", { shift: true }),
  "remote-back": b("BracketLeft"),
  "home-action": b("KeyA", { primary: false }),
  "home-reply": b("KeyR", { primary: false }),
  "home-handover": b("KeyH", { primary: false }),
  "home-snooze": b("KeyS", { primary: false }),
  "home-archive": b("KeyE", { primary: false }),
};

/** Why a shortcut can't be changed (the platform's, a control's, a range). */
export const FIXED_WHY: Partial<Record<ShortcutId, string>> = {
  close: "Esc closes what's open everywhere.",
  "row-menu": "⇧F10 is the system's key for a menu.",
  "row-open": "↩ is the row's own key.",
  "row-go-to": "⌘↩ is the row's own key.",
  "go-place": "One key for each app in the list, in its order.",
  "rail-move": "A pair: up and down.",
  density: "⌃D works the same way on every system.",
  floor: "The menu bar's own key: it works from the Floor's window too.",
  "inspector-tab": "One key for each of the inspector's tabs.",
  "leave-terminal": "The terminal's own key.",
  "terminal-find": "The terminal's own tools, read while you type in it.",
  "terminal-larger": "The terminal's own tools, read while you type in it.",
  "terminal-smaller": "The terminal's own tools, read while you type in it.",
  "terminal-reset": "The terminal's own tools, read while you type in it.",
  "terminal-maximize": "The terminal's own tools, read while you type in it.",
  "page-action": "⌘↩ is the button's own key, where a page offers one.",
  "epoch-resume": "The switch graph's own key, on its open epoch.",
  "home-next": "Two keys each: J and ↓.",
  "home-prev": "Two keys each: K and ↑.",
  "home-open": "↩ is the row's own key.",
  "go-to": "Chosen in Settings › Desktop apps, as it works from any app.",
};

/** Keys the system or a text field keeps. The system's own list comes
 * from the backend's platform seam (`juggler_platform::system::
 * reserved_keys`, in the app info: AJ-E2905 R7); macOS's stands in until
 * it answers, and in the preview. */
const MAC_RESERVED: { binding: Binding; whose: string }[] = [
  { binding: b("KeyQ"), whose: "quits the app" },
  { binding: b("KeyW"), whose: "closes the window" },
  { binding: b("KeyH"), whose: "hides the app" },
  { binding: b("KeyM"), whose: "minimises the window" },
  { binding: b("Tab"), whose: "switches apps" },
  { binding: b("Space"), whose: "is Spotlight" },
  { binding: b("KeyC"), whose: "copies" },
  { binding: b("KeyV"), whose: "pastes" },
  { binding: b("KeyX"), whose: "cuts" },
  { binding: b("KeyA"), whose: "selects all" },
];

let reserved = MAC_RESERVED;

/** The system's reserved keys, from the app info (an empty list keeps
 * the stand-in). */
export function setReservedKeys(list: readonly { code: string; primary: boolean; alt: boolean; whose: string }[] | undefined): void {
  if (!list || list.length === 0) return;
  reserved = list.map((key) => ({ binding: { code: key.code, primary: key.primary, alt: key.alt, shift: false }, whose: key.whose }));
}

const STORE_KEY = "ui.shortcuts";

let custom: Partial<Record<ShortcutId, Binding>> = {};
const listeners = new Set<() => void>();

export const same = (x: Binding, y: Binding): boolean => x.primary === y.primary && x.shift === y.shift && x.alt === y.alt && x.code === y.code;

/** The binding `id` uses now (changed, or as shipped); undefined for a fixed one. */
export function effectiveBinding(id: ShortcutId): Binding | undefined {
  return custom[id] ?? DEFAULT_BINDINGS[id];
}

/** Whether `id` was changed from its default. */
export function isChanged(id: ShortcutId): boolean {
  return custom[id] !== undefined;
}

/** The changed binding, if any (`matches` reads it). */
export function customBinding(id: ShortcutId): Binding | undefined {
  return custom[id];
}

/** Whether `event` is `binding`. */
export function matchBinding(binding: Binding, event: KeyLike): boolean {
  const primary = event.metaKey || event.ctrlKey;
  return primary === binding.primary && !!event.shiftKey === binding.shift && !!event.altKey === binding.alt && event.code === binding.code;
}

const KEY_WORDS: Record<string, string> = {
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Minus: "−",
  Equal: "=",
  Backquote: "`",
  Enter: "↩",
  Space: "Space",
  Tab: "Tab",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

/** A key's place, as its key cap reads ("KeyP" → "P"). */
function keyCap(code: string): string {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return KEY_WORDS[code] ?? code;
}

/** How a binding reads: "⌥⌘N", "⇧V", "R". */
export function bindingWords(binding: Binding): string {
  if (!binding.primary && binding.shift && binding.code === "Slash" && !binding.alt) return "?";
  return `${binding.alt ? "⌥" : ""}${binding.shift ? "⇧" : ""}${binding.primary ? "⌘" : ""}${keyCap(binding.code)}`.replace("⇧⌘", "⌘⇧");
}

/** The `aria-keyshortcuts` form: "Meta+Shift+P". */
export function bindingAria(binding: Binding): string {
  const key = keyCap(binding.code);
  return [binding.primary ? "Meta" : null, binding.alt ? "Alt" : null, binding.shift ? "Shift" : null, key === "↩" ? "Enter" : key].filter(Boolean).join("+");
}

/** The binding a key press makes; null for a lone modifier. */
export function bindingOf(event: KeyLike): Binding | null {
  if (!event.code || ["MetaLeft", "MetaRight", "ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight"].includes(event.code)) return null;
  return { primary: event.metaKey || event.ctrlKey, shift: event.shiftKey, alt: event.altKey, code: event.code };
}

/** The keys the fixed shortcuts hold (a change can't take them). */
const FIXED_KEYS: { id: ShortcutId; binding: Binding }[] = [
  ...[1, 2, 3, 4, 5, 6, 7, 8, 9].map((n) => ({ id: "go-place" as ShortcutId, binding: b(`Digit${n}`) })),
  ...[1, 2, 3, 4, 5, 6].map((n) => ({ id: "inspector-tab" as ShortcutId, binding: b(`Digit${n}`, { alt: true }) })),
  { id: "row-go-to", binding: b("Enter") },
  { id: "epoch-resume", binding: b("KeyR") },
  { id: "density", binding: b("KeyD") },
  { id: "floor", binding: b("KeyM", { shift: true }) },
  { id: "rail-move", binding: b("ArrowUp", { alt: true }) },
  { id: "rail-move", binding: b("ArrowDown", { alt: true }) },
];

/** Why `id` can't take `binding`, in words, or null when it can. `labelOf`
 * names another shortcut; `area` is `id`'s (Home's row keys may be plain
 * letters). */
export function refusal(id: ShortcutId, binding: Binding, labelOf: (id: ShortcutId) => string, area: string): string | null {
  const words = bindingWords(binding);
  for (const other of Object.keys(DEFAULT_BINDINGS) as ShortcutId[]) {
    const theirs = effectiveBinding(other);
    if (other !== id && theirs && same(theirs, binding)) return `${words} is ${labelOf(other)}. Change that one first.`;
  }
  const fixed = FIXED_KEYS.find((entry) => same(entry.binding, binding));
  if (fixed) return `${words} is ${labelOf(fixed.id)}. Pick another key.`;
  const kept = reserved.find((entry) => same(entry.binding, binding));
  if (kept) return `${words} ${kept.whose} on this computer. Pick another key.`;
  if (!binding.primary && !binding.alt && area !== "Home") return "Use ⌘ or ⌥ with the key: a plain key belongs to what you type.";
  return null;
}

function notify(): void {
  for (const listener of listeners) listener();
}

/** Calls `on` when a binding changes; returns the disposer. */
export function onBindingsChanged(on: () => void): () => void {
  listeners.add(on);
  return () => listeners.delete(on);
}

async function save(): Promise<void> {
  await appSettingSet(STORE_KEY, Object.keys(custom).length === 0 ? "" : JSON.stringify(custom));
}

/** Changes `id` to `binding` (checked by the caller with `refusal`). */
export async function setBinding(id: ShortcutId, binding: Binding): Promise<void> {
  custom = same(binding, DEFAULT_BINDINGS[id] ?? binding) && DEFAULT_BINDINGS[id] ? withoutId(id) : { ...custom, [id]: binding };
  notify();
  await save();
}

function withoutId(id: ShortcutId): Partial<Record<ShortcutId, Binding>> {
  const { [id]: _, ...rest } = custom;
  return rest;
}

/** Puts `id` back as shipped. */
export async function resetBinding(id: ShortcutId): Promise<void> {
  custom = withoutId(id);
  notify();
  await save();
}

/** Puts every shortcut back; returns what was changed, for Undo. */
export async function resetAll(): Promise<Partial<Record<ShortcutId, Binding>>> {
  const before = custom;
  custom = {};
  notify();
  await save();
  return before;
}

/** Puts `bindings` back (Reset all's Undo). */
export async function restoreBindings(bindings: Partial<Record<ShortcutId, Binding>>): Promise<void> {
  custom = { ...bindings };
  notify();
  await save();
}

/** Reads the changed bindings (once, at start). One a newer version makes
 * clash (its shortcut gone, or its key now another's or the system's) goes
 * back to the default; `onDropped` says which, once. */
export async function loadBindings(labelOf: (id: ShortcutId) => string, areaOf: (id: ShortcutId) => string | undefined, onDropped?: (words: string) => void): Promise<void> {
  let raw: string | null = null;
  try {
    raw = await appSettingGet(STORE_KEY);
  } catch {
    raw = null;
  }
  let parsed: Partial<Record<ShortcutId, Binding>> = {};
  try {
    parsed = raw ? (JSON.parse(raw) as Partial<Record<ShortcutId, Binding>>) : {};
  } catch {
    parsed = {};
  }
  custom = {};
  const dropped: string[] = [];
  for (const [id, binding] of Object.entries(parsed) as [ShortcutId, Binding][]) {
    const area = areaOf(id);
    if (!DEFAULT_BINDINGS[id] || !area || refusal(id, binding, labelOf, area) !== null) {
      dropped.push(labelOf(id));
      continue;
    }
    custom = { ...custom, [id]: binding };
  }
  notify();
  if (dropped.length > 0) {
    await save();
    onDropped?.(`${dropped.join(", ")}: back to the usual key, as the one you chose is now taken.`);
  }
}

/** For tests: forget every change. */
export function resetForTests(): void {
  custom = {};
  reserved = MAC_RESERVED;
  notify();
}
