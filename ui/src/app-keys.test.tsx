/**
 * AJ-E1504 — keys: the `?` sheet lists exactly the registered shortcuts;
 * ⌘1–4 open Home, Accounts, Usage and Fleet; ⌘⇧I folds the inspector and
 * remembers it; the inspector's tabs moved to ⌥⌘1–6; and the typing rule
 * holds for every shortcut (no plain key or Ctrl-combo while typing in a
 * field or the terminal; ⌘ ones still work, except those a field owns).
 */
import appSource from "./App.tsx?raw";
import appKeysSource from "./lib/app-keys.ts?raw";
import homeSource from "./screens/Home.tsx?raw";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { ShortcutSheet } from "./components/ShortcutSheet";
import { matches, SHORTCUTS, SHORTCUT_AREAS, type KeyLike, type ShortcutId } from "./lib/shortcuts";

vi.mock("./components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} className="xterm" />,
}));

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  localStorage.setItem("aij.onboarding.dismissed", "1");
  sessionStorage.setItem("aij.ready.skipped", "1");
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
});

describe("AJ-E1504 the ? sheet", () => {
  it("lists exactly the registered shortcuts, by area, with their keys", () => {
    render(<ShortcutSheet onClose={() => {}} />);
    const rows = [...document.querySelectorAll<HTMLElement>("[data-shortcut]")];
    expect(rows.map((row) => row.dataset.shortcut)).toEqual(
      SHORTCUT_AREAS.flatMap((area) => SHORTCUTS.filter((entry) => entry.area === area).map((entry) => entry.id)),
    );
    expect(rows).toHaveLength(SHORTCUTS.length);
    for (const entry of SHORTCUTS) {
      const row = rows.find((candidate) => candidate.dataset.shortcut === entry.id)!;
      expect(row.textContent).toContain(entry.label);
      for (const keys of entry.keys) expect(row.textContent).toContain(keys);
    }
    for (const area of SHORTCUT_AREAS) expect(screen.getByRole("heading", { name: area })).toBeTruthy();
  });

  it("every area has a shortcut, every id is unique, and only a natively handled key has no match", () => {
    expect(new Set(SHORTCUTS.map((entry) => entry.id)).size).toBe(SHORTCUTS.length);
    for (const area of SHORTCUT_AREAS) expect(SHORTCUTS.some((entry) => entry.area === area)).toBe(true);
    const unmatched = SHORTCUTS.filter((entry) => !("match" in entry)).map((entry) => entry.id);
    expect(unmatched.sort()).toEqual(["close", "go-to", "home-open", "leave-terminal", "row-menu"]);
  });

  // The anti-drift rule: App and Home test keys only through the
  // registry, so a key can't work without being on the sheet.
  it("App and Home read no key by hand", () => {
    for (const [file, source] of [["App.tsx", appSource], ["lib/app-keys.ts", appKeysSource], ["screens/Home.tsx", homeSource]] as const) {
      // Esc closing a menu is the registry's "close" (handled natively).
      expect(source, file).not.toMatch(/event\.key(\.toLowerCase\(\))?\s*===\s*"(?!Escape")/);
      expect(source, file).not.toMatch(/(metaKey|ctrlKey)\s*(\|\||&&)/);
    }
  });
});

const key = (over: Partial<KeyLike>): KeyLike => ({
  key: "",
  metaKey: false,
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  target: null,
  ...over,
});

