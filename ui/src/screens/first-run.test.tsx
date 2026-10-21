/**
 * D-25 vitest — first run: what's already here (grouped, preselected,
 * one button), the sign-in list (one row each, Sign in to all), and the
 * platform's computer access (polled; skipped when the OS needs none).
 * AJ-E1022: a fresh account (a free name filled in → the same sign-in)
 * and a folder (the picker → a check of what's in it → a copy), which
 * replaced the old account wizard.
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FirstRun, freeLabel, groupOf, groupsFor, labelOf, outcomeWords, plainImportError, preselected, usedWords } from "./FirstRun";
import type { DiscoveredInfo } from "../lib/bridge";

const w = () => window as unknown as Record<string, unknown>;
const NOW = Math.floor(Date.now() / 1000);

function found(path: string, origin: string, extra: Partial<DiscoveredInfo> = {}): DiscoveredInfo {
  return {
    harness: "claude-code",
    path,
    origin,
    login: "logged_in",
    bytes: 1000,
    last_used_secs: NOW - 3600,
    managed: false,
    notes: [],
    signals: [origin],
    inference: {
      purpose: "Main",
      purpose_confidence: "likely",
      login: "signed-in",
      login_confidence: "found",
      activity: "active",
      activity_confidence: "found",
      activity_words: "Active this week",
      keychain_bound: false,
    },
    ...extra,
  } as DiscoveredInfo;
}

const PROFILES = "/Users/you/Library/Application Support/ClaudeCodeProfiles";
const SCAN = {
  installs: [
    found("/Users/you/.claude", "vendor-default"),
    found(`${PROFILES}/work-a`, "manager-tool", { last_used_secs: NOW - 60 }),
    found(`${PROFILES}/personal-max`, "manager-tool"),
    found("/Users/you/.claude-old", "home-pattern", { login: "logged_out", last_used_secs: NOW - 90 * 86_400 }),
    found(`${PROFILES}/already`, "manager-tool", { managed: true }),
  ],
  probes: [{ label: "Claude desktop profiles", health: "ok", detail: "3 profiles", reason: null }],
  seeds: [],
  desktop_apps: [],
  cancelled: false,
};

afterEach(cleanup);
beforeEach(() => {
  for (const key of Object.keys(w()).filter((key) => key.startsWith("__AIJ_"))) delete w()[key];
  w().__AIJ_SCAN_REPORT__ = SCAN;
});

describe("D-25 first run — the pure rules", () => {
  it("groups by kind of setup — profiles, the default, others — not by tool", () => {
    expect(groupOf(SCAN.installs[0]!)).toBe("claude-code:default");
    expect(groupOf(SCAN.installs[1]!)).toBe("claude-code:profiles");
    expect(groupOf(found(`${PROFILES}/x`, "env-override"))).toBe("claude-code:profiles");
    expect(groupOf(found("/Users/you/.claude-work", "env-override"))).toBe("claude-code:other");
    expect(groupOf(SCAN.installs[3]!)).toBe("claude-code:other");
  });

  it("names: the default ~/.claude reads 'Default' (it isn't only the terminal's); others keep their folder name", () => {
    expect(labelOf(SCAN.installs[0]!)).toBe("Default");
    expect(labelOf(SCAN.installs[1]!)).toBe("work-a");
    expect(labelOf(SCAN.installs[3]!)).toBe("claude-old");
  });

  it("preselects what's signed in or used in 30 days, never what's already in", () => {
    expect(preselected(SCAN.installs[1]!, NOW)).toBe(true);
    expect(preselected(SCAN.installs[3]!, NOW)).toBe(false);
    expect(preselected(SCAN.installs[4]!, NOW)).toBe(false);
  });

  it("says where a setup is open right now, from the running processes", () => {
    expect(usedWords(found("/Users/you/.claude", "vendor-default", { in_use: ["desktop"] }), NOW)).toBe("Open in the Claude app");
    expect(usedWords(found("/x", "home-pattern", { in_use: ["terminal"] }), NOW)).toBe("Running in a terminal");
    expect(usedWords(found("/x", "home-pattern", { in_use: ["desktop", "terminal"] }), NOW)).toBe("Open in the Claude app and a terminal");
  });

  it("an import error drops the internal path prefix; the row already names the setup", () => {
    expect(
      plainImportError(
        "account directory error at /Users/you/Library/Application Support/AI Juggler/accounts/claude-code/x/skills/a: cannot copy skills/a — the partial import was removed",
      ),
    ).toBe("Cannot copy skills/a — the partial import was removed");
    expect(plainImportError("Disk full — this one couldn't be copied.")).toBe("Disk full — this one couldn't be copied.");
  });

  // AJ-E1525: the titles are each tool's own (its spec), so Claude's words
  // never reach another tool's groups (review of #72), and a tool no spec
  // describes gets one neutral group.
  it("group titles come from each tool's spec, Claude's words only for Claude Code", () => {
    const titles = (tool: string) => groupsFor([tool]).map((group) => group.title);
    expect(titles("claude-code")).toEqual(["Your Claude profiles", "The default setup", "Other Claude setups"]);
    expect(titles("codex")).toEqual(["Codex"]);
    expect(titles("acme")).toEqual(["acme setups"]);
  });

  it("the result line says what came in, what didn't and what already was", () => {
    const base = { stopped: false, done: 0, waiting: 0, failed: 0, already: [] as string[] };
    expect(outcomeWords({ ...base, done: 5, failed: 1 })).toBe("5 brought in. 1 couldn't come in.");
    expect(outcomeWords({ ...base, failed: 2 })).toBe("2 couldn't come in.");
    expect(outcomeWords({ ...base, stopped: true, done: 1, waiting: 2 })).toBe("Stopped. 1 brought in, 2 still to go.");
    expect(outcomeWords({ ...base, already: ["work-a", "work-b"] })).toBe("2 were already in AI Juggler.");
    expect(outcomeWords({ ...base, done: 2, already: ["Default"] })).toBe("2 brought in. Default was already in AI Juggler.");
  });

  it("says how recently each was used in plain words", () => {
    expect(usedWords(SCAN.installs[1]!, NOW)).toBe("In use now");
    expect(usedWords(SCAN.installs[0]!, NOW)).toBe("Used today");
    expect(usedWords(found("/x", "home-pattern", { last_used_secs: NOW - 3 * 86_400 }), NOW)).toBe("Used 3 days ago");
  });
});

describe("D-25 first run — the flow", () => {
  const renderIt = (props: Partial<Parameters<typeof FirstRun>[0]> = {}) =>
    render(
      <FirstRun
        mode={props.mode}
        onDone={props.onDone ?? (() => null)}
        onAccountsChanged={props.onAccountsChanged}
        openLoginTerminal={props.openLoginTerminal ?? (() => null)}
      />,
    );

  it("shows what's here, grouped, with one button that counts the picks", async () => {
    const { findByTestId, getByTestId } = renderIt();
    const list = await findByTestId("first-run-found");
    expect(list.textContent).toContain("You already have 5 Claude Code setups on this Mac.");
    expect(getByTestId("first-run-group-profiles").textContent).toContain("work-a");
    expect(getByTestId("first-run-group-default").textContent).toContain("Default");
    expect(getByTestId("first-run-group-other").textContent).toContain("claude-old");
    expect(list.textContent).toContain("Already in AI Juggler");
    expect(getByTestId("first-run-bring-in").textContent).toBe("Bring in 3 accounts");
    fireEvent.click(within(getByTestId("first-run-group-other")).getByRole("checkbox"));
    expect(getByTestId("first-run-bring-in").textContent).toBe("Bring in 4 accounts");
  });

  it("shows where it looked, on request", async () => {
    const { findByRole, getByTestId } = renderIt();
    fireEvent.click(await findByRole("button", { name: "Where we looked" }));
    expect(getByTestId("first-run-looked").textContent).toContain("Claude desktop profiles checked — 3 profiles");
  });

  it("copies: brings them in, then lists each sign-in once, with Sign in to all", async () => {
    const opened: string[] = [];
    const { findByTestId, getByTestId, findAllByTestId } = renderIt({
      openLoginTerminal: (_id, title) => opened.push(title),
    });
    await findByTestId("first-run-found");
    fireEvent.click(within(getByTestId("first-run-placement-copy")).getByRole("radio"));
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    const signin = await findByTestId("first-run-signin");
    const rows = await findAllByTestId("first-run-signin-row");
    expect(rows.length).toBeGreaterThan(0);
    expect(signin.textContent).toMatch(/need a quick sign-in|needs a quick sign-in/);
    const all = within(signin).queryByRole("button", { name: /Sign in to all/ });
    if (rows.length > 1) expect(all).not.toBeNull();
    await act(async () => {
      fireEvent.click(within(rows[0]!).getByRole("button", { name: /^Sign in/ }));
    });
    await waitFor(() => expect(opened.length).toBe(1));
    expect(opened[0]).toMatch(/^Sign in · /);
  });

  it("computer access: Allow asks the system, the entry flips to Allowed, Start working finishes", async () => {
    const onDone = vi.fn();
    const { findByTestId, getByTestId } = renderIt({ onDone });
    await findByTestId("first-run-found");
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    const access = await findByTestId("first-run-access");
    expect(access.textContent).toContain("2 optional permissions make it smoother.");
    await act(async () => {
      fireEvent.click(within(getByTestId("first-run-access-preview")).getByRole("button", { name: /^Allow/ }));
    });
    await waitFor(() => expect(getByTestId("first-run-access-preview").textContent).toContain("Allowed"));
    expect(w().__AIJ_OS_ACCESS_CALLS__).toEqual(["request:preview"]);
    fireEvent.click(getByTestId("first-run-start"));
    expect(onDone).toHaveBeenCalledOnce();
  });

  it("an entry already declined offers its settings pane instead of a prompt", async () => {
    w().__AIJ_OS_ACCESS__ = [
      {
        id: "focus",
        title: "Jump to a session's own window",
        why: "Brings the right window forward.",
        system_name: "Accessibility and Automation",
        status: "not_allowed",
        action: "settings",
        note: null,
      },
    ];
    const { findByTestId, getByTestId } = renderIt();
    await findByTestId("first-run-found");
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    const entry = await findByTestId("first-run-access-focus");
    await act(async () => {
      fireEvent.click(within(entry).getByRole("button", { name: /Open System Settings/ }));
    });
    expect(w().__AIJ_OS_ACCESS_CALLS__).toEqual(["settings:focus"]);
  });

  it("where the OS needs nothing, there is no access step", async () => {
    w().__AIJ_OS_ACCESS__ = [];
    const onDone = vi.fn();
    const { findByTestId, getByTestId, queryByText } = renderIt({ onDone });
    await findByTestId("first-run-found");
    expect(queryByText("Computer access")).toBeNull();
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    await waitFor(() => expect(onDone).toHaveBeenCalledOnce());
  });

  it("ADR-0018: uses them where they are by default — the choice is in view, and signed-in setups stay signed in", async () => {
    w().__AIJ_OS_ACCESS__ = [];
    const onDone = vi.fn();
    const { findByTestId, getByTestId, queryByTestId } = renderIt({ onDone });
    const found = await findByTestId("first-run-found");
    const inPlace = within(getByTestId("first-run-placement-in_place")).getByRole("radio") as HTMLInputElement;
    const copy = within(getByTestId("first-run-placement-copy")).getByRole("radio") as HTMLInputElement;
    expect(inPlace.checked).toBe(true);
    expect(copy.checked).toBe(false);
    expect(found.textContent).toContain("uses each one where it is");
    fireEvent.click(copy);
    expect(found.textContent).toContain("makes its own copy of each");
    fireEvent.click(inPlace);
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    // Nothing to sign in to: straight on (no access step on this OS).
    await waitFor(() => expect(onDone).toHaveBeenCalledOnce());
    expect(queryByTestId("first-run-signin")).toBeNull();
  });

  it("Codex added on its own offers the same choice, worded for Codex (ADR-0018)", async () => {
    w().__AIJ_SCAN_REPORT__ = { ...SCAN, installs: [found("/Users/you/.codex", "vendor-default", { harness: "codex" } as Partial<DiscoveredInfo>)] };
    const bridge = await import("../lib/bridge");
    const spy = vi.spyOn(bridge, "accountImport");
    const onDone = vi.fn();
    const { findByTestId, getByTestId, queryByTestId } = render(<FirstRun mode="add" provider="codex" onDone={onDone} openLoginTerminal={() => null} />);
    const list = await findByTestId("first-run-found");
    const inPlace = within(getByTestId("first-run-placement-in_place")).getByRole("radio") as HTMLInputElement;
    expect(inPlace.checked).toBe(true);
    expect(getByTestId("first-run-placement-in_place").textContent).toContain("as when you run Codex yourself");
    expect(list.textContent).not.toContain("Claude app");
    // One setup: worded for one (0.0.11 walk).
    expect(list.textContent).toContain("AI Juggler uses it where it is");
    expect(list.textContent).not.toContain("Pick the ones");
    expect(getByTestId("first-run-placement-in_place").textContent).toContain("Use it where it is");
    expect(getByTestId("first-run-placement-copy").textContent).toContain("Make a separate copy");
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    expect(spy.mock.calls.map((call) => [call[0], call[6]])).toEqual([["codex", true]]);
    // Adding later never asks for computer access again (first run did).
    await waitFor(() => expect(onDone).toHaveBeenCalledOnce());
    expect(queryByTestId("first-run-access")).toBeNull();
    spy.mockRestore();
  });

  it("one failing never blocks the rest: Continue with the ones that came in", async () => {
    w().__AIJ_IMPORT_FAIL_PATHS__ = ["work-a"];
    const { findByTestId, getByTestId, queryByTestId } = renderIt();
    await findByTestId("first-run-found");
    fireEvent.click(within(getByTestId("first-run-placement-copy")).getByRole("radio"));
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    const outcome = await findByTestId("first-run-outcome");
    expect(outcome.textContent).toContain("2 brought in. 1 couldn't come in.");
    expect(within(getByTestId("first-run-failed")).getByText("work-a")).toBeTruthy();
    expect(queryByTestId("first-run-bring-in")).toBeNull();
    await act(async () => {
      fireEvent.click(getByTestId("first-run-continue"));
    });
    // The two copies that came in go on to their sign-ins.
    expect((await findByTestId("first-run-signin")).textContent).toMatch(/sign-in/);
    expect((await findByTestId("first-run-signin")).textContent).not.toContain("work-a");
  });

  it("Try again retries only the failed ones and keeps the others' sign-ins", async () => {
    w().__AIJ_IMPORT_FAIL_PATHS__ = ["work-a"];
    const { findByTestId, getByTestId, findAllByTestId } = renderIt();
    await findByTestId("first-run-found");
    fireEvent.click(within(getByTestId("first-run-placement-copy")).getByRole("radio"));
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    await findByTestId("first-run-outcome");
    w().__AIJ_IMPORT_FAIL_PATHS__ = [];
    await act(async () => {
      fireEvent.click(getByTestId("first-run-retry"));
    });
    await findByTestId("first-run-signin");
    expect(await findAllByTestId("first-run-signin-row")).toHaveLength(3);
  });

  it("a failed copy can come in where it is instead", async () => {
    w().__AIJ_IMPORT_FAIL_PATHS__ = ["work-a"];
    const { findByTestId, getByTestId, findAllByTestId } = renderIt();
    await findByTestId("first-run-found");
    fireEvent.click(within(getByTestId("first-run-placement-copy")).getByRole("radio"));
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    await findByTestId("first-run-outcome");
    await act(async () => {
      fireEvent.click(getByTestId("first-run-use-in-place"));
    });
    // work-a is in, signed in where it is; the two copies still sign in.
    await findByTestId("first-run-signin");
    expect(await findAllByTestId("first-run-signin-row")).toHaveLength(2);
  });

  it("Stop (Close or Esc while bringing in) finishes the one in progress and keeps what came in", async () => {
    w().__AIJ_IMPORT_DELAY_MS__ = 40;
    const onDone = vi.fn();
    const { findByTestId, getByTestId } = renderIt({ onDone });
    await findByTestId("first-run-found");
    act(() => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    const close = getByTestId("first-run-close");
    await waitFor(() => expect(close.textContent).toBe("Stop"));
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(close.textContent).toBe("Stopping after this one…"));
    const outcome = await findByTestId("first-run-outcome", {}, { timeout: 2000 });
    expect(outcome.textContent).toContain("Stopped. 1 brought in, 2 still to go.");
    expect(onDone).not.toHaveBeenCalled();
    expect(close.textContent).toBe("Not now");
    expect(document.activeElement).toBe(getByTestId("first-run-continue"));
    await act(async () => {
      fireEvent.click(getByTestId("first-run-rest"));
    });
    // The other two come in, then it goes on (in place: no sign-ins).
    expect(await findByTestId("first-run-access", {}, { timeout: 2000 })).toBeTruthy();
  });

  it("a setup already in AI Juggler (as a copy) isn't called brought in", async () => {
    w().__AIJ_IMPORT_DUPLICATE_LABEL__ = "work-a, a copy";
    const { findByTestId, getByTestId, getAllByText } = renderIt();
    await findByTestId("first-run-found");
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    await waitFor(() => expect(getAllByText("Already in AI Juggler").length).toBeGreaterThan(0));
    expect((await findByTestId("first-run-outcome")).textContent).toContain("were already in AI Juggler.");
  });

  it("add mode (Accounts › Add account): closable, labelled, the same list", async () => {
    const onDone = vi.fn();
    const { findByTestId, getByTestId } = render(
      <FirstRun mode="add" onDone={onDone} openLoginTerminal={() => null} />,
    );
    expect((await findByTestId("first-run-found")).textContent).toContain("You already have 5 Claude Code setups");
    expect(getByTestId("first-run").getAttribute("aria-label")).toBe("Add accounts");
    fireEvent.click(getByTestId("first-run-close"));
    expect(onDone).toHaveBeenCalledOnce();
  });

  it("first run can be left too — Not now, or Escape — in both modes (0.0.7 dogfood)", async () => {
    const onDone = vi.fn();
    const first = renderIt({ onDone });
    await first.findByTestId("first-run-found");
    const close = first.getByTestId("first-run-close");
    expect(close.textContent).toBe("Not now");
    fireEvent.click(close);
    expect(onDone).toHaveBeenCalledTimes(1);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onDone).toHaveBeenCalledTimes(2);
    cleanup();

    const onAddDone = vi.fn();
    const add = render(
      <FirstRun mode="add" onDone={onAddDone} openLoginTerminal={() => null} />,
    );
    await add.findByTestId("first-run-found");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onAddDone).toHaveBeenCalledOnce();
  });

  it("an Escape a popover inside already handled leaves the screen open", async () => {
    const onDone = vi.fn();
    const { findByTestId } = render(
      <FirstRun mode="add" onDone={onDone} openLoginTerminal={() => null} />,
    );
    await findByTestId("first-run-found");
    const event = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
    event.preventDefault();
    window.dispatchEvent(event);
    expect(onDone).not.toHaveBeenCalled();
  });

  it("add mode with nothing new to bring in goes straight to a fresh account", async () => {
    w().__AIJ_SCAN_REPORT__ = { ...SCAN, installs: [found(`${PROFILES}/already`, "manager-tool", { managed: true })] };
    const { findByTestId, queryByRole } = render(<FirstRun mode="add" onDone={() => null} openLoginTerminal={() => null} />);
    expect((await findByTestId("first-run-fresh")).textContent).toContain("Add a fresh account.");
    // Nothing to go back to; a folder is still one link away.
    expect(queryByRole("button", { name: "Back" })).toBeNull();
    expect(queryByRole("button", { name: "Import from a folder instead…" })).not.toBeNull();
  });

  it("everything already allowed: no access step", async () => {
    w().__AIJ_OS_ACCESS__ = [
      { id: "focus", title: "Jump", why: "Why", system_name: "Accessibility", status: "allowed", action: "none", note: null },
    ];
    const onDone = vi.fn();
    const { findByTestId, getByTestId } = renderIt({ onDone });
    await findByTestId("first-run-found");
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    await waitFor(() => expect(onDone).toHaveBeenCalledOnce());
  });

  it("nothing found: a plain start, and Add an account opens the fresh step", async () => {
    w().__AIJ_SCAN_REPORT__ = { ...SCAN, installs: [], probes: [] };
    const { findByTestId, getByRole } = renderIt();
    expect((await findByTestId("first-run-found")).textContent).toContain("No Claude Code setups found yet.");
    fireEvent.click(getByRole("button", { name: "Add an account" }));
    expect(await findByTestId("first-run-fresh")).toBeTruthy();
  });
});

describe("AJ-E1022 a fresh account", () => {
  it("a free name: the tool's name, numbered past the ones taken", () => {
    expect(freeLabel("Claude Code", ["Work"])).toBe("Claude Code");
    expect(freeLabel("Claude Code", ["claude code", "Claude Code 2"])).toBe("Claude Code 3");
  });

  it("name it (filled in), Create and sign in, then the one sign-in row", async () => {
    w().__AIJ_FIXTURE__ = undefined;
    const changed = vi.fn();
    const opened: string[] = [];
    const { findByTestId, getByRole, getByTestId, findAllByTestId } = render(
      <FirstRun mode="add" onDone={() => null} onAccountsChanged={changed} openLoginTerminal={(_id, title) => opened.push(title)} />,
    );
    await findByTestId("first-run-found");
    fireEvent.click(getByRole("button", { name: "Add a fresh account instead" }));
    const name = (await findByTestId("first-run-fresh-name")) as HTMLInputElement;
    await waitFor(() => expect(name.value).toMatch(/^Claude Code/));
    expect(document.activeElement).toBe(name);
    fireEvent.change(name, { target: { value: "Side project" } });
    await act(async () => {
      fireEvent.click(getByTestId("first-run-fresh-create"));
    });
    const rows = await findAllByTestId("first-run-signin-row");
    expect(rows).toHaveLength(1);
    expect(rows[0]!.textContent).toContain("Side project");
    expect(changed).toHaveBeenCalledOnce();
    await act(async () => {
      fireEvent.click(within(rows[0]!).getByRole("button", { name: /^Sign in/ }));
    });
    await waitFor(() => expect(opened).toEqual(["Sign in · Side project"]));
  });

  it("an empty name can't be created", async () => {
    const { findByTestId, getByRole, getByTestId } = render(<FirstRun mode="add" onDone={() => null} openLoginTerminal={() => null} />);
    await findByTestId("first-run-found");
    fireEvent.click(getByRole("button", { name: "Add a fresh account instead" }));
    fireEvent.change(await findByTestId("first-run-fresh-name"), { target: { value: "  " } });
    expect((getByTestId("first-run-fresh-create") as HTMLButtonElement).disabled).toBe(true);
  });
});

describe("AJ-E1022 another folder", () => {
  const INSPECT = {
    total_bytes: 46_000_000,
    file_count: 128,
    folders: [
      { name: "projects/", bytes: 40_000_000 },
      { name: "settings.json", bytes: 60 },
    ],
    truncated: false,
    ceiling_bytes: 2_000_000_000,
    is_config: true,
    already_managed: false,
    login: "logged_in",
    keychain_bound: true,
  };
  const openFolder = async () => {
    const view = render(<FirstRun mode="add" onDone={onDone} openLoginTerminal={() => null} />);
    await view.findByTestId("first-run-found");
    await act(async () => {
      fireEvent.click(view.getByRole("button", { name: "Import from another folder…" }));
    });
    return view;
  };
  let onDone = vi.fn();
  beforeEach(() => {
    onDone = vi.fn();
    w().__AIJ_PICK_DIRECTORY__ = "/Users/you/old-claude";
    w().__AIJ_SOURCE_INSPECT__ = INSPECT;
  });

  it("the picker cancelled changes nothing", async () => {
    delete w().__AIJ_PICK_DIRECTORY__;
    const view = await openFolder();
    expect(view.queryByTestId("first-run-folder")).toBeNull();
    expect(view.getByTestId("first-run-found")).toBeTruthy();
  });

  it("checks what's in it before copying: size, sign-in, the Keychain note, the breakdown on request", async () => {
    const view = await openFolder();
    const check = await view.findByTestId("first-run-folder-check");
    await waitFor(() => expect(check.textContent).toContain("128 files · 43.9 MB"));
    expect(check.textContent).toContain("~/old-claude");
    expect(check.textContent).toContain("Signed in");
    expect(view.getByTestId("first-run-folder-keychain").textContent).toContain("macOS Keychain");
    expect(view.getByTestId("source-breakdown").textContent).toContain("projects/");
    expect((view.getByTestId("first-run-folder-name") as HTMLInputElement).value).toBe("old-claude");
  });

  it("Bring it in copies it and asks for the sign-in the Keychain needs", async () => {
    const view = await openFolder();
    await act(async () => {
      fireEvent.click(await view.findByTestId("first-run-folder-bring-in"));
    });
    const rows = await view.findAllByTestId("first-run-signin-row");
    expect(rows).toHaveLength(1);
    expect(rows[0]!.textContent).toContain("old-claude");
  });

  it("too large: settings only, or a fresh account — nothing else", async () => {
    w().__AIJ_SOURCE_INSPECT__ = { ...INSPECT, total_bytes: 40_000_000_000 };
    const view = await openFolder();
    expect((await view.findByTestId("first-run-folder-large")).textContent).toContain("Bring in the settings only");
    expect(view.getByTestId("first-run-folder-bring-in").textContent).toBe("Bring in settings only");
    fireEvent.click(view.getByRole("button", { name: "Add a fresh account instead" }));
    expect(await view.findByTestId("first-run-fresh")).toBeTruthy();
  });

  it("not a setup: says so plainly and offers another folder, no Bring it in", async () => {
    w().__AIJ_SOURCE_INSPECT__ = { ...INSPECT, is_config: false };
    const view = await openFolder();
    const alert = await view.findByTestId("source-not-config");
    expect(alert.getAttribute("role")).toBe("alert");
    expect(alert.textContent).toContain("doesn't look like a Claude Code setup");
    expect(view.queryByTestId("first-run-folder-bring-in")).toBeNull();
    expect(view.getByTestId("first-run-folder-again")).toBeTruthy();
  });

  it("already brought in: the error names it, and Done closes", async () => {
    w().__AIJ_IMPORT_DUPLICATE_LABEL__ = "Work";
    const view = await openFolder();
    await act(async () => {
      fireEvent.click(await view.findByTestId("first-run-folder-bring-in"));
    });
    const dup = await view.findByTestId("first-run-folder-duplicate");
    expect(dup.textContent).toContain("Already imported as «Work»");
    fireEvent.click(within(dup).getByRole("button", { name: "Done" }));
    expect(onDone).toHaveBeenCalledOnce();
  });

  it("moved or gone since the pick: says so, and the picker is one click away", async () => {
    w().__AIJ_IMPORT_ERROR__ = { code: "io", retryable: false, user_message: "/Users/you/old-claude does not exist" };
    const view = await openFolder();
    await act(async () => {
      fireEvent.click(await view.findByTestId("first-run-folder-bring-in"));
    });
    expect((await view.findByTestId("first-run-folder-moved")).textContent).toContain("That folder moved or is gone.");
    w().__AIJ_PICK_DIRECTORY__ = "/Users/you/other-claude";
    delete w().__AIJ_IMPORT_ERROR__;
    await act(async () => {
      fireEvent.click(view.getByTestId("first-run-folder-again"));
    });
    await waitFor(() => expect(view.getByTestId("first-run-folder-check").textContent).toContain("~/other-claude"));
  });
});

describe("AJ-E1103 onboarding.login — how first run's sign-ins end", () => {
  const events = () =>
    ((w().__AIJ_STATS_EVENTS__ as { metric: string; dims: Record<string, string> }[] | undefined) ?? []).filter(
      (event) => event.metric === "onboarding.login",
    );

  it("Sign in later counts each one left as skipped, with the tool and no names", async () => {
    const { findByTestId, getByTestId } = render(<FirstRun onDone={() => null} openLoginTerminal={() => null} />);
    await findByTestId("first-run-found");
    // Copies need a sign-in each (used where they are, they keep theirs).
    fireEvent.click(within(getByTestId("first-run-placement-copy")).getByRole("radio"));
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    const rows = await waitFor(() => {
      const found = document.querySelectorAll("[data-testid='first-run-signin-row']");
      expect(found.length).toBeGreaterThan(0);
      return found;
    });
    await act(async () => {
      fireEvent.click(getByTestId("first-run-signin-next"));
    });
    expect(events()).toHaveLength(rows.length);
    for (const event of events()) expect(event.dims).toEqual({ provider: "claude-code", outcome: "skipped" });
  });

  it("a folder whose sign-in has expired counts as expired", async () => {
    w().__AIJ_PICK_DIRECTORY__ = "/Users/you/old-claude";
    const { findByTestId, getByRole } = render(<FirstRun mode="add" onDone={() => null} openLoginTerminal={() => null} />);
    await findByTestId("first-run-found");
    // The preview import reports logged_out; an expired one is the backend's word.
    const bridge = await import("../lib/bridge");
    const spy = vi.spyOn(bridge, "accountImport").mockResolvedValue({
      account: { id: "acct-x", provider: "claude-code", label: "old-claude", plan: null, login_state: "expired", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false },
      files_copied: 1,
      bytes_copied: 1,
      folders: [],
      degraded: false,
      skipped: [],
      login: "expired",
      login_note: null,
    } as unknown as Awaited<ReturnType<typeof bridge.accountImport>>);
    await act(async () => {
      fireEvent.click(getByRole("button", { name: "Import from another folder…" }));
    });
    await act(async () => {
      fireEvent.click(await findByTestId("first-run-folder-bring-in"));
    });
    await waitFor(() => expect(events()).toEqual([{ metric: "onboarding.login", dims: { provider: "claude-code", outcome: "expired" }, value: undefined }]));
    spy.mockRestore();
  });
});

describe("AJ-E1301 — Codex setups in first run", () => {
  const codex = (path: string, origin: string, extra: Partial<DiscoveredInfo> = {}) =>
    found(path, origin, { harness: "codex", ...extra });

  it("groups Codex homes under their own heading and names the standard one", () => {
    expect(groupOf(codex("/Users/you/.codex", "vendor-default"))).toBe("codex:codex");
    expect(labelOf(codex("/Users/you/.codex", "vendor-default"))).toBe("Codex");
    expect(labelOf(codex("/Users/you/.codex-work", "home-pattern"))).toBe("codex-work");
  });

  it("lists Codex beside Claude, preselected, and says Codex isn't installed when it isn't", async () => {
    w().__AIJ_SCAN_REPORT__ = {
      ...SCAN,
      installs: [...SCAN.installs, codex("/Users/you/.codex", "vendor-default")],
    };
    const view = render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
    const group = await view.findByTestId("first-run-group-codex");
    expect(within(group).getByTestId("first-run-row").textContent).toContain("~/.codex");
    const box = within(group).getByRole("checkbox") as HTMLInputElement;
    expect(box.checked).toBe(true);
    expect(view.getByRole("heading", { level: 1 }).textContent).toBe(
      "You already have 6 setups on this Mac.",
    );
    // The default mock has Codex not installed.
    expect((await view.findByTestId("first-run-no-codex")).textContent).toContain("npm install -g @openai/codex");
  });

  it("while looking, first run names no single tool; Add account names its own (0.0.11 walk)", async () => {
    const first = render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
    expect(first.getByTestId("first-run-scanning").textContent).toContain("Looking for your setups on this Mac");
    await first.findByTestId("first-run-found");
    cleanup();
    const add = render(<FirstRun mode="add" provider="codex" onDone={() => null} openLoginTerminal={() => null} />);
    expect(add.getByTestId("first-run-scanning").textContent).toContain("Looking for Codex on this Mac");
    await add.findByTestId("first-run-found");
  });

  it("keeps the Claude-only screen exactly as it was", async () => {
    const view = render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
    await view.findByTestId("first-run-found");
    expect(view.queryByTestId("first-run-group-codex")).toBeNull();
    expect(view.queryByTestId("first-run-no-codex")).toBeNull();
    expect(view.getByRole("heading", { level: 1 }).textContent).toBe(
      "You already have 5 Claude Code setups on this Mac.",
    );
  });

  it("Codex rows come in where they are too, by default", async () => {
    w().__AIJ_SCAN_REPORT__ = {
      ...SCAN,
      installs: [...SCAN.installs, codex("/Users/you/.codex", "vendor-default")],
    };
    const bridge = await import("../lib/bridge");
    const spy = vi.spyOn(bridge, "accountImport");
    const { findByTestId, getByTestId, queryByTestId } = render(
      <FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />,
    );
    await findByTestId("first-run-group-codex");
    // Every tool on the list can be used where it is: no copies-only note.
    expect(queryByTestId("first-run-copy-only")).toBeNull();
    expect(getByTestId("first-run-found").textContent).toContain("AI Juggler uses each one where it is");
    expect(getByTestId("first-run-placement-in_place").textContent).toContain("as wherever you use them now");
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    expect(spy.mock.calls.find((call) => call[0] === "codex")?.[6]).toBe(true);
    expect(spy.mock.calls.every((call) => call[6] === true)).toBe(true);
    spy.mockRestore();
  });

  it("with copies of both tools, the sign-in step names both", async () => {
    w().__AIJ_SCAN_REPORT__ = {
      ...SCAN,
      installs: [...SCAN.installs, codex("/Users/you/.codex", "vendor-default")],
    };
    const { findByTestId, getByTestId } = render(
      <FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />,
    );
    await findByTestId("first-run-group-codex");
    fireEvent.click(getByTestId("first-run-placement-copy").querySelector("input")!);
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    const signin = await findByTestId("first-run-signin");
    expect(signin.textContent).toContain("Claude Code's or Codex's (OpenAI)");
  });

  it("adding for one tool lists only that tool's setups", async () => {
    w().__AIJ_SCAN_REPORT__ = {
      ...SCAN,
      installs: [...SCAN.installs, codex("/Users/you/.codex", "vendor-default")],
    };
    const codexOnly = render(<FirstRun mode="add" provider="codex" onDone={() => null} openLoginTerminal={() => null} />);
    await codexOnly.findByTestId("first-run-group-codex");
    expect(codexOnly.getAllByTestId("first-run-row")).toHaveLength(1);
    expect(codexOnly.queryByTestId("first-run-group-profiles")).toBeNull();
    cleanup();
    const claudeOnly = render(<FirstRun mode="add" provider="claude-code" onDone={() => null} openLoginTerminal={() => null} />);
    await claudeOnly.findByTestId("first-run-found");
    expect(claudeOnly.queryByTestId("first-run-group-codex")).toBeNull();
  });

  it("first-run sign-in counts are by each account's own tool, no names", async () => {
    w().__AIJ_SCAN_REPORT__ = {
      ...SCAN,
      installs: [
        found("/Users/you/.claude-old", "home-pattern", { login: "logged_out" }),
        codex("/Users/you/.codex", "vendor-default", { login: "logged_out" }),
      ],
    };
    const { findByTestId, getByTestId } = render(<FirstRun onDone={() => null} openLoginTerminal={() => null} />);
    await findByTestId("first-run-group-codex");
    // Copies, so both tools' accounts need their sign-in.
    fireEvent.click(getByTestId("first-run-placement-copy").querySelector("input")!);
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    await findByTestId("first-run-signin");
    await act(async () => {
      fireEvent.click(getByTestId("first-run-signin-next"));
    });
    const providers = ((w().__AIJ_STATS_EVENTS__ as { metric: string; dims: Record<string, string> }[] | undefined) ?? [])
      .filter((event) => event.metric === "onboarding.login")
      .map((event) => event.dims.provider)
      .sort();
    expect(providers).toEqual(["claude-code", "codex"]);
  });

  it("an all-Codex machine is named for Codex", async () => {
    w().__AIJ_SCAN_REPORT__ = { ...SCAN, installs: [codex("/Users/you/.codex", "vendor-default")] };
    const view = render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
    await view.findByTestId("first-run-group-codex");
    expect(view.getByRole("heading", { level: 1 }).textContent).toBe(
      "You already have a Codex setup on this Mac.",
    );
  });

  it("when the capabilities call fails, the choice stays and in place is preselected", async () => {
    w().__AIJ_SCAN_REPORT__ = {
      ...SCAN,
      installs: [...SCAN.installs, codex("/Users/you/.codex", "vendor-default")],
    };
    w().__AIJ_PROVIDER_CAPABILITIES_ERROR__ = { code: "io", user_message: "the app is still starting" };
    // Install probes failing too changes nothing about the choice.
    w().__AIJ_PROVIDER_INSTALLS_ERROR__ = { code: "io", user_message: "probe failed" };
    const bridge = await import("../lib/bridge");
    const spy = vi.spyOn(bridge, "accountImport");
    const { findByTestId, getByTestId } = render(
      <FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />,
    );
    await findByTestId("first-run-group-codex");
    const inPlace = within(getByTestId("first-run-placement-in_place")).getByRole("radio") as HTMLInputElement;
    expect(inPlace.checked).toBe(true);
    expect(getByTestId("first-run-placement-copy")).toBeTruthy();
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    expect(spy.mock.calls.length).toBeGreaterThan(0);
    expect(spy.mock.calls.every((call) => call[6] === true)).toBe(true);
    spy.mockRestore();
  });

  it("a tool the backend can't use in place comes in as a copy, with no choice", async () => {
    w().__AIJ_SCAN_REPORT__ = { ...SCAN, installs: [codex("/Users/you/.codex", "vendor-default")] };
    w().__AIJ_PROVIDER_CAPABILITIES__ = [
      { provider: "claude-code", can_use_in_place: true },
      { provider: "codex", can_use_in_place: false },
    ];
    const bridge = await import("../lib/bridge");
    const spy = vi.spyOn(bridge, "accountImport");
    const view = render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
    await view.findByTestId("first-run-group-codex");
    expect(view.queryByTestId("first-run-placement")).toBeNull();
    expect(view.getByTestId("first-run-found").textContent).toContain("AI Juggler makes its own copy;");
    await act(async () => {
      fireEvent.click(view.getByTestId("first-run-bring-in"));
    });
    expect(spy.mock.calls.map((call) => [call[0], call[6]])).toEqual([["codex", false]]);
    spy.mockRestore();
  });

  it("a Codex-only first run offers the choice for Codex and no Claude note", async () => {
    w().__AIJ_SCAN_REPORT__ = { ...SCAN, installs: [codex("/Users/you/.codex", "vendor-default")] };
    // Neither tool installed: only Codex's note, over its own rows.
    w().__AIJ_PROVIDER_INSTALLS__ = [
      { provider: "claude-code", display_name: "Claude Code", install_url: null, status: "not_installed", version: null, binary: null, notes: [] },
      { provider: "codex", display_name: "Codex", install_url: null, status: "not_installed", version: null, binary: null, notes: [] },
    ];
    const bridge = await import("../lib/bridge");
    const spy = vi.spyOn(bridge, "accountImport");
    const view = render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);
    await view.findByTestId("first-run-group-codex");
    expect(view.getByTestId("first-run-placement-in_place").textContent).toContain("as when you run Codex yourself");
    expect(view.queryByTestId("first-run-copy-only")).toBeNull();
    const text = view.getByTestId("first-run-found").textContent ?? "";
    expect(text).toContain("AI Juggler uses it where it is");
    expect(text).not.toContain("Claude app");
    expect(await view.findByTestId("first-run-no-codex")).toBeTruthy();
    expect(view.queryByTestId("first-run-no-cli")).toBeNull();
    await act(async () => {
      fireEvent.click(view.getByTestId("first-run-bring-in"));
    });
    expect(spy.mock.calls.map((call) => [call[0], call[6]])).toEqual([["codex", true]]);
    spy.mockRestore();
  });

  it("a failed Codex copy can come in where it is instead", async () => {
    w().__AIJ_SCAN_REPORT__ = {
      ...SCAN,
      installs: [...SCAN.installs, codex("/Users/you/.codex", "vendor-default")],
    };
    w().__AIJ_IMPORT_FAIL_PATHS__ = ["/Users/you/.codex"];
    const bridge = await import("../lib/bridge");
    const spy = vi.spyOn(bridge, "accountImport");
    const { findByTestId, getByTestId } = render(
      <FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />,
    );
    await findByTestId("first-run-group-codex");
    fireEvent.click(within(getByTestId("first-run-placement-copy")).getByRole("radio"));
    await act(async () => {
      fireEvent.click(getByTestId("first-run-bring-in"));
    });
    const failed = await findByTestId("first-run-failed");
    expect(failed.textContent).toContain("Codex");
    spy.mockClear();
    await act(async () => {
      fireEvent.click(getByTestId("first-run-use-in-place"));
    });
    expect(spy.mock.calls.map((call) => [call[0], call[6]])).toEqual([["codex", true]]);
    spy.mockRestore();
  });

});
