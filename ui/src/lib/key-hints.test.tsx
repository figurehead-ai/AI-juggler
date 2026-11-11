/**
 * AJ-E2905 R8 — every place that shows a key reads it from the shortcut
 * registry: no key text written into JSX in the switch graph, the rows'
 * drag handle, a list group's header, a row's ↗, Compose, an account's
 * reading choice or New session; the hints and `aria-keyshortcuts` they
 * draw are the registry's.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ariaOf, ariaOfWords, keysOf, placeKey, SHORTCUTS } from "./shortcuts";
import { FIXED_WHY } from "./shortcut-bindings";
import { PlaceHeader } from "../components/PlaceHeader";
import { Grip } from "../components/RowTools";

afterEach(cleanup);

/** The code without its comments (`//`, `/* … *\/` and JSX's `{/* … *\/}`). */
function code(source: string): string {
  return source.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|[^:"'`])\/\/.*$/gm, "$1");
}

describe("AJ-E2905 R8 — key hints come from the registry", () => {
  it("no key text or aria key is written by hand in those files", async () => {
    const FILES = {
      "SwitchGraph.tsx": (await import("../components/SwitchGraph.tsx?raw")).default,
      "RowTools.tsx": (await import("../components/RowTools.tsx?raw")).default,
      "PlaceHeader.tsx": (await import("../components/PlaceHeader.tsx?raw")).default,
      "ItemMenu.tsx": (await import("../components/ItemMenu.tsx?raw")).default,
      "Compose.tsx": (await import("../pages/messages/Compose.tsx?raw")).default,
      "TermChoice.tsx": (await import("../pages/subscription/TermChoice.tsx?raw")).default,
      "NewSession.tsx": (await import("../screens/NewSession.tsx?raw")).default,
    };
    for (const [file, source] of Object.entries(FILES)) {
      const body = code(source);
      expect(body, file).not.toMatch(/[⌘⌥⌃⇧]/);
      expect(body, file).not.toMatch(/aria-keyshortcuts="/);
      expect(body, file).not.toMatch(/"(Meta|Alt|Control)\+/);
      expect(body, file).toMatch(/keysOf\(|ariaOf\(|placeKey\(/);
    }
  });

  it("the keys those controls own are in the registry, fixed, with the reason", () => {
    for (const id of ["epoch-resume", "page-action", "row-go-to", "rail-move", "go-place"] as const) {
      const entry = SHORTCUTS.find((shortcut) => shortcut.id === id)!;
      expect(entry, id).toBeTruthy();
      expect(FIXED_WHY[id], id).toBeTruthy();
    }
    expect(keysOf("epoch-resume")).toEqual(["⌘R"]);
    expect(keysOf("page-action")).toEqual(["⌘↩"]);
  });

  it("writes the registry's keys in aria-keyshortcuts form", () => {
    expect(ariaOfWords("⌘↩")).toBe("Meta+Enter");
    expect(ariaOfWords("⌥⌘↑")).toBe("Meta+Alt+ArrowUp");
    expect(ariaOfWords("⌘⇧N")).toBe("Meta+Shift+N");
    expect(ariaOfWords("⌘R")).toBe("Meta+R");
    expect(ariaOf("row-go-to")).toBe("Meta+Enter");
    expect(ariaOf("rail-move")).toBe("Meta+Alt+ArrowUp Meta+Alt+ArrowDown");
    // A rebindable one keeps its binding's form.
    expect(ariaOf("new-session")).toBe("Meta+N");
    expect(placeKey(3)).toEqual({ words: "⌘3", aria: "Meta+3" });
  });

  it("a group header's ⌘ digit and the drag handle's keys are the registry's", () => {
    const arrange = { group: "g", enabled: true, grip: vi.fn(), drop: null, dragging: false };
    render(<PlaceHeader place={null} label="Work" count={1} needs={0} folded={null} onToggle={vi.fn()} onOpen={vi.fn()} shortcut={3} arrange={arrange} />);
    expect(screen.getByTestId("place-page").getAttribute("aria-keyshortcuts")).toBe(`${placeKey(3).aria} ${ariaOf("rail-move")}`);
    expect(screen.getByTestId("place-open").getAttribute("title")).toBe(`Bring it forward (${placeKey(3).words})`);
    expect(screen.getByTestId("order-grip").getAttribute("title")).toBe(`Drag to move Work (or ${keysOf("rail-move").join(" and ")})`);
    cleanup();
    render(<Grip arrange={arrange} label="Fix billing" />);
    expect(screen.getByTestId("order-grip").getAttribute("title")).toBe("Drag to move Fix billing (or ⌥⌘↑ and ⌥⌘↓)");
  });
});
