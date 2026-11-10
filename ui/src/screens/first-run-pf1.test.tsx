/**
 * AJ-E3306 — the PF1 persona walk on rc26 (a newcomer's first run on a Mac
 * shaped like a heavy real one): every tool Home says it found is offered
 * (a tool with nothing set up as a fresh App, in its own group, unticked);
 * the desktop apps' profiles have a door; a folder named for a word is
 * brought in by that word ("Work"), twins told apart; and a setup with no
 * conversations never reads as an archive candidate.
 */
import { act, cleanup, fireEvent, render, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { desktopNoneWords, FirstRun, labelOf, setupNames, usedWords } from "./FirstRun";
import { foundWords } from "./firstRunScan";
import type { DiscoveredInfo } from "../types/generated/DiscoveredInfo";

vi.mock("../components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

const NOW = Math.floor(Date.now() / 1000);

function setup(path: string, origin: string, extra: Partial<DiscoveredInfo> = {}, inference: Partial<DiscoveredInfo["inference"]> = {}): DiscoveredInfo {
  return {
    harness: "claude-code",
    path,
    origin,
    set_by: null,
    login: "logged_in",
    bytes: 1000,
    last_used_secs: null,
    managed: false,
    notes: [],
    signals: [origin],
    in_use: [],
    inference: {
      purpose: "Main",
      purpose_confidence: "found",
      purpose_rule: "usual-folder",
      login: "signed-in",
      login_confidence: "found",
      login_rule: "sign-in-present",
      activity: "unused",
      activity_confidence: "guessed",
      activity_rule: "no-conversations",
      activity_words: "Not used yet",
      keychain_bound: false,
      ...inference,
    },
    ...extra,
  };
}

const WORK = { purpose: "Work", purpose_confidence: "likely", purpose_rule: "name-word" } as const;
const PF1 = {
  installs: [setup("/Users/you/.claude", "vendor-default"), setup("/Users/you/.claude-work", "home-pattern", {}, WORK)],
  probes: [],
  seeds: [],
  desktop_apps: ["Claude.app"],
  not_set_up: ["codex", "opencode", "grok-build"],
  cancelled: false,
};

afterEach(cleanup);
beforeEach(() => {
  for (const name of Object.keys(window).filter((one) => one.startsWith("__AIJ_"))) Reflect.deleteProperty(window, name);
  Object.assign(window, { __AIJ_SCAN_REPORT__: PF1, __AIJ_OS_ACCESS__: [] });
});

const renderIt = () => render(<FirstRun mode="first-run" onDone={() => null} openLoginTerminal={() => null} />);

describe("AJ-E3306 PF1 — names", () => {
  it("a folder named for a word comes in by that word; one that says more keeps its name", () => {
    expect(labelOf(PF1.installs[1]!)).toBe("Work");
    expect(labelOf(setup("/Users/you/.claude-personal", "home-pattern", {}, { purpose: "Personal", purpose_rule: "name-word" }))).toBe("Personal");
    expect(labelOf(setup("/Users/you/.claude-client-acme", "home-pattern", {}, WORK))).toBe("claude-client-acme");
    expect(labelOf(setup("/Users/you/.claude-side", "home-pattern", {}, { purpose: "Personal", purpose_rule: "name-word" }))).toBe("claude-side");
    expect(labelOf(PF1.installs[0]!)).toBe("Default");
  });

  it("two of one tool that read the same are told apart, and never take an App's name", () => {
    const twins = [PF1.installs[1]!, setup("/Users/you/.claude_work", "home-pattern", {}, WORK)];
    expect([...setupNames(twins, []).values()]).toEqual(["Work", "Work 2"]);
    expect([...setupNames([PF1.installs[1]!], ["work"]).values()]).toEqual(["Work 2"]);
    const codex = setup("/Users/you/.codex-work", "home-pattern", { harness: "codex" }, WORK);
    expect([...setupNames([PF1.installs[1]!, codex], []).values()]).toEqual(["Work", "Work"]);
  });
});

describe("AJ-E3306 PF1 — no conversations is never an archive candidate", () => {
  it("says Not used yet, whatever tier an older backend sent", () => {
    const old = setup("/Users/you/.claude", "vendor-default", {}, { activity: "archive", activity_words: "Archive candidate · 0 d" });
    expect(usedWords(old, NOW)).toBe("Not used yet");
  });

  it("the found rows of a new setup say Not used yet, never Archive", async () => {
    const view = renderIt();
    const found = await view.findByTestId("first-run-found");
    expect(found.textContent).not.toContain("Archive");
    expect(view.getAllByTestId("first-run-row")[0]!.textContent).toContain("Not used yet");
  });
});

describe("AJ-E3306 PF1 — every tool found is offered", () => {
  it("each tool with nothing set up has its own group, unticked, saying so", async () => {
    const view = renderIt();
    await view.findByTestId("first-run-found");
    for (const [tool, name] of [["codex", "Codex"], ["opencode", "OpenCode"], ["grok-build", "Grok Build"]] as const) {
      const group = view.getByTestId(`first-run-group-fresh-${tool}`);
      const row = within(group).getByTestId("first-run-fresh-row");
      expect(row.textContent).toContain(`${name} is installed, with nothing set up yet`);
      expect(row.textContent).toContain("Sign in after");
      expect(within(row).getByRole("checkbox")).toHaveProperty("checked", false);
    }
    expect(view.getByTestId("first-run-bring-in").textContent).toBe("Bring in 2 apps");
  });

  it("a ticked one is set up fresh with Bring in, and its sign-in comes after", async () => {
    const accounts = await import("../lib/ipc/accounts");
    const spy = vi.spyOn(accounts, "createAccount");
    const view = renderIt();
    await view.findByTestId("first-run-found");
    fireEvent.click(within(view.getByTestId("first-run-group-fresh-codex")).getByRole("checkbox"));
    expect(view.getByTestId("first-run-bring-in").textContent).toBe("Bring in 3 apps");
    await act(async () => {
      fireEvent.click(view.getByTestId("first-run-bring-in"));
    });
    expect(spy).toHaveBeenCalledWith("codex", "Codex", null);
    expect(spy).toHaveBeenCalledTimes(1);
    const signin = await view.findByTestId("first-run-signin");
    expect(signin.textContent).toContain("Codex");
    spy.mockRestore();
  });

  it("with only tools to set up, the heading says what is installed", () => {
    expect(foundWords(null, 0, "Claude Code", "Claude Code", true, "claude-code", ["Codex", "OpenCode"]).heading).toBe(
      "Codex and OpenCode are installed, with nothing set up yet.",
    );
  });

  it("the desktop app with no profile seen open says where its profiles come from", async () => {
    expect(desktopNoneWords(["Claude app"])).toBe(
      "The Claude app is on this computer. Open a profile you use and it shows here to add, or add one any time in Settings › Desktop apps.",
    );
    const view = renderIt();
    await view.findByTestId("first-run-found");
    expect(view.getByTestId("first-run-desktop-none").textContent).toContain("The Claude app is on this computer.");
  });

  it("Add apps for one tool lists no other tool", async () => {
    const view = render(<FirstRun mode="add" provider="claude-code" onDone={() => null} openLoginTerminal={() => null} />);
    await view.findByTestId("first-run-found");
    expect(view.queryByTestId("first-run-group-fresh-codex")).toBeNull();
    expect(view.queryByTestId("first-run-desktop")).toBeNull();
  });
});
