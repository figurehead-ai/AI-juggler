/**
 * AJ-E1504 — keys: the `?` sheet lists exactly the registered shortcuts;
 * ⌘1–9 go to the rail's places (AJ-E2104); ⌘0, ⌘⇧A, ⌘⇧U and ⌘⇧F open Home,
 * Accounts, Usage and Fleet; ⇧V opens the View; ⌘⇧I folds the inspector and
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
    render(<ShortcutSheet onClose={() => {}} fleet />);
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

  it("leaves Fleet's keys out while Fleet doesn't show (the 0.2 walk, F18; D-49), and names each key once per place", () => {
    render(<ShortcutSheet onClose={() => {}} />);
    const rows = [...document.querySelectorAll<HTMLElement>("[data-shortcut]")];
    expect(rows.map((row) => row.dataset.shortcut)).not.toContain("go-fleet");
    expect(rows.map((row) => row.dataset.shortcut)).not.toContain("remote-back");
    expect(rows.some((row) => /Fleet/.test(row.textContent ?? ""))).toBe(false);
    expect(rows).toHaveLength(SHORTCUTS.filter((entry) => !("fleet" in entry)).length);
    // ⌘0 twice: Home, and the terminal's text size, which says where.
    const zero = SHORTCUTS.filter((entry) => (entry.keys as readonly string[]).includes("⌘0"));
    expect(zero.map((entry) => entry.label)).toEqual(["Home", "Terminal text as set (in the terminal)"]);
  });

  it("every area has a shortcut, every id is unique, and only a natively handled key has no match", () => {
    expect(new Set(SHORTCUTS.map((entry) => entry.id)).size).toBe(SHORTCUTS.length);
    for (const area of SHORTCUT_AREAS) expect(SHORTCUTS.some((entry) => entry.area === area)).toBe(true);
    const unmatched = SHORTCUTS.filter((entry) => !("match" in entry)).map((entry) => entry.id);
    expect(unmatched.sort()).toEqual(["close", "go-to", "home-open", "leave-terminal", "row-menu", "row-open"]);
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

  it("holds for every shortcut: plain keys and Ctrl never while typing (Ctrl+Shift as ⌘⇧); ⌘ unless the field owns it", () => {
    const samples: Record<string, Partial<KeyLike>> = {
      help: { key: "?", shiftKey: true },
      palette: { key: "k" },
      "new-session": { key: "n" },
      "new-project": { key: "N", shiftKey: true },
      "toggle-rail": { key: "b" },
      inspector: { key: "I", shiftKey: true },
      density: { key: "d" },
      theme: { key: "L", shiftKey: true },
      undo: { key: "z" },
      "go-place": { key: "1", code: "Digit1" },
      "rail-view": { key: "V", shiftKey: true },
      // D-47: a row or place moved in Your order (a field's arrows are its own).
      "rail-move": { key: "ArrowUp", altKey: true },
      "go-home": { key: "0", code: "Digit0" },
      "go-projects": { key: "P", shiftKey: true },
      "go-pools": { key: "O", shiftKey: true },
      "go-tasks": { key: "T", shiftKey: true },
      "go-accounts": { key: "A", shiftKey: true },
      "go-usage": { key: "U", shiftKey: true },
      "go-fleet": { key: "F", shiftKey: true },
      settings: { key: "," },
      notifications: { key: "˜", code: "KeyN", altKey: true },
      processes: { key: "C", shiftKey: true },
      "processes-search": { key: "f" },
      stop: { key: "." },
      handover: { key: "H", shiftKey: true },
      "inspector-tab": { key: "¡", code: "Digit1", altKey: true },
      "remote-back": { key: "[" },
      "home-next": { key: "j" },
      "home-prev": { key: "k" },
      "home-action": { key: "a" },
      // AJ-E1506: the terminal's tools, ⌘ or Ctrl+Shift, by the key's place.
      "terminal-find": { key: "F", code: "KeyF", shiftKey: true },
      "terminal-larger": { key: "+", code: "Equal", shiftKey: true },
      "terminal-smaller": { key: "_", code: "Minus", shiftKey: true },
      "terminal-reset": { key: ")", code: "Digit0", shiftKey: true },
      "terminal-maximize": { key: "Enter", shiftKey: true },
      // A session row's ⌘↩: Go to it.
      "row-go-to": { key: "Enter" },
    };
    const plainKeys = new Set(["help", "home-next", "home-prev", "home-action", "rail-view"]);
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
        const metaAllowed = !plainKeys.has(id) && !("fieldOwnsIt" in entry && entry.fieldOwnsIt);
        // Ctrl+Shift+letter (Windows, Linux) is the app's while typing, as
        // ⌘⇧ is; never Ctrl+Shift+C or V, a terminal's copy and paste.
        const ctrlShiftAllowed = metaAllowed && !!sample!.shiftKey && !["c", "v"].includes(sample!.key!.toLowerCase());
        expect(matches(id, key({ ...withCtrl, target }), true), `${id} with Ctrl while typing`).toBe(ctrlShiftAllowed);
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

  it("⌘1 is the first place and ⌥⌘1 is the inspector's first tab, never both; ⌘0 is Home but the terminal's own", () => {
    const cmd1 = key({ key: "1", code: "Digit1", metaKey: true });
    const optCmd1 = key({ key: "¡", code: "Digit1", metaKey: true, altKey: true });
    expect(matches("go-place", cmd1)).toBe(true);
    expect(matches("go-home", cmd1)).toBe(false);
    expect(matches("inspector-tab", cmd1)).toBe(false);
    expect(matches("inspector-tab", optCmd1)).toBe(true);
    expect(matches("go-place", optCmd1)).toBe(false);
    expect(matches("go-home", key({ key: "0", code: "Digit0", metaKey: true }))).toBe(true);
    expect(matches("go-home", key({ key: "0", code: "Digit0", metaKey: true, target: inTerminal }))).toBe(false);
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
    expect(await view.findByRole("dialog", { name: "Keyboard shortcuts" })).toBeTruthy();
    fireEvent.keyDown(view.getByRole("dialog", { name: "Keyboard shortcuts" }), { key: "Escape" });
    expect(view.queryByTestId("shortcut-sheet")).toBeNull();
  });

  it("D-50: ⌘N opens New session, ⌘⇧N New project, ⌥⌘N the notifications", async () => {
    const view = await openApp();
    press({ key: "n", metaKey: true });
    expect(await view.findByTestId("new-session-sheet")).toBeTruthy();
    expect(view.queryByTestId("new-project-drop")).toBeNull();
    fireEvent.keyDown(view.getByTestId("new-session-sheet"), { key: "Escape" });
    await waitFor(() => expect(view.queryByTestId("new-session-sheet")).toBeNull());
    press({ key: "N", shiftKey: true, metaKey: true });
    expect(await view.findByTestId("new-project-drop")).toBeTruthy();
    expect(view.queryByTestId("new-session-sheet")).toBeNull();
    expect(view.queryByTestId("notification-centre")).toBeNull();
  });

  it("D-50: ⌥⌘N opens the notifications (by the key's place)", async () => {
    const view = await openApp();
    press({ key: "˜", code: "KeyN", altKey: true, metaKey: true });
    expect(await view.findByTestId("notification-centre")).toBeTruthy();
    expect(view.queryByTestId("new-project-drop")).toBeNull();
    expect(view.queryByTestId("new-session-sheet")).toBeNull();
  });

  it("⌘⇧A, ⌘⇧U and ⌘⇧F open Accounts, Usage and Fleet; ⌘0 Home", async () => {
    const view = await openApp();
    const title = () => view.getByRole("heading", { level: 1 }).textContent;
    press({ key: "A", shiftKey: true, metaKey: true });
    await waitFor(() => expect(title()).toBe("Accounts"));
    press({ key: "U", shiftKey: true, metaKey: true });
    await waitFor(() => expect(title()).toBe("Usage"));
    press({ key: "F", shiftKey: true, metaKey: true });
    await waitFor(() => expect(title()).toBe("Fleet"));
    press({ key: "0", code: "Digit0", metaKey: true });
    await waitFor(() => expect(view.getByTestId("home")).toBeTruthy());
  });

  it("⌘1 goes to the first place and ⇧V opens the View", async () => {
    const view = await openApp();
    const places: number[] = [];
    const listen = (event: Event) => places.push((event as CustomEvent<{ index: number }>).detail.index);
    window.addEventListener("aij:go-place", listen);
    press({ key: "1", code: "Digit1", metaKey: true });
    press({ key: "3", code: "Digit3", metaKey: true });
    window.removeEventListener("aij:go-place", listen);
    expect(places).toEqual([0, 2]);
    press({ key: "V", shiftKey: true });
    expect(await view.findByRole("dialog", { name: "View" })).toBeTruthy();
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

  it("⌥⌘2 picks the inspector's second tab; ⌘⇧A leaves for Accounts", async () => {
    const view = await openApp();
    const row = [...view.getByTestId("rail").querySelectorAll<HTMLButtonElement>("button")].find((b) => /feat\/read-model/.test(b.textContent ?? ""))!;
    fireEvent.click(row);
    await view.findByTestId("right-inspector");
    press({ key: "™", code: "Digit2", metaKey: true, altKey: true });
    // A terminal's session: Overview (its terminal), then Usage.
    await waitFor(() => expect(view.getByRole("tab", { selected: true }).textContent).toBe("Usage"));
    press({ key: "A", shiftKey: true, metaKey: true });
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