describe("AJ-E1504 the typing rule", () => {
  const input = document.createElement("input");
  const terminal = document.createElement("div");
  terminal.className = "xterm";
  const inTerminal = document.createElement("textarea");
  terminal.append(inTerminal);
  const body = document.body;

  it("holds for every shortcut: plain keys and Ctrl never while typing; ⌘ unless the field owns it", () => {
    const samples: Record<string, Partial<KeyLike>> = {
      help: { key: "?", shiftKey: true },
      palette: { key: "k" },
      "new-project": { key: "n" },
      "toggle-rail": { key: "b" },
      inspector: { key: "I", shiftKey: true },
      density: { key: "d" },
      theme: { key: "L", shiftKey: true },
      undo: { key: "z" },
      "go-home": { key: "1", code: "Digit1" },
      "go-accounts": { key: "2", code: "Digit2" },
      "go-usage": { key: "3", code: "Digit3" },
      "go-fleet": { key: "4", code: "Digit4" },
      settings: { key: "," },
      cleanup: { key: "C", shiftKey: true },
      "cleanup-filter": { key: "f" },
      stop: { key: "." },
      handover: { key: "H", shiftKey: true },
      "inspector-tab": { key: "¡", code: "Digit1", altKey: true },
      "remote-back": { key: "[" },
      "home-next": { key: "j" },
      "home-prev": { key: "k" },
      "home-action": { key: "a" },
    };
    const plainKeys = new Set(["help", "home-next", "home-prev", "home-action"]);
    for (const entry of SHORTCUTS) {
      if (!("match" in entry)) continue;
      const id = entry.id as ShortcutId;
      const sample = samples[id];
      expect(sample, `a sample for ${id}`).toBeTruthy();
      const withMeta = plainKeys.has(id) ? sample : { ...sample, metaKey: true };
      const withCtrl = plainKeys.has(id) ? sample : { ...sample, ctrlKey: true };
      // Outside a field: fires (the density key as ⌘D only in the app).
      expect(matches(id, key({ ...withMeta, target: body }), true), `${id} outside`).toBe(true);
      expect(matches(id, key({ ...withCtrl, target: body }), true), `${id} with Ctrl outside`).toBe(true);
      for (const target of [input, inTerminal]) {
        expect(matches(id, key({ ...withCtrl, target }), true), `${id} with Ctrl while typing`).toBe(false);
        const metaAllowed = !plainKeys.has(id) && !("fieldOwnsIt" in entry && entry.fieldOwnsIt);
        expect(matches(id, key({ ...withMeta, target }), true), `${id} with ⌘ while typing`).toBe(metaAllowed);
      }
    }
  });

  it("Ctrl+K and Ctrl+D in the terminal are the shell's; ⌘K there still searches; ⌘Z in a field is the field's", () => {
    expect(matches("palette", key({ key: "k", ctrlKey: true, target: inTerminal }))).toBe(false);
    expect(matches("density", key({ key: "d", ctrlKey: true, target: inTerminal }))).toBe(false);
    expect(matches("palette", key({ key: "k", metaKey: true, target: inTerminal }))).toBe(true);
    expect(matches("undo", key({ key: "z", metaKey: true, target: input }))).toBe(false);
    expect(matches("help", key({ key: "?", shiftKey: true, target: input }))).toBe(false);
  });

  it("⌘1 is Home and ⌥⌘1 is the inspector's first tab, never both", () => {
    const cmd1 = key({ key: "1", code: "Digit1", metaKey: true });
    const optCmd1 = key({ key: "¡", code: "Digit1", metaKey: true, altKey: true });
    expect(matches("go-home", cmd1)).toBe(true);
    expect(matches("inspector-tab", cmd1)).toBe(false);
    expect(matches("inspector-tab", optCmd1)).toBe(true);
    expect(matches("go-home", optCmd1)).toBe(false);
  });
});

async function openApp() {
  const view = render(<App />);
  await waitFor(() => expect(view.container.querySelector("[data-testid='rail']")).not.toBeNull());
  await waitFor(() => expect(view.getByTestId("home")).toBeTruthy());
  return view;
}

const press = (init: KeyboardEventInit, target: EventTarget = document.body) =>
  act(() => {
    target.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init }));
  });

