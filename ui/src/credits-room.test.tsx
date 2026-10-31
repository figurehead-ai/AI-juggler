/**
 * The maintainer's report (0.3.2): the banner said "has run out of room ·
 * 0% of its weekly window left" while the session kept working on the
 * account's pre-bought credits. A window used up while the credits cover
 * is its own room state, "credits": ranked after low and before out of
 * room, never Suggested, and every surface says the credits truth ("Using
 * credits · 14,150 left"; "A session there goes on with its credits until
 * the window resets …"). Credits that don't cover (a balance of 0, or
 * none) keep today's out-of-room words exactly.
 */
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as bridge from "./lib/bridge";
import * as setup from "./lib/ipc/switchSetup";
import * as subscriptions from "./lib/subscriptions";
import * as data from "./pages/session/data";
import type { AccountInfo, InstanceInfo, PreflightInfoData, QuotaDisplayData, SwitchTargetData } from "./lib/bridge";
import type { CreditsDisplay } from "./lib/ipc/quota";
import { creditsConsequence, creditsCover, usingCredits, usingCreditsWords, wholeNumber } from "./lib/credits";
import { rankByRoom, roomKind } from "./lib/room";
import { rankTargets } from "./lib/room-targets";
import { pickLaunch } from "./lib/launch-pick";
import { newWorkAdvice, runwayLines, runwayOf } from "./components/AccountRunway";
import { SwitchTargetRows } from "./components/SwitchTargetRows";
import { SwitchFlow } from "./components/SwitchFlow";
import { LaunchPickWhy } from "./components/LaunchPickWhy";
import { RunwayPanel } from "./components/usage/Runway";
import { NotificationCentre } from "./components/NotificationCentre";
import { NotificationRegion } from "./components/NotificationRegion";
import { resetNoticeStandIn, notificationsSummary, type NoticeInfo, type NoticeSeed } from "./lib/ipc/notices";
import { resetNotices } from "./lib/notices-store";
import { noticeTone, quotaDetail } from "./lib/notice-words";
import { resetToasts } from "./components/Toast";

const NOW = Date.now();
const RESET = NOW + 3 * 86_400_000;

const credits = (balance: string | null, extra: Partial<CreditsDisplay> = {}): CreditsDisplay => ({
  has_credits: true,
  unlimited: false,
  balance,
  as_of_ms: NOW - 12 * 60_000,
  source: "Codex",
  ...extra,
});

/** A display with `left`% of its week left, and its credits. */
const display = (account: string, left: number | null, withCredits: CreditsDisplay | null = null): QuotaDisplayData =>
  ({
    account,
    weekly: left === null ? null : { state: left === 0 ? "exhausted" : "tracking", percentage: 100 - left, resets_at_ms: RESET, badge: "measured", burned_tokens: 0 },
    five_hour: null,
    forecast: { state: "learning", badge: "learning" },
    renewal: { state: "none", label: "", entered_by_user: false },
    unavailable_reason: null,
    credits: withCredits,
  }) as unknown as QuotaDisplayData;

const REAL = credits("14150.1635025000");
const ON_CREDITS = display("codex", 0, REAL);
const ZERO = display("codex", 0, credits("0"));

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  resetToasts();
});

describe("the one rule: credits cover", () => {
  it("unlimited, or a balance above zero; with no balance, probably; zero or none, never", () => {
    expect(creditsCover(REAL)).toEqual({ kind: "balance", balance: 14150.1635025 });
    expect(creditsCover(credits(null, { unlimited: true, has_credits: false }))).toEqual({ kind: "unlimited" });
    expect(creditsCover(credits(null))).toEqual({ kind: "probably" });
    expect(creditsCover(credits("0"))).toBeNull();
    expect(creditsCover(credits("0.0000"))).toBeNull();
    expect(creditsCover(credits("12", { has_credits: false }))).toBeNull();
    expect(creditsCover(null)).toBeNull();
  });

  it("says it plain and short, the balance whole with separators", () => {
    expect(wholeNumber(14150.1635025)).toBe("14,150");
    expect(usingCreditsWords({ kind: "balance", balance: 14150.1635025 })).toBe("Using credits · 14,150 left");
    expect(usingCreditsWords({ kind: "probably" })).toBe("Probably using credits");
    expect(usingCreditsWords({ kind: "unlimited" })).toBe("Using credits · unlimited");
    expect(creditsConsequence(RESET, NOW)).toMatch(/^A session there goes on with its credits until the window resets .+\.$/);
    expect(creditsConsequence(null, NOW)).toBe("A session there goes on with its credits until the window resets.");
  });

  it("only while a window is used up", () => {
    expect(usingCredits(ON_CREDITS)).toEqual({ kind: "balance", balance: 14150.1635025 });
    expect(usingCredits(display("codex", 40, REAL))).toBeNull();
    expect(usingCredits(ZERO)).toBeNull();
  });
});

