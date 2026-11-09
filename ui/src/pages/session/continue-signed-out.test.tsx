/**
 * 0.4.1 (the rc5 walk): the switch's picker showed a folder at "76% left"
 * while New session said the same folder isn't signed in. A target whose
 * tool isn't signed in there never shows its account's percentage: it
 * says "Not signed in", as New session does, and is never suggested.
 */
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "../../lib/ipc/switch-targets";
import * as setup from "../../lib/ipc/switchSetup";
import { instance } from "../../test/fixtures";
import type { QuotaDisplayData } from "../../lib/ipc/quota";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { DESKTOP_ONLY, NOT_SIGNED_IN, rankTargets } from "../../lib/room-targets";
import { SwitchTargetRows } from "../../components/SwitchTargetRows";
import { ContinueOn } from "./ContinueOn";
import { fromInstance } from "./model";
import { previewNamed } from "../../preview/names";
import { setToolsForTests } from "../../lib/tools";
import { PREVIEW_TOOLS } from "../../preview/tools";

setToolsForTests(PREVIEW_TOOLS);

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

beforeEach(() => {
  vi.spyOn(setup, "switchSetupCheck").mockResolvedValue({ from: "Lab", to: "x", already_there: 0, offers: [] });
});

const account = (key: string, ids: string[]): SubscriptionInfo =>
  previewNamed([
    {
      key,
      tool: "claude-code",
      state: "signed_in",
      email: null,
      organisation: null,
      reason: null,
      profiles: ids.map((id) => ({ id, label: `Profile ${id}`, folder: `~/p/${id}`, plan: null, renewal_date: null, fee_usd: null })),
      plan: "Max",
      renewal_date: null,
      fee_usd: null,
      differs: [],
      choices: [],
    },
  ])[0]!;

const display = (id: string, left: number): QuotaDisplayData => ({
  account: id,
  weekly: {
    state: "tracking",
    percentage: 100 - left,
    badge: "measured",
    stale_note_ms: null,
    resets_at_ms: null,
    reset_basis: null,
    measured_at_ms: null,
    burned_tokens: 0,
    burned_credits: null,
    headroom_tokens: null,
  },
  weekly_omitted_reason: null,
  five_hour: null,
  five_hour_omitted_reason: null,
  forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "learning" },
  renewal: { state: "none", label: "", entered_by_user: false },
  unavailable_reason: null,
  credits: null,
});

const target = (account_id: string, extra: Partial<bridge.SwitchTargetData> = {}): bridge.SwitchTargetData => ({
  recipe_id: `recipe-${account_id}`,
  provider: "claude-code",
  account_id,
  account_label: `App ${account_id}`,
  badge: "Same conversation, another app · Beta",
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

// The source on Lab; the test profile on Test (76% left), signed out in a terminal.
const ACCOUNTS = [account("lab", ["lab"]), account("test", ["test", "test-2"]), account("other", ["other"])];
const DISPLAYS = [display("test", 76), display("test-2", 76), display("other", 40)];

function show(rows: bridge.SwitchTargetData[]) {
  vi.spyOn(bridge, "switchTargets").mockResolvedValue(rows);
  const session = instance({ id: "sess-1", provider: "claude-code", account_id: "lab", account_label: "Lab", conversation_id: "c-1", cwd: "/w" });
  render(<ContinueOn model={fromInstance(session)} accounts={ACCOUNTS} displays={DISPLAYS} onPick={vi.fn()} />);
}

describe("0.4.1 a target that isn't signed in", () => {
  it("ranks with no percentage, says Not signed in, and is never suggested", () => {
    const ranked = rankTargets([target("test", { signed_out: true }), target("other")], ACCOUNTS, DISPLAYS, "lab");
    const test = ranked.find((row) => row.account_id === "test")!;
    expect(test.room).toMatchObject({ kind: "blocked", left: null, words: NOT_SIGNED_IN, credits: null });
    expect(test.suggested).toBe(false);
    expect(ranked.map((row) => row.account_id)).toEqual(["other", "test"]);
    expect(ranked[0]!.suggested).toBe(true);
  });

  it("the switch dialog's row says Not signed in, not 76% left", () => {
    const ranked = rankTargets([target("test", { signed_out: true })], ACCOUNTS, DISPLAYS, "lab");
    render(<SwitchTargetRows targets={ranked} onPick={vi.fn()} />);
    const rows = screen.getByTestId("switch-target-rows");
    expect(within(rows).getByTestId("target-room").textContent).toBe(NOT_SIGNED_IN);
    expect(rows.textContent).not.toMatch(/% left/);
  });

  it("rc6: no forecast of its account's room on its row either", () => {
    const ranked = rankTargets([target("test", { signed_out: true, tte_min: 90, confidence: "measured" })], ACCOUNTS, DISPLAYS, "lab");
    render(<SwitchTargetRows targets={ranked} onPick={vi.fn()} />);
    expect(screen.getByTestId("switch-target-rows").textContent).not.toMatch(/exhausts|forecast|measured/);
  });

  it("Continue in another app says it beside the account, with no percentage", async () => {
    show([target("test", { signed_out: true })]);
    await waitFor(() => expect(screen.getByTestId("continue-target")).toBeTruthy());
    const group = screen.getByTestId("continue-account");
    expect(group.dataset.room).toBe("blocked");
    expect(within(group).getByTestId("continue-signed-out").textContent).toBe(NOT_SIGNED_IN);
    expect(group.textContent).not.toMatch(/% left/);
    expect(screen.getByTestId("continue-target").dataset.suggested).toBeUndefined();
  });

  it("an account with another app signed in keeps its room; the signed-out app says so on its own row", async () => {
    show([target("test-2"), target("test", { signed_out: true })]);
    await waitFor(() => expect(screen.getAllByTestId("continue-target").length).toBe(2));
    const group = screen.getByTestId("continue-account");
    expect(group.textContent).toContain("76% left");
    const checks = within(group).getAllByTestId("continue-check");
    expect(checks.map((check) => check.textContent)).toContain(NOT_SIGNED_IN);
  });

  it("rc12: a signed-in Desktop with a signed-out terminal shows the account's room and says which is which", async () => {
    const one = target("test", { signed_out: true, desktop: true });
    const ranked = rankTargets([one], ACCOUNTS, DISPLAYS, "lab");
    expect(ranked[0]!.room).toMatchObject({ kind: "room", left: 76 });
    expect(ranked[0]!.suggested).toBe(true);
    render(<SwitchTargetRows targets={ranked} onPick={vi.fn()} />);
    expect(screen.getByTestId("target-room").textContent).toContain("76% left");
    expect(screen.getByTestId("target-sign-in").textContent).toBe(DESKTOP_ONLY);
    expect(screen.getByTestId("switch-target-rows").textContent).not.toContain(NOT_SIGNED_IN);
    cleanup();
    show([one]);
    await waitFor(() => expect(screen.getByTestId("continue-target")).toBeTruthy());
    const group = screen.getByTestId("continue-account");
    expect(group.textContent).toContain("76% left");
    expect(within(group).getByTestId("continue-check").textContent).toBe(DESKTOP_ONLY);
  });
});
