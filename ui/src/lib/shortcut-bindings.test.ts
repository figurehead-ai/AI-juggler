/**
 * AJ-E2905 vitest — rebinding: a change applies at once and is kept; it is
 * refused for another shortcut's key, a fixed one, the system's, or a
 * plain key outside Home; Reset and Reset all (with its Undo) put keys
 * back; a stored change a newer version makes clash falls back once.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  bindingAria,
  bindingOf,
  bindingWords,
  DEFAULT_BINDINGS,
  FIXED_WHY,
  isChanged,
  loadBindings,
  refusal,
  resetAll,
  resetBinding,
  resetForTests,
  restoreBindings,
  setBinding,
} from "./shortcut-bindings";
import { ariaKeysOf, keysOf, matches, SHORTCUTS, type ShortcutId } from "./shortcuts";

const w = () => window as unknown as { __AIJ_APP_SETTINGS__?: Record<string, string> };
const labelOf = (id: ShortcutId) => SHORTCUTS.find((entry) => entry.id === id)?.label ?? id;
const areaOf = (id: ShortcutId) => SHORTCUTS.find((entry) => entry.id === id)?.area;
const key = (code: string, mods: { meta?: boolean; shift?: boolean; alt?: boolean } = {}) => ({
  key: code.replace(/^Key|^Digit/, "").toLowerCase(),
  code,
  metaKey: mods.meta ?? false,
  ctrlKey: false,
  shiftKey: mods.shift ?? false,
  altKey: mods.alt ?? false,
});

beforeEach(() => {
  w().__AIJ_APP_SETTINGS__ = {};
  resetForTests();
});
afterEach(() => resetForTests());

describe("AJ-E2905 shortcut bindings", () => {
  it("every default matches the keys the registry shows", () => {
    for (const [id, binding] of Object.entries(DEFAULT_BINDINGS) as [ShortcutId, NonNullable<(typeof DEFAULT_BINDINGS)[ShortcutId]>][]) {
      const shown = SHORTCUTS.find((entry) => entry.id === id)!.keys as readonly string[];
      expect(shown, id).toContain(bindingWords(binding));
    }
  });

  it("every shortcut is either rebindable or says why it's fixed", () => {
    for (const entry of SHORTCUTS) {
      const id = entry.id as ShortcutId;
      expect(DEFAULT_BINDINGS[id] !== undefined || FIXED_WHY[id] !== undefined, id).toBe(true);
    }
  });

  it("a change applies at once, is kept, and the old key stops", async () => {
    const binding = bindingOf(key("KeyJ", { meta: true, alt: true }))!;
    expect(refusal("palette", binding, labelOf, "Anywhere")).toBeNull();
    await setBinding("palette", binding);
    expect(isChanged("palette")).toBe(true);
    expect(keysOf("palette")).toEqual(["⌥⌘J"]);
    expect(ariaKeysOf("palette")).toBe("Meta+Alt+J");
    expect(matches("palette", key("KeyJ", { meta: true, alt: true }))).toBe(true);
    expect(matches("palette", key("KeyK", { meta: true }))).toBe(false);
    expect(JSON.parse(w().__AIJ_APP_SETTINGS__!["ui.shortcuts"]!)).toEqual({ palette: binding });
    await resetBinding("palette");
    expect(isChanged("palette")).toBe(false);
    expect(matches("palette", key("KeyK", { meta: true }))).toBe(true);
    expect(w().__AIJ_APP_SETTINGS__!["ui.shortcuts"]).toBe("");
  });

  it("refuses another shortcut's key, a fixed one, the system's, and plain keys outside Home", () => {
    expect(refusal("palette", bindingOf(key("KeyN", { meta: true }))!, labelOf, "Anywhere")).toBe(`⌘N is ${labelOf("new-session")}. Change that one first.`);
    expect(refusal("palette", bindingOf(key("Digit3", { meta: true }))!, labelOf, "Anywhere")).toContain("Pick another key.");
    expect(refusal("palette", bindingOf(key("KeyQ", { meta: true }))!, labelOf, "Anywhere")).toBe("⌘Q quits the app on this computer. Pick another key.");
    expect(refusal("palette", bindingOf(key("KeyJ"))!, labelOf, "Anywhere")).toContain("plain key");
    expect(refusal("home-reply", bindingOf(key("KeyW"))!, labelOf, "Home")).toBeNull();
    expect(bindingOf({ ...key("ShiftLeft"), shiftKey: true })).toBeNull();
    expect(bindingAria(DEFAULT_BINDINGS.notifications!)).toBe("Meta+Alt+N");
  });

  it("Reset all puts every key back, and Undo restores them", async () => {
    await setBinding("palette", bindingOf(key("KeyJ", { meta: true, alt: true }))!);
    await setBinding("home-reply", bindingOf(key("KeyW"))!);
    const before = await resetAll();
    expect(isChanged("palette") || isChanged("home-reply")).toBe(false);
    await restoreBindings(before);
    expect(keysOf("home-reply")).toEqual(["W"]);
    expect(isChanged("palette")).toBe(true);
  });

  it("a stored change that now clashes falls back once, and says so", async () => {
    w().__AIJ_APP_SETTINGS__ = {
      "ui.shortcuts": JSON.stringify({ palette: { primary: true, shift: false, alt: true, code: "KeyJ" }, settings: { primary: true, shift: false, alt: false, code: "KeyN" }, gone: { primary: true, shift: false, alt: false, code: "KeyG" } }),
    };
    const said = vi.fn();
    await loadBindings(labelOf, areaOf, said);
    expect(keysOf("palette")).toEqual(["⌥⌘J"]);
    expect(isChanged("settings")).toBe(false);
    expect(said).toHaveBeenCalledTimes(1);
    expect(said.mock.calls[0]![0]).toContain(labelOf("settings"));
    expect(JSON.parse(w().__AIJ_APP_SETTINGS__!["ui.shortcuts"]!)).toEqual({ palette: { primary: true, shift: false, alt: true, code: "KeyJ" } });
  });

  it("a broken store reads as no changes", async () => {
    w().__AIJ_APP_SETTINGS__ = { "ui.shortcuts": "{not json" };
    await loadBindings(labelOf, areaOf);
    expect(Object.keys(DEFAULT_BINDINGS).some((id) => isChanged(id as ShortcutId))).toBe(false);
  });
});