describe("AJ-E1504 keys in the app", () => {
  it("? opens the sheet; typed in a field it doesn't", async () => {
    const view = await openApp();
    const search = view.getByRole("searchbox", { name: /Search sessions/i });
    press({ key: "?", shiftKey: true }, search);
    expect(view.queryByTestId("shortcut-sheet")).toBeNull();
    press({ key: "?", shiftKey: true });
    expect(view.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeTruthy();
    fireEvent.keyDown(view.getByRole("dialog", { name: "Keyboard shortcuts" }), { key: "Escape" });
    expect(view.queryByTestId("shortcut-sheet")).toBeNull();
  });

  it("⌘1–4 open Home, Accounts, Usage and Fleet", async () => {
    const view = await openApp();
    const title = () => view.getByRole("heading", { level: 1 }).textContent;
    press({ key: "2", code: "Digit2", metaKey: true });
    await waitFor(() => expect(title()).toBe("Accounts"));
    press({ key: "3", code: "Digit3", metaKey: true });
    await waitFor(() => expect(title()).toBe("Usage"));
    press({ key: "4", code: "Digit4", metaKey: true });
    await waitFor(() => expect(title()).toBe("Fleet"));
    press({ key: "1", code: "Digit1", metaKey: true });
    await waitFor(() => expect(view.getByTestId("home")).toBeTruthy());
  });

  it("⌘⇧I folds the inspector to its header line, keeps the open tab, and remembers it", async () => {
    const view = await openApp();
    const row = [...view.getByTestId("rail").querySelectorAll<HTMLButtonElement>("button")].find((b) => /feat\/read-model/.test(b.textContent ?? ""))!;
    fireEvent.click(row);
    const inspector = await view.findByTestId("right-inspector");
    expect(view.getByRole("tablist")).toBeTruthy();
    press({ key: "I", shiftKey: true, metaKey: true });
    await waitFor(() => expect(inspector.dataset.folded).toBe("true"));
    expect(view.queryByRole("tablist")).toBeNull();
    expect(view.getByRole("tabpanel")).toBeTruthy();
    expect(view.getByRole("button", { name: "Unfold the inspector" }).getAttribute("aria-expanded")).toBe("false");
    expect(localStorage.getItem("aij.inspector.folded.main")).toBe("1");
    cleanup();
    const again = await openApp();
    const rowAgain = [...again.getByTestId("rail").querySelectorAll<HTMLButtonElement>("button")].find((b) => /feat\/read-model/.test(b.textContent ?? ""))!;
    fireEvent.click(rowAgain);
    expect((await again.findByTestId("right-inspector")).dataset.folded).toBe("true");
    fireEvent.click(again.getByRole("button", { name: "Unfold the inspector" }));
    expect(again.getByRole("tablist")).toBeTruthy();
    expect(localStorage.getItem("aij.inspector.folded.main")).toBe("0");
  });

  it("⌥⌘2 picks the inspector's second tab; ⌘2 leaves for Accounts", async () => {
    const view = await openApp();
    const row = [...view.getByTestId("rail").querySelectorAll<HTMLButtonElement>("button")].find((b) => /feat\/read-model/.test(b.textContent ?? ""))!;
    fireEvent.click(row);
    await view.findByTestId("right-inspector");
    press({ key: "™", code: "Digit2", metaKey: true, altKey: true });
    await waitFor(() => expect(view.getByRole("tab", { selected: true }).textContent).toBe("Usage"));
    press({ key: "2", code: "Digit2", metaKey: true });
    await waitFor(() => expect(view.getByRole("heading", { level: 1 }).textContent).toBe("Accounts"));
  });

  it("Ctrl+K in the terminal goes to the shell, not the search", async () => {
    const view = await openApp();
    const row = [...view.getByTestId("rail").querySelectorAll<HTMLButtonElement>("button")].find((b) => /feat\/read-model/.test(b.textContent ?? ""))!;
    fireEvent.click(row);
    const terminal = await view.findByTestId(/^terminal-/);
    press({ key: "k", ctrlKey: true }, terminal);
    expect(view.queryByRole("dialog", { name: /Go to/i })).toBeNull();
  });
});
