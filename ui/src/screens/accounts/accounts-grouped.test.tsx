/**
 * AJ-E2109 vitest (D-35) — Accounts shows each subscription once, with
 * the profiles signed in to it beneath; profiles on one account share one
 * runway; Not signed in comes last; values the profiles disagree on show
 * per profile. AJ-E2108: a profile on the session's own account gives no
 * new room, so it isn't offered as a move.
 */
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProfilesScreen } from "../Profiles";
import * as bridge from "../../lib/bridge";
import * as subs from "../../lib/ipc/subscriptions";
import { accountDisplay, accountName, shareAccount, type SubscriptionInfo } from "../../lib/subscriptions";
import { byAccount, setupWords } from "../../pages/session/ContinueOn";
import type { AccountInfo, QuotaDisplayData } from "../../lib/bridge";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const profile = (id: string, label: string, extra: Partial<AccountInfo> = {}): AccountInfo => ({
  id,
  provider: "claude-code",
  label,
  plan: null,
  login_state: "logged_in",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: true,
  ...extra,
});

const member = (id: string, extra: Partial<SubscriptionInfo["profiles"][number]> = {}) => ({ id, folder: `~/p/${id}`, plan: null, renewal_date: null, fee_usd: null, ...extra });

const account = (key: string, ids: string[], extra: Partial<SubscriptionInfo> = {}): SubscriptionInfo => ({
  key,
  tool: "claude-code",
  state: "signed_in",
  email: "o…@work.com",
  organisation: null,
  reason: null,
  profiles: ids.map((id) => member(id)),
  plan: "Max",
  renewal_date: null,
  fee_usd: null,
  differs: [],
  ...extra,
});

const measured = (id: string, used: number, at: number): QuotaDisplayData =>
  ({
    account: id,
    five_hour: { state: "tracking", percentage: used, badge: null, stale_note_ms: null, resets_at_ms: null, reset_basis: null, measured_at_ms: at, burned_tokens: 0, burned_credits: null, headroom_tokens: null },
    five_hour_omitted_reason: null,
    weekly: null,
    weekly_omitted_reason: null,
    renewal: { state: "none", label: "", entered_by_user: false },
    forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "" },
    unavailable_reason: null,
    credits: null,
  }) as QuotaDisplayData;

describe("AJ-E2109 the Accounts screen", () => {
  beforeEach(() => {
    vi.spyOn(bridge, "launchRecipesList").mockResolvedValue([]);
    vi.spyOn(bridge, "quotaDisplayAll").mockResolvedValue([]);
    vi.spyOn(bridge, "providersCapabilities").mockResolvedValue([]);
  });

  it("two subscriptions over six profiles: two accounts, each with its profiles, Not signed in last", async () => {
    vi.spyOn(bridge, "accountsList").mockResolvedValue([
      profile("w1", "Work"),
      profile("p1", "Personal"),
      profile("w2", "Work 2"),
      profile("w3", "Client"),
      profile("p2", "Weekend"),
      profile("new", "Fresh", { login_state: "logged_out" }),
    ]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([
      account("work", ["w1", "w2", "w3"], { organisation: "Acme", plan: "Team" }),
      account("me", ["p1", "p2"], { email: "o…@me.com" }),
      account("out", ["new"], { state: "signed_out", email: null, plan: null }),
    ]);
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} />);
    const cards = await screen.findAllByTestId("account-card");
    expect(cards.map((card) => card.querySelector("[data-testid='account-name']")?.textContent)).toEqual(["Acme", "Claude Max", "Not signed in"]);
    expect(within(cards[0]!).getAllByTestId("account-label").map((n) => n.textContent)).toEqual(["Work", "Work 2", "Client"]);
    expect(within(cards[0]!).getByTestId("account-profiles-count").textContent).toBe("3 profiles share this account");
    // One runway per account, none for profiles that aren't signed in.
    expect(within(cards[0]!).getAllByTestId("account-runway")).toHaveLength(1);
    expect(within(cards[2]!).queryByTestId("account-runway")).toBeNull();
    expect(within(cards[2]!).getByTestId("profile-signed-out").textContent).toBe("Not signed in");
    expect(within(cards[1]!).getAllByTestId("profile-folder").map((n) => n.textContent)).toEqual(["~/p/p1", "~/p/p2"]);
    expect(screen.getByTestId("accounts-definition").textContent).toContain("One account can have several profiles.");
  });

  it("one profile: one account; nothing measured says so and still shows its usage", async () => {
    vi.spyOn(bridge, "accountsList").mockResolvedValue([profile("w1", "Work")]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([account("work", ["w1"])]);
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} />);
    const runway = await screen.findByTestId("account-runway");
    expect(runway.textContent).toContain("Not measured yet");
    // Its usage shows either way, and where limits come from, with the way to its page.
    expect(screen.getByTestId("runway-usage").textContent).toContain("Last 7 days");
    const note = await screen.findByTestId("account-unmeasured");
    expect(note.textContent).toContain("No current reading.");
    expect(screen.getByTestId("account-measure").textContent).toBe("Measure its limits…");
    expect(screen.queryByTestId("account-profiles-count")).toBeNull();
  });

  it("what the profiles disagree on shows on each profile", async () => {
    vi.spyOn(bridge, "accountsList").mockResolvedValue([profile("a", "A"), profile("b", "B")]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([
      { ...account("x", []), profiles: [member("a", { fee_usd: 100 }), member("b", { fee_usd: 200 })], differs: ["fee"] },
    ]);
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} />);
    await waitFor(() => expect(screen.getAllByTestId("profile-apps")).toHaveLength(2));
    expect(screen.getAllByTestId("profile-apps").map((n) => n.textContent)).toEqual(["Claude Code · $100/month", "Claude Code · $200/month"]);
    expect(screen.getByTestId("account-line").textContent).not.toContain("$");
  });
});

