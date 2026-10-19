/**
 * D-25 vitest — first run: what's already here (grouped, preselected,
 * one button), the sign-in list (one row each, Sign in to all), and the
 * platform's computer access (polled; skipped when the OS needs none).
 */
import { act, cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FirstRun, groupOf, labelOf, preselected, usedWords } from "./FirstRun";
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
  it("groups as the user thinks of it: desktop profiles, the terminal, other setups", () => {
    expect(groupOf(SCAN.installs[0]!)).toBe("terminal");
    expect(groupOf(SCAN.installs[1]!)).toBe("desktop");
    expect(groupOf(found(`${PROFILES}/x`, "env-override"))).toBe("desktop");
    expect(groupOf(SCAN.installs[3]!)).toBe("other");
  });

  it("names: the terminal default reads 'Terminal'; others keep their folder name", () => {
    expect(labelOf(SCAN.installs[0]!)).toBe("Terminal");
    expect(labelOf(SCAN.installs[1]!)).toBe("work-a");
    expect(labelOf(SCAN.installs[3]!)).toBe("claude-old");
  });

  it("preselects what's signed in or used in 30 days, never what's already in", () => {
    expect(preselected(SCAN.installs[1]!, NOW)).toBe(true);
    expect(preselected(SCAN.installs[3]!, NOW)).toBe(false);
    expect(preselected(SCAN.installs[4]!, NOW)).toBe(false);
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
    expect(getByTestId("first-run-group-desktop").textContent).toContain("work-a");
    expect(getByTestId("first-run-group-terminal").textContent).toContain("Terminal");
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

  it("brings them in, then lists each sign-in once, with Sign in to all", async () => {
    const opened: string[] = [];
    const { findByTestId, getByTestId, findAllByTestId } = renderIt({
      openLoginTerminal: (_id, title) => opened.push(title),
    });
    await findByTestId("first-run-found");
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
    await act(async () => {
      fireEvent.click(await findByTestId("first-run-signin-next"));
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
    await act(async () => {
      fireEvent.click(await findByTestId("first-run-signin-next"));
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
    await act(async () => {
      fireEvent.click(await findByTestId("first-run-signin-next"));
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
