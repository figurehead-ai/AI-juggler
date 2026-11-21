/**
 * AJ-E2312 item 5 (LC1: New session didn't remember "Run without a
 * terminal", so the person picked it again at every start): it's
 * remembered for the App it was started on, and New session opens on it
 * there again; a start there with another choice forgets it; another App
 * doesn't take it. It's offered only where the App can run without a
 * terminal now (D-127), and a remembered one never applies where it can't.
 */
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NewSessionModal } from "../screens/NewSession";
import * as launchModule from "../lib/ipc/launch";
import * as runtimeModule from "../lib/ipc/runtime";
import { clearSeams, project, recipe, setSeam } from "../test/fixtures";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { ProfileRuntimeInfo } from "../types/generated/ProfileRuntimeInfo";

const folder = (id: string, label: string): AccountInfo => ({ id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false });
const ACCOUNTS = [folder("claude-lab", "Lab"), folder("claude-work", "Work")];
const RECIPES = [recipe({ id: "p-lab", account_id: "claude-lab", account_label: "Lab" }), recipe({ id: "p-work", account_id: "claude-work", account_label: "Work" })];
const WEB = project({ id: "proj-web", root: "/Users/you/lab/web", name: "web" });
const can = (profile: string, unavailable: string | null = null): ProfileRuntimeInfo => ({ profile_id: profile, account_label: "Lab", name: "Main", label: "Claude Code's stream", default: "terminal", chosen: null, beta: true, unavailable });

let launchSpy: ReturnType<typeof vi.spyOn>;
const offer = (profiles: ProfileRuntimeInfo[], beta = true) => vi.spyOn(runtimeModule, "runtimeChoices").mockResolvedValue({ beta_on: beta, profiles });

beforeEach(() => {
  localStorage.clear();
  setSeam("__AIJ_RECIPES__", RECIPES);
  setSeam("__AIJ_RANKING__", []);
  launchSpy = vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  clearSeams("__AIJ_RECIPES__", "__AIJ_RANKING__");
});

const open = async (app: string) => {
  const view = render(<NewSessionModal projects={[WEB]} accounts={ACCOUNTS} recipes={RECIPES} initialDirectory={WEB.root} initialAccount={app} onClose={() => {}} onLaunched={() => {}} />);
  // The runtimes' read has answered.
  await act(async () => {});
  return view;
};
const labelOf = () => screen.getByTestId("new-session-more-values").textContent;
const howItRuns = async (choice: string) => {
  fireEvent.click(screen.getByTestId("new-session-more-toggle"));
  fireEvent.click(within(screen.getByTestId("runtime-pick")).getByRole("button"));
  fireEvent.click(await screen.findByText(choice));
};
const offered = async () => {
  fireEvent.click(screen.getByTestId("new-session-more-toggle"));
  fireEvent.click(within(screen.getByTestId("runtime-pick")).getByRole("button"));
  // The list is open once its first choice shows.
  await screen.findByText("Start in terminal");
  return screen.queryByText("Run without a terminal") !== null;
};
const start = async (runtime: string | undefined) => {
  fireEvent.keyDown(window, { key: "Enter", metaKey: true });
  await waitFor(() => expect(launchSpy).toHaveBeenLastCalledWith(expect.any(String), "/Users/you/lab/web", runtime));
};

describe("Run without a terminal, remembered for its App (AJ-E2312 item 5)", () => {
  it("chosen and started on an App, New session opens on it there next time, and starts that way", async () => {
    offer([can("p-lab"), can("p-work")]);
    const first = await open("claude-lab");
    await howItRuns("Run without a terminal");
    await start("driven");
    first.unmount();
    await open("claude-lab");
    expect(labelOf()).toContain("runs without a terminal");
    await start("driven");
  });

  it("another App doesn't take it", async () => {
    offer([can("p-lab"), can("p-work")]);
    const first = await open("claude-lab");
    await howItRuns("Run without a terminal");
    await start("driven");
    first.unmount();
    await open("claude-work");
    expect(labelOf()).not.toContain("without a terminal");
    await start(undefined);
  });

  it("a start there with another choice forgets it", async () => {
    offer([can("p-lab")]);
    const first = await open("claude-lab");
    await howItRuns("Run without a terminal");
    await start("driven");
    first.unmount();
    const second = await open("claude-lab");
    await howItRuns("As its launch settings say");
    await start(undefined);
    second.unmount();
    await open("claude-lab");
    expect(labelOf()).not.toContain("without a terminal");
  });
});

describe("offered only where the App can use it (D-127)", () => {
  it("an App whose tool can't run without a terminal isn't offered it", async () => {
    offer([can("p-lab")]);
    await open("claude-work");
    expect(await offered()).toBe(false);
  });

  it("nor is one whose tool can't now, and a remembered choice doesn't apply there", async () => {
    offer([can("p-lab")]);
    const first = await open("claude-lab");
    expect(await offered()).toBe(true);
    fireEvent.click(screen.getByText("Run without a terminal"));
    await start("driven");
    first.unmount();
    vi.restoreAllMocks();
    launchSpy = vi.spyOn(launchModule, "launchSession").mockResolvedValue({ session_id: "s1", pid: 1 });
    offer([can("p-lab", "Claude Code 2.1 or later is needed")]);
    await open("claude-lab");
    expect(labelOf()).not.toContain("without a terminal");
    await start(undefined);
  });

  it("with the Beta switch off, it isn't offered", async () => {
    offer([can("p-lab")], false);
    await open("claude-lab");
    expect(await offered()).toBe(false);
  });
});
