/**
 * AJ-E2110: the "Open AI Juggler from anywhere" key, in words. There is
 * no default: the user picks one (Settings › Desktop apps, or ⌘K's
 * footer), and ⌥Space is refused because ChatGPT already uses it.
 */

/** Builds a Tauri accelerator string ("Control+Alt+Space") from a
 * KeyboardEvent; null while only modifiers are held (the capture waits
 * for a real key). */
export function toAccelerator(event: {
  key: string;
  code?: string;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}): string | null {
  if (["Control", "Alt", "AltGraph", "Shift", "Meta", "Dead", "CapsLock", "Escape"].includes(event.key)) {
    return null;
  }
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Control");
  if (event.altKey) parts.push("Alt");
  if (event.metaKey) parts.push("Super");
  if (event.shiftKey) parts.push("Shift");
  // The physical key: on a Mac ⌥ turns J into ∆ and Space into a
  // no-break space, so the key's own code names it when there is one.
  const code = event.code ?? "";
  const physical = /^Key[A-Z]$/.test(code) ? code.slice(3) : /^Digit[0-9]$/.test(code) ? code.slice(5) : code === "Space" ? "Space" : null;
  parts.push(physical ?? (event.key === " " ? "Space" : event.key.length === 1 ? event.key.toUpperCase() : event.key));
  return parts.join("+");
}

const SYMBOLS: Record<string, string> = {
  Control: "⌃",
  Ctrl: "⌃",
  Alt: "⌥",
  Option: "⌥",
  Shift: "⇧",
  Super: "⌘",
  Command: "⌘",
  Cmd: "⌘",
  CmdOrCtrl: "⌘",
  CommandOrControl: "⌘",
};

/** "Control+Alt+Space" → "⌃⌥Space"; "" when there is none. */
export function hotkeyWords(binding: string): string {
  if (binding.trim() === "") return "";
  const parts = binding.split("+");
  const key = parts.pop() ?? "";
  return parts.map((part) => SYMBOLS[part] ?? `${part}+`).join("") + key;
}

/** Keys another app on most Macs already holds, and whose they are. */
const TAKEN: Record<string, string> = {
  "Alt+Space": "ChatGPT uses ⌥Space. Pick another key.",
  "Super+Space": "Spotlight uses ⌘Space. Pick another key.",
};

/** Why a key can't be the hotkey, or null when it can. A key needs a
 * modifier, so typing a letter never opens AI Juggler. */
export function hotkeyRefusal(binding: string): string | null {
  const taken = TAKEN[binding];
  if (taken) return taken;
  if (!binding.includes("+") || /^Shift\+[^+]+$/.test(binding)) return "Use ⌃, ⌥ or ⌘ with the key.";
  return null;
}
