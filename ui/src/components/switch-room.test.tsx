/**
 * The 0.2 walk, F1/F12/F17 — the switch and New session rank by room as
 * Home does, mark the same account Suggested, and never offer an account
 * out of room as if it had some: it's listed last, says until when and
 * what picking it does, and the switch's last button reads "Switch
 * anyway". "What's carried" and Confirm agree on what the other account
 * has (one source: the set-up check).
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../lib/bridge";
import * as setup from "../lib/ipc/switchSetup";
import * as subscriptions from "../lib/subscriptions";
import * as data from "../pages/session/data";
import type { AccountInfo, InstanceInfo, PreflightInfoData, QuotaDisplayData, SwitchTargetData } from "../lib/bridge";
import type { SetupCheckInfo } from "../lib/ipc/switchSetup";
import { newWorkAdvice } from "./AccountRunway";
import { rankByRoom } from "../lib/room";
import { rankTargets } from "../lib/room-targets";
import { ContinueOn } from "../pages/session/ContinueOn";
import { fromInstance } from "../pages/session/model";
import { SwitchFlow } from "./SwitchFlow";
import { CarryList, withSetup } from "./switch/CarryList";
import { LaunchPickWhy } from "./LaunchPickWhy";
import { pickLaunch } from "../lib/launch-pick";
import { resetToasts } from "./Toast";

const RESET = new Date(2026, 9, 29, 8, 0).getTime();

/** A profile's display with `left`% of its week left (`null`: not measured). */
const display = (account: string, left: number | null): QuotaDisplayData =>
  ({
    account,
    weekly: left === null ? null : { state: left === 0 ? "exhausted" : "ok", percentage: 100 - left, resets_at_ms: RESET, badge: "measured" },
    five_hour: null,
    forecast: { state: "learning", badge: "learning" },
    renewal: { state: "none", label: "", entered_by_user: false },
  }) as unknown as QuotaDisplayData;

const profile = (id: string, label: string): AccountInfo => ({ id, label, login_state: "logged_in", provider: "claude-code" }) as unknown as AccountInfo;

const target = (account_id: string, account_label: string, extra: Partial<SwitchTargetData> = {}): SwitchTargetData => ({
  recipe_id: `r-${account_id}`,
  provider: "claude-code",
  account_id,
  account_label,
  badge: "Same conversation, new account · Beta",
  validated: false,
  cross_provider: false,
  drops: [],
  score: null,
  filtered_reason: null,
  top_factors: [],
  factors: [],
  confidence: "Measured",
  tte_min: null,
  suggested: false,
  ...extra,
});

// The walk's case: 75, 98, 76, 66, 0 and 91 % left, in the order added.
const LEFT: Record<string, number> = { a: 75, b: 98, c: 76, d: 66, f: 0, g: 91 };
const DISPLAYS = Object.entries(LEFT).map(([id, left]) => display(id, left));
const TARGETS = Object.keys(LEFT).map((id) => target(id, `Account ${id.toUpperCase()}`));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  resetToasts();
});

describe("F1 one order by room", () => {
  it("ranks most room first, unmeasured before low, out of room last, and suggests only one with room", () => {
    const ranked = rankByRoom([["low", 10], ["out", 0], ["none", null], ["most", 90], ["some", 40]] as const, ([, left]) => left);
    expect(ranked.map((row) => row.item[0])).toEqual(["most", "some", "none", "low", "out"]);
    expect(ranked.filter((row) => row.suggested).map((row) => row.item[0])).toEqual(["most"]);
    // Nothing over 25% left: nothing suggested.
    expect(rankByRoom([10, 0, null], (left) => left).some((row) => row.suggested)).toBe(false);
  });

  it("the switch suggests the account Home calls Best for new work", () => {
    const profiles = Object.keys(LEFT).map((id) => profile(id, `Account ${id.toUpperCase()}`));
    const home = newWorkAdvice(profiles, DISPLAYS, []);
    expect(home.words).toBe("Best for new work: Account B, 98% left.");
    const ranked = rankTargets(TARGETS, null, DISPLAYS, "src");
    expect(ranked.map((row) => row.room.left)).toEqual([98, 91, 76, 75, 66, 0]);
    expect(ranked.find((row) => row.suggested)?.account_label).toBe("Account B");
    expect(ranked.at(-1)?.room.kind).toBe("out");
  });
});