describe("the room state, credits", () => {
  it("ranks room, unknown, low, credits, out, blocked; never Suggested", () => {
    expect(roomKind(0, true, true)).toBe("credits");
    expect(roomKind(0, false, true)).toBe("blocked");
    const rows = [["out", 0, false], ["credits", 0, true], ["blocked", 50, false], ["low", 10, false], ["none", null, false], ["room", 60, false]] as const;
    const ranked = rankByRoom(rows, ([, left]) => left, ([name]) => name !== "blocked", ([, , onCredits]) => onCredits);
    expect(ranked.map((row) => row.item[0])).toEqual(["room", "none", "low", "credits", "out", "blocked"]);
    const only = rankByRoom([["credits", 0, true]] as const, ([, left]) => left, () => true, ([, , onCredits]) => onCredits);
    expect(only[0]!.kind).toBe("credits");
    expect(only[0]!.suggested).toBe(false);
  });

  it("the runway says it, never only 0% left; and its window line is amber, not red", () => {
    const run = runwayOf(ON_CREDITS);
    expect(run.left).toBe(0);
    expect(run.words).toBe("Using credits · 14,150 left");
    expect(run.credits).not.toBeNull();
    const week = (lines: ReturnType<typeof runwayLines>) => lines.find((line) => line.left !== null)!;
    expect(week(runwayLines(ON_CREDITS)).tone).toBe("low");
    // A balance of 0: as today.
    expect(runwayOf(ZERO).words).toBe("0% left this week");
    expect(week(runwayLines(ZERO)).tone).toBe("out");
  });

  it("Home's advice never calls it out of room", () => {
    const codex = { id: "codex", label: "Codex", login_state: "logged_in", provider: "codex" } as unknown as AccountInfo;
    expect(newWorkAdvice([codex], [ON_CREDITS], []).words).toBe("No account has room in its window: Codex is using its credits.");
  });
});

const target = (account_id: string, account_label: string): SwitchTargetData => ({
  recipe_id: `r-${account_id}`,
  provider: "codex",
  account_id,
  account_label,
  badge: "Same conversation, new account · Beta",
  validated: false,
  cross_provider: false,
  drops: [],
  score: 0.3,
  filtered_reason: null,
  top_factors: ["using its credits", "quiet"],
  factors: [],
  confidence: "Measured",
  tte_min: null,
  suggested: false,
});

