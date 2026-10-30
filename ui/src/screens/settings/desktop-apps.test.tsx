/**
 * AJ-E2110 vitest — Settings › Desktop apps: each app with what was
 * found open in it and its switch; the exact window offered here (never
 * from a click on Go to it); the key that opens AI Juggler from
 * anywhere (none until chosen, ⌥Space refused); and Last messages, on
 * until turned off.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DesktopAppsPane } from "./DesktopAppsPane";
import { ConversationsProvider } from "../../lib/conversations";
import { appsFound, foundWords, useHiddenApps } from "../../lib/desktop-apps";
import { hotkeyRefusal, hotkeyWords, toAccelerator } from "../../lib/hotkey";
import { SHOW_LAST_MESSAGES_KEY, showLastMessages } from "../../lib/last-messages";
import { GO_TO_HOTKEY_EVENT } from "../../lib/outside";
import type { ConversationInfo } from "../../lib/ipc/conversations";

type Seams = { __AIJ_APP_SETTINGS__?: Record<string, string>; __AIJ_OS_ACCESS__?: unknown; __AIJ_OS_ACCESS_CALLS__?: string[] };
const seams = () => window as unknown as Seams;
const stored = () => seams().__AIJ_APP_SETTINGS__ ?? {};

afterEach(() => {
  cleanup();
  delete seams().__AIJ_APP_SETTINGS__;
  delete seams().__AIJ_OS_ACCESS__;
  delete seams().__AIJ_OS_ACCESS_CALLS__;
});

const pane = () =>
  render(
    <ConversationsProvider>
      <DesktopAppsPane />
    </ConversationsProvider>,
  );

describe("Show sessions from", () => {
  it("lists each app with what was found open in it", async () => {
    pane();
    const claude = await screen.findByTestId("desktop-app-claude-app");
    await waitFor(() => expect(claude.textContent).toContain("1 open · Work Max"));
    expect(screen.getByTestId("desktop-app-terminal").textContent).toContain("Nothing open now.");
    // One name for it everywhere: ChatGPT, the host registry's.
    expect(screen.getByTestId("desktop-app-codex-app").textContent).toContain("ChatGPT");
    expect(screen.getByTestId("desktop-app-cursor")).toBeTruthy();
    const section = claude.closest("section")!;
    expect(section.textContent).toContain("It never types into them.");
  });

  it("an app is shown until turned off, and only off is kept", async () => {
    pane();
    const toggle = await screen.findByRole("switch", { name: "Claude app" });
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    await act(async () => {
      fireEvent.click(toggle);
    });
    await waitFor(() => expect(toggle.getAttribute("aria-checked")).toBe("false"));
    expect(stored()["show_sessions_from.claude-app"]).toBe("0");
    await act(async () => {
      fireEvent.click(toggle);
    });
    await waitFor(() => expect(toggle.getAttribute("aria-checked")).toBe("true"));
    expect(stored()["show_sessions_from.claude-app"]).toBe("");
  });

  it("an app found open but not listed joins the list by itself", () => {
    const live = {
      id: "c1",
      live: true,
      places: [{ app: "Ghostty", app_id: "ghostty", profile: "Work", instance: null, pid: 1, label: "Ghostty · Work" }],
    } as unknown as ConversationInfo;
    const found = appsFound([live]);
    const ghostty = found.find((entry) => entry.app.id === "ghostty")!;
    expect(ghostty.app.name).toBe("Ghostty");
    expect(foundWords(ghostty)).toBe("1 open · Work");
    expect(found.slice(0, 4).map((entry) => entry.app.id)).toEqual(["claude-app", "terminal", "codex-app", "cursor"]);
  });

  it("the views read the same choice", async () => {
    seams().__AIJ_APP_SETTINGS__ = { "show_sessions_from.cursor": "0" };
    const seen: Array<ReadonlySet<string>> = [];
    function Probe() {
      seen.push(useHiddenApps(["claude-app", "cursor"]));
      return null;
    }
    render(<Probe />);
    await waitFor(() => expect([...seen.at(-1)!]).toEqual(["cursor"]));
  });
});

describe("Going to a session", () => {
  it("the exact window is set up here, and says On once macOS allows it", async () => {
    pane();
    const row = await screen.findByTestId("desktop-exact-window");
    expect(row.textContent).toContain("Needs Accessibility");
    await act(async () => {
      fireEvent.click(await within(row).findByRole("button", { name: "Set up…" }));
    });
    expect(seams().__AIJ_OS_ACCESS_CALLS__).toEqual(["request:focus"]);
    await waitFor(() => expect(row.textContent).toContain("On"));
  });

  it("has no key to open AI Juggler until one is chosen, refuses ⌥Space, and can turn it off", async () => {
    const heard = vi.fn();
    window.addEventListener(GO_TO_HOTKEY_EVENT, heard);
    pane();
    const row = await screen.findByTestId("desktop-hotkey");
    await waitFor(() => expect(row.textContent).toContain("None"));
    expect(row.textContent).toContain("ChatGPT uses it");
    fireEvent.click(within(row).getByRole("button", { name: "Choose…" }));
    const capture = within(row).getByTestId("desktop-hotkey-capture");
    fireEvent.keyDown(capture, { key: " ", altKey: true });
    expect(within(row).getByTestId("desktop-hotkey-error").textContent).toBe("ChatGPT uses ⌥Space. Pick another key.");
    expect(stored().go_to_hotkey).toBeUndefined();
    await act(async () => {
      fireEvent.keyDown(capture, { key: " ", altKey: true, ctrlKey: true });
    });
    await waitFor(() => expect(within(row).getByTestId("desktop-hotkey-key").textContent).toBe("⌃⌥Space"));
    expect(stored().go_to_hotkey).toBe("Control+Alt+Space");
    expect((heard.mock.calls[0]![0] as CustomEvent).detail).toBe("Control+Alt+Space");
    await act(async () => {
      fireEvent.click(within(row).getByRole("button", { name: "Turn off" }));
    });
    await waitFor(() => expect(row.textContent).toContain("None"));
    expect(stored().go_to_hotkey).toBe("");
    window.removeEventListener(GO_TO_HOTKEY_EVENT, heard);
  });

  it("names keys the way the menu bar does, and needs a modifier", () => {
    expect(hotkeyWords("Control+Alt+Space")).toBe("⌃⌥Space");
    expect(hotkeyWords("Super+Shift+G")).toBe("⌘⇧G");
    expect(hotkeyWords("")).toBe("");
    expect(hotkeyRefusal("G")).toBe("Use ⌃, ⌥ or ⌘ with the key.");
    expect(hotkeyRefusal("Shift+G")).toBe("Use ⌃, ⌥ or ⌘ with the key.");
    expect(hotkeyRefusal("Super+Space")).toContain("Spotlight");
    expect(hotkeyRefusal("Control+Alt+J")).toBeNull();
  });

  it("reads the physical key, so ⌥ on a Mac still names J and Space", () => {
    const mac = { ctrlKey: true, altKey: true, metaKey: false, shiftKey: false };
    expect(toAccelerator({ ...mac, key: "∆", code: "KeyJ" })).toBe("Control+Alt+J");
    expect(toAccelerator({ ...mac, key: " ", code: "Space" })).toBe("Control+Alt+Space");
    expect(toAccelerator({ ...mac, ctrlKey: false, key: " ", code: "Space" })).toBe("Alt+Space");
    expect(toAccelerator({ ...mac, key: "F5", code: "F5" })).toBe("Control+Alt+F5");
  });
});

describe("Last messages", () => {
  it("Last messages are on until you turn them off, and only off is kept", async () => {
    expect(await showLastMessages()).toBe(true);
    pane();
    const toggle = await screen.findByRole("switch", { name: "Show a session's last messages" });
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    const why = document.getElementById(toggle.getAttribute("aria-describedby")!)!.textContent!;
    expect(why).toContain("Never in the rail, the menu bar or notifications");
    expect(toggle.closest("section")!.textContent).toContain("Read from the session's transcript on this computer when you open it, and never sent. Only its subagents' prompts and results are kept, in the local cache.");
    await act(async () => {
      fireEvent.click(toggle);
    });
    await waitFor(() => expect(toggle.getAttribute("aria-checked")).toBe("false"));
    expect(stored()[SHOW_LAST_MESSAGES_KEY]).toBe("0");
    expect(await showLastMessages()).toBe(false);
    await act(async () => {
      fireEvent.click(toggle);
    });
    expect(stored()[SHOW_LAST_MESSAGES_KEY]).toBe("");
  });

  it("every switch on the pane has a name", async () => {
    pane();
    await screen.findByRole("switch", { name: "Show a session's last messages" });
    for (const toggle of screen.getAllByRole("switch")) {
      const name = toggle.getAttribute("aria-label") ?? document.getElementById(toggle.getAttribute("aria-labelledby") ?? "")?.textContent;
      expect(name, toggle.outerHTML).toBeTruthy();
    }
  });
});
