/**
 * AJ-E2905 vitest — Settings' search and the Keyboard pane: the index
 * holds every section every pane draws (and nothing that's gone), `/`
 * focuses the field, a match opens its pane at its section, the panes
 * with nothing that matches dim, and Keyboard changes a key (refusing a
 * taken one, with the reason), resets it, and Reset all has Undo.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { SettingsScreen } from "../Settings";
import { PANES } from "./panes";
import { searchSettings, SETTINGS_INDEX } from "./settings-index";
import { isChanged, resetForTests } from "../../lib/shortcut-bindings";

const w = () => window as unknown as Record<string, unknown>;
const PEER = {
  machine_id: "machine-mini",
  label: "Mac mini",
  fingerprint: "11AA 22BB 33CC 44DD 55EE 66FF 7788 99AA",
  permission: "view",
  permission_label: "View only",
  ask_each_time: false,
  paired_via: "code",
  paired_at_ms: 1,
  last_seen_ms: null,
  connection: "link_off",
  protocol: null,
  connection_detail: null,
};

beforeEach(() => {
  w().__AIJ_APP_SETTINGS__ = {};
  w().__AIJ_FLEET_PEERS__ = [PEER];
  localStorage.clear();
  resetForTests();
});
afterEach(() => {
  cleanup();
  delete w().__AIJ_FLEET_PEERS__;
  resetForTests();
});

const labels = Object.fromEntries(PANES.map((pane) => [pane.id, pane.label]));

describe("AJ-E2905 Settings search", () => {
  it("indexes every section every pane draws, and no section that's gone", async () => {
    const { getByTestId, findByTestId } = render(<SettingsScreen />);
    await waitFor(() => expect(getByTestId("settings-nav").querySelector("[data-pane='macs']")).not.toBeNull());
    for (const pane of PANES) {
      fireEvent.click(getByTestId("settings-nav").querySelector(`[data-pane='${pane.id}']`)!);
      const body = await findByTestId(`settings-pane-${pane.id}`);
      // Sections that load their data draw after it.
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 20));
      });
      const drawn = [...body.querySelectorAll<HTMLElement>("[data-settings-section]")].map((section) => section.dataset.settingsSection);
      const indexed = SETTINGS_INDEX.filter((entry) => entry.pane === pane.id).map((entry) => entry.anchor);
      expect(drawn.filter((id) => !indexed.includes(id!)), `${pane.id}: sections missing from settings-index.ts`).toEqual([]);
      expect(indexed.filter((id) => !drawn.includes(id)), `${pane.id}: index entries with no section`).toEqual([]);
    }
  });

  it("finds by a setting's words, title matches first", () => {
    expect(searchSettings("mute", labels)[0]).toMatchObject({ pane: "notifications", anchor: "notification-mutes" });
    expect(searchSettings("quiet hours", labels)[0]?.anchor).toBe("notifications");
    expect(searchSettings("keep alive", labels)[0]?.anchor).toBe("keep-alive");
    expect(searchSettings("inspector", labels).map((hit) => hit.pane)).toContain("keyboard");
    expect(searchSettings("zzzz", labels)).toEqual([]);
    expect(searchSettings("   ", labels)).toEqual([]);
  });

  it("`/` focuses it; ↓ and ↩ open the match at its section; others dim; Esc clears", async () => {
    const { getByTestId, findByTestId, queryByTestId } = render(<SettingsScreen />);
    const field = getByTestId("settings-search-field") as HTMLInputElement;
    fireEvent.keyDown(document.body, { key: "/" });
    expect(document.activeElement).toBe(field);
    fireEvent.change(field, { target: { value: "cache" } });
    const hits = within(getByTestId("settings-search-results")).getAllByTestId("settings-search-hit");
    expect(hits[0]!.textContent).toBe("AdvancedLocal cache");
    expect(getByTestId("settings-nav").querySelector("[data-pane='general']")!.className).toContain("opacity-45");
    expect(getByTestId("settings-nav").querySelector("[data-pane='advanced']")!.className).not.toContain("opacity-45");
    fireEvent.keyDown(field, { key: "Enter" });
    await findByTestId("settings-pane-advanced");
    expect(getByTestId("settings-nav").querySelector("[aria-current='page']")?.textContent).toBe("Advanced");
    expect(field.value).toBe("");
    // ↩ moved focus to the section; back to the field.
    expect(document.activeElement?.id).toBe("settings-local-cache-title");
    field.focus();
    fireEvent.change(field, { target: { value: "nothing like it" } });
    expect(getByTestId("settings-search-none").textContent).toContain("No setting matches");
    fireEvent.keyDown(field, { key: "Escape" });
    expect(field.value).toBe("");
    expect(queryByTestId("settings-search-results")).toBeNull();
    expect(document.activeElement).toBe(field);
    fireEvent.keyDown(field, { key: "Escape" });
    expect(document.activeElement).not.toBe(field);
  });
});

describe("AJ-E2905 Settings › Keyboard", () => {
  it("changes a key, refuses a taken one with the reason, resets, and Reset all has Undo", async () => {
    const { getByTestId, findByTestId, queryByTestId } = render(<SettingsScreen focusSection="keyboard" />);
    await findByTestId("settings-pane-keyboard");
    fireEvent.click(getByTestId("keyboard-change-palette"));
    const capture = getByTestId("keyboard-capture-palette");
    expect(document.activeElement).toBe(capture);
    fireEvent.keyDown(capture, { key: "Meta", code: "MetaLeft", metaKey: true });
    expect(queryByTestId("keyboard-refused")).toBeNull();
    fireEvent.keyDown(capture, { key: "n", code: "KeyN", metaKey: true });
    expect(getByTestId("keyboard-refused").textContent).toContain("Change that one first.");
    fireEvent.keyDown(capture, { key: "j", code: "KeyJ", metaKey: true, altKey: true });
    await waitFor(() => expect(isChanged("palette")).toBe(true));
    const row = getByTestId("settings-pane-keyboard").querySelector("[data-shortcut='palette']")!;
    expect(row.textContent).toContain("⌥⌘J");
    expect(row.textContent).toContain("changed");
    fireEvent.click(getByTestId("keyboard-reset-palette"));
    await waitFor(() => expect(isChanged("palette")).toBe(false));
    // Esc cancels a capture.
    fireEvent.click(getByTestId("keyboard-change-settings"));
    fireEvent.keyDown(getByTestId("keyboard-capture-settings"), { key: "Escape", code: "Escape" });
    expect(getByTestId("keyboard-change-settings").textContent).toBe("Change…");
    // Reset all, then Undo.
    fireEvent.click(getByTestId("keyboard-change-settings"));
    fireEvent.keyDown(getByTestId("keyboard-capture-settings"), { key: ";", code: "Semicolon", metaKey: true });
    await waitFor(() => expect(isChanged("settings")).toBe(true));
    fireEvent.click(await findByTestId("keyboard-reset-all"));
    await waitFor(() => expect(isChanged("settings")).toBe(false));
    expect(queryByTestId("keyboard-reset-all")).toBeNull();
    // Fixed ones say why; the go-to key leads to Desktop apps.
    expect(getByTestId("settings-pane-keyboard").querySelector("[data-shortcut='close']")!.textContent).toContain("Esc closes what's open everywhere.");
    // The go-to key: the same capture and refusals as Desktop apps.
    fireEvent.click(await findByTestId("keyboard-go-to-choose"));
    fireEvent.keyDown(getByTestId("keyboard-go-to-capture"), { key: " ", code: "Space", altKey: true });
    expect(getByTestId("keyboard-go-to-error").textContent).toContain("ChatGPT uses ⌥Space");
    fireEvent.keyDown(getByTestId("keyboard-go-to-capture"), { key: " ", code: "Space", altKey: true, ctrlKey: true });
    await waitFor(() => expect(getByTestId("keyboard-go-to-key").textContent).toBe("⌃⌥Space"));
  });
});