describe("the switch list and its confirm", () => {
  it("lists it after low ones, before any out of room, saying it goes on with its credits", () => {
    const displays = [ON_CREDITS, display("out", 0, credits("0")), display("low", 10), display("room", 70)];
    const ranked = rankTargets([target("codex", "Codex"), target("out", "Out"), target("low", "Low"), target("room", "Room")], null, displays, "src");
    expect(ranked.map((row) => row.account_label)).toEqual(["Room", "Low", "Codex", "Out"]);
    render(<SwitchTargetRows targets={ranked} onPick={vi.fn()} />);
    const rooms = screen.getAllByTestId("target-room");
    const codex = rooms.find((row) => row.dataset.room === "credits")!;
    expect(codex.textContent).toMatch(/^Using credits · 14,150 leftA session there goes on with its credits until the window resets .+\.$/);
    expect(codex.textContent).not.toMatch(/out of room/i);
    // Zero credits: today's words exactly.
    expect(rooms.find((row) => row.dataset.room === "out")!.textContent).toMatch(/^Out of room until /);
  });

  it("Confirm says the room in credits, and the button is Switch now", async () => {
    vi.spyOn(data, "useRunways").mockReturnValue([ON_CREDITS]);
    vi.spyOn(subscriptions, "useSubscriptions").mockReturnValue(null);
    vi.spyOn(setup, "switchSetupCheck").mockResolvedValue({ from: "Work", to: "Codex", already_there: 0, offers: [] } as never);
    (window as never as { __AIJ_SWITCH_PREFLIGHT__?: unknown }).__AIJ_SWITCH_PREFLIGHT__ = {
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
      experimental_copy: "",
      door: "one-click",
      wizard_steps: [],
      wizard_reasons: [],
      usual_account: false,
      carry: { carried: [], regenerated: [], dropped: [] },
      redaction: { global: "strip", override: null, effective: "strip", source: "global", line: "" },
      redaction_items: [],
      carry_items: [],
      move_line: "The whole conversation is carried over.",
      secrets: { asks: false, reader: "Codex" },
      target_tool: "codex",
      target_account: "codex",
    } as unknown as PreflightInfoData;
    try {
      render(<SwitchFlow sessionId="s-1" targetProfileId="r-codex" targetLabel="Codex" onClose={vi.fn()} onSwitched={vi.fn()} />);
      const summary = await screen.findByTestId("switch-summary-room");
      expect(summary.textContent).toMatch(/^Room on Codex: Using credits · 14,150 left\. It goes on with its credits until the window resets .+\.$/);
      expect(screen.getByTestId("switch-confirm").textContent).toBe("Switch now");
      expect(screen.queryByText(/out of room/i)).toBeNull();
    } finally {
      delete (window as never as { __AIJ_SWITCH_PREFLIGHT__?: unknown }).__AIJ_SWITCH_PREFLIGHT__;
    }
  });
});

describe("New session's pick", () => {
  const recipe = (account: string, label: string) => ({ id: `r-${account}`, name: "Default", account_id: account, account_label: label, provider: "codex", model: null, default_cwd: null, restart_on_crash: null }) as unknown as bridge.LaunchRecipeInfo;
  const accounts = [
    { id: "codex", label: "Codex", login_state: "logged_in", provider: "codex", is_default: true },
    { id: "b", label: "Account B", login_state: "logged_in", provider: "codex" },
  ] as unknown as AccountInfo[];

  it("your default on credits stays the pick (a session there goes on), and says so", () => {
    const recipes = [recipe("codex", "Codex"), recipe("b", "Account B")];
    const quota = { codex: ON_CREDITS, b: display("b", 80) };
    const pick = pickLaunch(recipes, null, Object.values(quota), accounts)!;
    expect(pick.recipe.account_id).toBe("codex");
    expect(pick.passedOver).toBeNull();
    render(<LaunchPickWhy pick={pick} quota={quota} onUse={vi.fn()} />);
    expect(screen.getByTestId("launch-credits").textContent).toMatch(/^A session there goes on with its credits until the window resets .+\.$/);
    expect(screen.queryByTestId("launch-passed-over")).toBeNull();
  });

  it("with a balance of 0 it is passed over, as today", () => {
    const recipes = [recipe("codex", "Codex"), recipe("b", "Account B")];
    const quota = { codex: ZERO, b: display("b", 80) };
    const pick = pickLaunch(recipes, null, Object.values(quota), accounts)!;
    expect(pick.recipe.account_id).toBe("b");
    render(<LaunchPickWhy pick={pick} quota={quota} onUse={vi.fn()} />);
    expect(screen.getByTestId("launch-passed-over").textContent).toMatch(/Codex \(your default\): Out of room until /);
    expect(screen.queryByTestId("launch-credits")).toBeNull();
  });
});