describe("AJ-E2109 accounts in words", () => {
  const rows = [account("work", ["w1", "w2"]), account("me", ["p1"], { email: "o…@me.com", plan: null })];

  it("names an account by its team, else its tool and plan", () => {
    expect(accountName(account("t", [], { organisation: "Acme" }))).toBe("Acme");
    expect(accountName(rows[0]!)).toBe("Claude Max");
    expect(accountName(rows[1]!)).toBe("Claude account");
  });

  it("two profiles on one login share one runway: the freshest measured reading", () => {
    expect(shareAccount(rows, "w1", "w2")).toBe(true);
    expect(shareAccount(rows, "w1", "p1")).toBe(false);
    const display = accountDisplay(rows[0]!, [measured("w1", 40, 1), measured("w2", 70, 2)]);
    expect(display?.account).toBe("w2");
  });

  it("AJ-E2108: a profile on the session's own account gives no new room", () => {
    const targets = [
      { profileId: "w2", label: "Work 2", recipeId: "r-w2", blocked: null, badge: null, crossTool: false },
      { profileId: "p1", label: "Personal", recipeId: "r-p1", blocked: null, badge: null, crossTool: false },
    ];
    const { others, sameAccount } = byAccount(targets, rows, "w1");
    expect(others.map((group) => group.targets.map((t) => t.profileId))).toEqual([["p1"]]);
    expect(sameAccount.map((t) => t.profileId)).toEqual(["w2"]);
  });

  it("the setup check in one line", () => {
    expect(setupWords({ from: "a", to: "b", already_there: 3, offers: [] })).toBe("Ready: same tools set up");
    const server = (id: string, verdict: string) => ({ id, kind: "mcp-server", name: id, scope: "user", verdict, note: null, suggested: true, about: null, used: null, origin: "own", provided_by: null, get_it: null, can_copy: false });
    expect(setupWords({ from: "a", to: "b", already_there: 0, offers: [server("x", "copy"), server("y", "copy")] })).toBe("Needs 2 MCP servers, set up as it moves");
    expect(setupWords({ from: "a", to: "b", already_there: 0, offers: [server("x", "needs-you")] })).toBe("Needs 1 MCP server, some set up by you");
    // Only what this conversation used counts.
    expect(setupWords({ from: "a", to: "b", already_there: 0, offers: [{ ...server("x", "turn-on"), used: false }] })).toBe(
      "Ready: has what this conversation used",
    );
  });
});
