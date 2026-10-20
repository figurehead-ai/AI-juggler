/**
 * D-25 vitest — first run: what's already here (grouped, preselected,
 * one button), the sign-in list (one row each, Sign in to all), and the
 * platform's computer access (polled; skipped when the OS needs none).
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FirstRun, groupOf, labelOf, plainImportError, preselected, usedWords } from "./FirstRun";
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
    expect(groupOf(SCAN.installs[0]!)).toBe("default");
    expect(groupOf(SCAN.installs[1]!)).toBe("profiles");
    expect(groupOf(found(`${PROFILES}/x`, "env-override"))).toBe("profiles");
    expect(groupOf(found("/Users/you/.claude-work", "env-override"))).toBe("other");
    expect(groupOf(SCAN.installs[3]!)).toBe("other");
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
        onDone={props.onDone ?? (() => null)}
        onAddFresh={props.onAddFresh ?? (() => null)}
        onImportFolder={props.onImportFolder ?? (() => null)}
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
    await act(async () => {
      fireEvent.click(getByTestId("first-run-rest"));
    });
    await waitFor(() => expect(getByTestId("first-run-close").textContent).toBe("Not now"), { timeout: 2000 });
  });

  it("add mode (Accounts › Add account): closable, labelled, the same list", async () => {
    const onDone = vi.fn();
    const { findByTestId, getByTestId } = render(
      <FirstRun mode="add" onDone={onDone} onAddFresh={() => null} onImportFolder={() => null} openLoginTerminal={() => null} />,
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
      <FirstRun mode="add" onDone={onAddDone} onAddFresh={() => null} onImportFolder={() => null} openLoginTerminal={() => null} />,
    );
    await add.findByTestId("first-run-found");
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onAddDone).toHaveBeenCalledOnce();
  });

  it("an Escape a popover inside already handled leaves the screen open", async () => {
    const onDone = vi.fn();
    const { findByTestId } = render(
      <FirstRun mode="add" onDone={onDone} onAddFresh={() => null} onImportFolder={() => null} openLoginTerminal={() => null} />,
    );
    await findByTestId("first-run-found");
    const event = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
    event.preventDefault();
    window.dispatchEvent(event);
    expect(onDone).not.toHaveBeenCalled();
  });

  it("add mode with nothing new to bring in goes straight to a fresh account", async () => {
    w().__AIJ_SCAN_REPORT__ = { ...SCAN, installs: [found(`${PROFILES}/already`, "manager-tool", { managed: true })] };
    const onAddFresh = vi.fn();
    render(<FirstRun mode="add" onDone={() => null} onAddFresh={onAddFresh} onImportFolder={() => null} openLoginTerminal={() => null} />);
    await waitFor(() => expect(onAddFresh).toHaveBeenCalledOnce());
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

  it("nothing found: a plain start, and Add an account", async () => {
    w().__AIJ_SCAN_REPORT__ = { ...SCAN, installs: [], probes: [] };
    const onAddFresh = vi.fn();
    const { findByTestId, getByRole } = renderIt({ onAddFresh });
    expect((await findByTestId("first-run-found")).textContent).toContain("No Claude Code setups found yet.");
    fireEvent.click(getByRole("button", { name: "Add an account" }));
    expect(onAddFresh).toHaveBeenCalledOnce();
  });
});