describe("the account card", () => {
  it("says it's using its credits and what that means for its sessions", () => {
    render(<RunwayPanel display={ON_CREDITS} limitsProfile={null} usage={null} usageTotal={null} color="#888" name="Codex" now={NOW} compact />);
    const line = screen.getByTestId("runway-credits");
    expect(line.textContent).toMatch(/^Using credits · 14,150 leftSessions on it go on with its credits until the window resets .+\.$/);
    const gauges = screen.getAllByTestId("window-gauge").map((gauge) => gauge.textContent).join(" | ");
    expect(gauges).toContain("Used up, on credits");
    expect(gauges).not.toContain("Limit hit");
  });

  it("with a balance of 0, the limit hit as today and no credits line", () => {
    render(<RunwayPanel display={ZERO} limitsProfile={null} usage={null} usageTotal={null} color="#888" name="Codex" now={NOW} compact />);
    expect(screen.queryByTestId("runway-credits")).toBeNull();
    expect(screen.getAllByTestId("window-gauge").map((gauge) => gauge.textContent).join(" | ")).toContain("Limit hit");
  });
});

describe("the notice", () => {
  type Seams = { __AIJ_NOTICES__?: NoticeSeed };
  const w = () => window as unknown as Seams;
  const facts = { window: "weekly", resets_at_ms: RESET, sessions: 1, version: null, channel: null, attempt: null, tries: null, account: "Codex" };
  const tgt = { session_id: null, conversation_id: null, account_id: "codex", project_id: null, page: null, task_id: null, pool_id: null };
  const onCredits: NoticeInfo = { id: 41, kind: "quota", needs: false, title: "Codex is using its credits", detail: "Its weekly window is used up · 14,150 credits left", project_id: null, project: null, target: tgt, facts: { ...facts, credits: "14,150 credits left" }, action: "hand-over", meter: null, created_ms: NOW, unread: true, subject: "n:41", snoozed: false, snoozed_until_ms: null, muted: false, dismissed: false };

  beforeEach(() => {
    delete w().__AIJ_NOTICES__;
    resetNoticeStandIn();
    resetNotices();
  });

  it("words: used up until the reset, what's left; probably when the balance isn't said; it informs", () => {
    expect(quotaDetail(onCredits, NOW)).toMatch(/^Its weekly window is used up until .+ · 14,150 credits left · 1 session uses it$/);
    expect(quotaDetail({ ...onCredits, facts: { ...onCredits.facts, credits: "" } }, NOW)).toMatch(/^Its weekly window is used up until .+ · 1 session uses it$/);
    expect(noticeTone(onCredits)).toBe("info");
    expect(noticeTone({ kind: "quota", facts: { ...facts, credits: null } })).toBe("warn");
  });

  it("the centre lists it under Earlier with Continue on another account…, never as Needs you", async () => {
    w().__AIJ_NOTICES__ = { earlier: [onCredits] };
    render(<NotificationCentre summary={await notificationsSummary()} sessions={[]} onClose={() => {}} />);
    const row = (await screen.findAllByTestId("notice-earlier"))[0]!;
    expect(row.textContent).toContain("Codex is using its credits");
    expect(row.textContent).toMatch(/Its weekly window is used up until .+ · 14,150 credits left/);
    expect(within(row).getByRole("button", { name: /^(Continue on another account…|Open its account)/ })).toBeTruthy();
    expect(row.textContent).not.toMatch(/out of room/i);
    expect(screen.queryAllByTestId("notice-need")).toHaveLength(0);
  });

  it("the banner band shows no out-of-room banner for it", async () => {
    w().__AIJ_NOTICES__ = { earlier: [onCredits] };
    vi.spyOn(bridge, "alertsTick").mockResolvedValue([]);
    vi.spyOn(bridge, "resetQueueList").mockResolvedValue([]);
    vi.spyOn(bridge, "parkedOffers").mockResolvedValue([]);
    vi.spyOn(bridge, "fleetRemote").mockResolvedValue([]);
    render(<NotificationRegion sessions={[] as InstanceInfo[]} />);
    await waitFor(() => expect(screen.getByTestId("notification-region")).toBeTruthy());
    expect(screen.queryByTestId("quota-banner")).toBeNull();
    expect(screen.getByTestId("notification-region").textContent).not.toMatch(/run out of room/);
  });
});
