/**
 * D-103.9 (persona R3, P7: "0 tokens" and "No usage yet" beside 14
 * sessions): with no tokens measured, a project's "Who did the work"
 * falls back to sessions per Account ("Client B · 14 sessions"), the
 * week's tokens tile says none were recorded and how many ran, and the
 * counts are the backend's when it has answered, sessions on no known
 * Account said as such.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import type { UsageHistoryInfo } from "../../lib/ipc/usageHistory";
import { sessionsByAccount, sessionsWords } from "./sessions-by-account";
import { WeekTokensTile, WhoDidTheWork } from "./WhoDidTheWork";

const NOW = 1_800_000_000_000;
const DAY = 86_400_000;

const subscription = (key: string, name: string, folders: string[], state = "signed_in"): SubscriptionInfo => ({
  key,
  account_id: `acct:${key}`,
  display_name: name,
  tool: "claude-code",
  state,
  email: null,
  organisation: null,
  reason: null,
  profiles: folders.map((id) => ({ id, label: id, folder: `~/.config/${id}`, plan: null, renewal_date: null, fee_usd: null })),
  plan: null,
  renewal_date: null,
  fee_usd: null,
  differs: [],
  choices: [],
});
const SUBS = [subscription("b", "Client B", ["cc-client-b"]), subscription("agency", "Agency", ["cc-agency"]), subscription("old", "Not signed in", ["cc-old"], "signed_out")];

const zero = { input: 0, output: 0, cache_read: 0, cache_write: 0, reasoning: 0 };
const totals = (input: number) => ({ tokens: { ...zero, input }, usd: null, unpriced: false, sessions: 0, active_hours: 0, messages: input > 0 ? 3 : 0 });
const history = (input: number): UsageHistoryInfo => ({
  edges_ms: [NOW - 7 * DAY, NOW],
  group_by: "profile",
  series: [],
  totals: totals(input),
  previous: totals(0),
  by_profile: input > 0 ? [{ key: "cc-client-b", label: "client-b", io: input, cache: 0, usd: null, sessions: 1, profiles: [] }] : [],
  by_tool: [],
  by_model: [],
  by_project: [],
  top_sessions: [],
  session_count: 0,
  sessions_per_bucket: [],
  started_per_bucket: [],
  active_hours: [],
  switches: [],
  unmeasured_sessions: 0,
  truncated: false,
  reading: null,
});
// The R3 lab: 14 sessions on Client B over the last week, no token use.
const ROWS = Array.from({ length: 14 }, (_, i) => ({ last: NOW - (i + 1) * 10 * 3_600_000, accountId: "cc-client-b" }));
const PROJECT = { id: "proj-client-b", name: "client-b" };

afterEach(cleanup);

describe("sessions per Account", () => {
  it("the backend's count by Account: Client B · 14 sessions", () => {
    const parts = sessionsByAccount({ counted: { byAccount: new Map([["acct:b", 14]]), total: 14 }, local: [], subscriptions: SUBS });
    expect(parts.total).toBe(14);
    expect(sessionsWords(parts.items)).toBe("Client B · 14 sessions");
  });

  it("sessions on no known Account are one more part, said as such, last", () => {
    const parts = sessionsByAccount({ counted: { byAccount: new Map([["acct:agency", 2], ["acct:b", 10]]), total: 14 }, local: [], subscriptions: SUBS });
    expect(sessionsWords(parts.items)).toBe("Client B · 10 sessions, Agency · 2 sessions, Account unknown · 2 sessions");
  });

  it("until the backend answers, the sessions the app holds, by their App's Account; one not signed in is unknown", () => {
    const local = [{ accountId: "cc-client-b" }, { accountId: "cc-client-b" }, { accountId: "cc-agency" }, { accountId: "cc-old" }, { accountId: null }];
    const parts = sessionsByAccount({ counted: null, local, subscriptions: SUBS });
    expect(parts.total).toBe(5);
    expect(sessionsWords(parts.items)).toBe("Client B · 2 sessions, Agency · 1 session, Account unknown · 2 sessions");
  });
});

describe("a project's Overview with no tokens measured", () => {
  it("Who did the work falls back to sessions per Account, never No usage yet", () => {
    render(<WhoDidTheWork project={PROJECT} data={history(0)} rows={ROWS} />);
    const who = screen.getByTestId("project-who");
    expect(who.textContent).toContain("sessions by account");
    expect(screen.getByTestId("project-who-sessions").textContent).toContain("14sessions");
    expect(who.textContent).not.toContain("No usage yet");
    expect(who.textContent).not.toContain("0 tokens");
  });

  it("with tokens, it's tokens by App over 30 days, as before", () => {
    render(<WhoDidTheWork project={PROJECT} data={history(1200)} rows={ROWS} />);
    expect(screen.queryByTestId("project-who-sessions")).toBeNull();
    expect(screen.getByTestId("project-who").textContent).toContain("last 30 days");
  });

  it("with nothing ran either, No usage yet", () => {
    render(<WhoDidTheWork project={PROJECT} data={history(0)} rows={[]} />);
    expect(screen.getByTestId("project-who").textContent).toContain("No usage yet");
  });

  it("the week's tokens tile says none were recorded beside the sessions that ran, never a bare 0", () => {
    render(<WeekTokensTile week={history(0)} trend={[]} rows={ROWS} now={NOW} />);
    const tile = screen.getByTestId("tile-week-tokens");
    expect(screen.getByTestId("stat-value").textContent).toBe("None recorded");
    expect(tile.textContent).toContain("14 sessions this week; their files hold no token use");
  });

  it("the tile is a number when tokens were recorded, or when nothing ran this week", () => {
    render(<WeekTokensTile week={history(1200)} trend={[]} rows={ROWS} now={NOW} />);
    expect(screen.getByTestId("stat-value").textContent).toMatch(/^1(\.2)?k$/);
    cleanup();
    render(<WeekTokensTile week={history(0)} trend={[]} rows={ROWS.map((row) => ({ ...row, last: NOW - 30 * DAY }))} now={NOW} />);
    expect(screen.getByTestId("stat-value").textContent).toBe("0");
  });
});