describe("F1 Continue on another account", () => {
  beforeEach(() => {
    vi.spyOn(setup, "switchSetupCheck").mockResolvedValue({ from: "Work", to: "x", already_there: 0, offers: [] });
    vi.spyOn(bridge, "switchTargets").mockResolvedValue(TARGETS);
  });

  it("lists by room, marks the best Suggested and keeps the account out of room, last, saying why", async () => {
    const model = fromInstance({ id: "s-1", provider: "claude-code", account_id: "src", account_label: "Source", conversation_id: "c", status: "running", cwd: "/w" } as unknown as InstanceInfo);
    render(<ContinueOn model={model} accounts={[]} displays={DISPLAYS} onPick={vi.fn()} />);
    await waitFor(() => expect(screen.getAllByTestId("continue-target").length).toBe(6));
    const names = screen.getAllByTestId("continue-target").map((row) => row.textContent);
    expect(names[0]).toContain("Account B");
    expect(names[0]).toContain("Suggested");
    expect(screen.getAllByTestId("suggested-mark")).toHaveLength(1);
    const last = screen.getAllByTestId("continue-account").at(-1)!;
    expect(last.dataset.room).toBe("out");
    expect(within(last).getByTestId("continue-room").textContent).toMatch(/Out of room until .+You can still pick it, but the conversation stops at Account F's limit straight away/);
    // Not hidden, not greyed: the user may still pick it.
    expect((within(last).getByTestId("continue-target") as HTMLButtonElement).disabled).toBe(false);
  });
});

const ONECLICK = {
  mechanism: "transplant",
  fidelity: "F4",
  fidelity_words: "Continues exactly",
  score: 1,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: false,
  source_keeps_running: false,
  source_outside: false,
  experimental_copy: "Continues with the full conversation.",
  door: "one-click",
  wizard_steps: [],
  wizard_reasons: [],
  usual_account: false,
  carry: { carried: [], regenerated: [], dropped: [] },
  redaction: { global: "strip", override: null, effective: "strip", source: "global", line: "" },
  redaction_items: [],
  carry_items: [
    { label: "The whole conversation", hint: "Every message.", how: "moves", kind: "chat" },
    { label: "Connected tools (MCP servers)", hint: "The servers.", how: "moves", kind: "mcp" },
  ],
  move_line: "The whole conversation is carried over — all 31,009 messages.",
  secrets: { asks: false, reader: "Claude Code (Anthropic)" },
  target_tool: "claude-code",
  target_account: "f",
} as unknown as PreflightInfoData;

describe("F1 the switch dialog says the target's room", () => {
  beforeEach(() => {
    vi.spyOn(data, "useRunways").mockReturnValue(DISPLAYS);
    vi.spyOn(subscriptions, "useSubscriptions").mockReturnValue(null);
  });
  afterEach(() => {
    delete (window as never as { __AIJ_SWITCH_PREFLIGHT__?: unknown }).__AIJ_SWITCH_PREFLIGHT__;
  });
  const open = (preflight: PreflightInfoData) => {
    (window as never as { __AIJ_SWITCH_PREFLIGHT__?: unknown }).__AIJ_SWITCH_PREFLIGHT__ = preflight;
    return render(<SwitchFlow sessionId="s-1" targetProfileId="r-f" targetLabel="Account F" onClose={vi.fn()} onSwitched={vi.fn()} />);
  };

  it("an account out of room: Confirm says so, explained, never 'Nothing needs you', and the button is Switch anyway", async () => {
    open(ONECLICK);
    const room = await screen.findByTestId("switch-room");
    expect(room.dataset.room).toBe("out");
    expect(room.textContent).toMatch(/Account F: Out of room until/);
    expect(room.textContent).toContain("stops at Account F's limit straight away");
    expect(within(room).getByTestId("switch-room-explain").textContent).toContain("What does this mean?");
    expect(screen.getByTestId("switch-summary-room").textContent).toBe("Account F is out of room: see below before you switch.");
    expect(screen.getByTestId("switch-summary-needs").textContent).not.toContain("Nothing needs you");
    expect(screen.getByTestId("switch-confirm").textContent).toBe("Switch anyway");
  });

  it("an account with room: its room in words on the Target step, and Switch now", async () => {
    open({ ...ONECLICK, door: "wizard", wizard_steps: ["Target", "Confirm"], target_account: "b" } as PreflightInfoData);
    const room = await screen.findByTestId("switch-room");
    expect(room.textContent).toBe("Room on Account F: 98% left this week · Week resets " + room.textContent!.split("Week resets ")[1]);
    expect(screen.getByTestId("switch-step-target").textContent).toContain("Nothing happens until you choose Switch now");
    fireEvent.click(screen.getByTestId("switch-continue"));
    expect(screen.getByTestId("switch-confirm").textContent).toBe("Switch now");
  });
});

describe("F12 what's carried agrees with the set-up check", () => {
  const items = ONECLICK.carry_items;
  const offer = (used: boolean | null) => ({ id: `o-${String(used)}`, kind: "mcp-server", name: "x", scope: "user", verdict: "copy", note: null, suggested: true, about: null, used, origin: "own", provided_by: null, get_it: null, can_copy: true });
  const check = (offers: ReturnType<typeof offer>[]): SetupCheckInfo => ({ from: "A", to: "B", already_there: 2, offers }) as unknown as SetupCheckInfo;

  it("never says MCP servers go along while the check lists ones the other account lacks", () => {
    const lacking = withSetup(items, check([offer(true), offer(null)]), "Account B");
    expect(lacking.items.map((item) => item.kind)).toEqual(["chat"]);
    expect(lacking.missing[0]!.hint).toBe("2 this conversation used aren't set up on Account B yet: pick them on the last step to set them up first.");
    expect(withSetup(items, check([]), "Account B").items[1]!.hint).toBe("Account B has the same ones set up.");
    expect(withSetup(items, check([offer(false)]), "Account B").items[1]!.hint).toContain("none of them used in this conversation");
    render(<CarryList items={items} target="Account B" tool="Claude Code" setup={check([offer(true)])} />);
    expect(screen.getByTestId("switch-carry-missing").textContent).toContain("Not on Account B yet");
    expect(screen.getByTestId("switch-carry-moves").textContent).not.toContain("MCP");
  });
});

describe("F17 New session's pick says why, and offers the Suggested one", () => {
  const recipe = (account: string, label: string) => ({ id: `r-${account}`, name: "Default", account_id: account, account_label: label, provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null }) as unknown as bridge.LaunchRecipeInfo;

  it("your default with room stays the pick, the Suggested one is one click away", () => {
    const recipes = [recipe("g", "Account G"), recipe("b", "Account B")];
    const accounts = [{ ...profile("g", "Account G"), is_default: true }, profile("b", "Account B")] as AccountInfo[];
    const quota = { g: display("g", 75), b: display("b", 98) };
    const pick = pickLaunch(recipes, null, Object.values(quota), accounts)!;
    expect(pick.recipe.account_id).toBe("g");
    const onUse = vi.fn();
    render(<LaunchPickWhy pick={pick} quota={quota} onUse={onUse} />);
    expect(screen.getByTestId("launch-default").textContent).toBe("your default");
    expect(screen.getByTestId("launch-suggested").textContent).toContain("Suggested: Account B has the most room (98% left).");
    fireEvent.click(screen.getByTestId("launch-use-suggested"));
    expect(onUse).toHaveBeenCalledWith("b");
  });

  it("your default out of room gives way, and the sheet says until when", () => {
    const recipes = [recipe("g", "Account G"), recipe("b", "Account B")];
    const accounts = [{ ...profile("g", "Account G"), is_default: true }, profile("b", "Account B")] as AccountInfo[];
    const quota = { g: display("g", 0), b: display("b", 98) };
    const pick = pickLaunch(recipes, null, Object.values(quota), accounts)!;
    expect(pick.recipe.account_id).toBe("b");
    render(<LaunchPickWhy pick={pick} quota={quota} onUse={vi.fn()} />);
    expect(screen.getByTestId("launch-default").textContent).toBe("Suggested: the most room");
    expect(screen.getByTestId("launch-passed-over").textContent).toMatch(/Account G \(your default\): Out of room until .+ A session there would stop at its limit at once/);
  });
});
