/**
 * AJ-E2109 vitest (D-35) — Accounts shows each subscription once, with
 * the profiles signed in to it beneath; profiles on one account share one
 * runway; Not signed in comes last; values the profiles disagree on show
 * per profile. AJ-E2108: a profile on the session's own account gives no
 * new room, so it isn't offered as a move.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProfilesScreen } from "../Profiles";
import * as bridge from "../../types/generated/InstanceInfo";
import * as bridgeLaunch from "../../lib/ipc/launch";
import * as bridgeAccounts from "../../lib/ipc/accounts";
import * as bridgeProviders from "../../lib/ipc/providers";
import * as bridgeQuota from "../../lib/ipc/quota";
import * as subs from "../../lib/ipc/subscriptions";
import { accountDisplay, accountLine, accountName, shareAccount, type SubscriptionInfo } from "../../lib/subscriptions";
import { previewNamed, type UnnamedSubscription } from "../../preview/names";
import { byAccount, setupWords } from "../../pages/session/ContinueOn";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { QuotaDisplayData } from "../../lib/ipc/quota";
import { setToolsForTests } from "../../lib/tools";
import { OPEN_ACCOUNT } from "../../lib/accounts-page";
import { PREVIEW_TOOLS } from "../../preview/tools";

// This file's top level reads tool names before the preview's data loads (src/test/setup.ts).
setToolsForTests(PREVIEW_TOOLS);

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

const member = (id: string, extra: Partial<SubscriptionInfo["profiles"][number]> = {}) => ({ id, label: `Profile ${id}`, folder: `~/p/${id}`, plan: null, renewal_date: null, fee_usd: null, ...extra });

/** An account as the backend hands it: named by its rule (the preview's
 * stand-in for it). */
const account = (key: string, ids: string[], extra: Partial<UnnamedSubscription> = {}): SubscriptionInfo =>
  previewNamed([
    {
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
      choices: [],
      ...extra,
    },
  ])[0]!;

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
    vi.spyOn(bridgeLaunch, "launchRecipesList").mockResolvedValue([]);
    vi.spyOn(bridgeQuota, "quotaDisplayAll").mockResolvedValue([]);
    vi.spyOn(bridgeProviders, "providersCapabilities").mockResolvedValue([]);
  });

  it("two subscriptions over six profiles: two accounts, each with its profiles, Not signed in last", async () => {
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([
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
    // D-62: an Account is named its own way (its team, else its plan),
    // never by its Apps' names; the line says the rest, once.
    expect(cards.map((card) => card.querySelector("[data-testid='account-name']")?.textContent)).toEqual(["Acme", "Claude Max", "Not signed in"]);
    expect(cards[0]!.querySelector("[data-testid='account-line']")?.textContent).toBe("o…@work.com · Team");
    // D-62: each row is an App, named by its tool and folder.
    expect(within(cards[0]!).getAllByTestId("account-label").map((n) => n.textContent)).toEqual(["Claude Code · Work", "Claude Code · Work 2", "Claude Code · Client"]);
    expect(within(cards[0]!).getByTestId("account-profiles-count").textContent).toBe("3 apps share this account");
    // One runway per account, none for profiles that aren't signed in.
    expect(within(cards[0]!).getAllByTestId("account-runway")).toHaveLength(1);
    expect(within(cards[2]!).queryByTestId("account-runway")).toBeNull();
    expect(within(cards[2]!).getByTestId("profile-signed-out").textContent).toBe("Not signed in");
    // The 0.4 review (63): an App's folder is said on its own page, not on its row.
    expect(within(cards[1]!).queryAllByTestId("profile-folder")).toHaveLength(0);
    expect(screen.getByTestId("accounts-definition").textContent).toContain("one account can serve several apps.");
  });

  it("an Account whose Apps were all accepted under one name at import is called by it (AJ-E3202 req. 3, 0.4.1)", async () => {
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([profile("w1", "Work"), profile("w2", "Work")]);
    const accepted = { accepted_name: "Work" };
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([
      account("work", [], { plan: null, profiles: [member("w1", accepted), member("w2", accepted)] }),
    ]);
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} />);
    const [card] = await screen.findAllByTestId("account-card");
    expect(card!.querySelector("[data-testid='account-name']")?.textContent).toBe("Work");
    expect(card!.querySelector("[data-testid='account-line']")?.textContent).toBe("o…@work.com");
  });

  it("an App whose sign-in couldn't be checked stays under its Account and says so (0.4.1)", async () => {
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([profile("w1", "Work"), profile("w2", "Work 2")]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([
      account("work", [], { profiles: [member("w1", { unchecked: "Claude Code's state file couldn't be read" }), member("w2")] }),
    ]);
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} />);
    const card = await screen.findByTestId("account-card");
    expect(card.querySelector("[data-testid='account-name']")?.textContent).toBe("Claude Max");
    const rows = within(card).getAllByTestId("profile-row");
    expect(rows).toHaveLength(2);
    expect(within(rows[0]!).getByTestId("profile-sign-in-unchecked").textContent).toBe("Couldn't check its sign-in · Claude Code's state file couldn't be read");
    expect(within(rows[1]!).queryByTestId("profile-sign-in-unchecked")).toBeNull();
    // Never "Account unknown" or Not signed in for it.
    expect(card.textContent).not.toContain("Account unknown");
    expect(within(rows[0]!).queryByTestId("profile-signed-out")).toBeNull();
  });

  it("one profile: one account; nothing measured says so once, in one line, and still shows its usage", async () => {
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([profile("w1", "Work")]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([account("work", ["w1"])]);
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} />);
    // The 0.4 review (50, 79): one "Not measured yet · Measure its limits ›" line, and no dial.
    const note = await screen.findByTestId("account-unmeasured");
    expect(note.textContent).toBe("Not measured yet ▸·Measure its limits ›");
    const card = screen.getByTestId("account-card");
    expect(within(card).queryByTestId("window-gauge")).toBeNull();
    expect(card.textContent!.split("Not measured yet").length - 1).toBe(1);
    // Its usage shows either way.
    expect(screen.getByTestId("runway-usage").textContent).toContain("Last 7 days");
    // Where limits come from is folded behind it (63).
    expect(screen.queryByTestId("account-unmeasured-why")).toBeNull();
    fireEvent.click(screen.getByTestId("account-unmeasured-fold"));
    expect(screen.getByTestId("account-unmeasured-why").textContent).toContain("No current reading.");
    expect(screen.getByTestId("account-unmeasured-fold").getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByTestId("account-measure").textContent).toBe("Measure its limits ›");
    expect(screen.queryByTestId("account-profiles-count")).toBeNull();
  });

  it("Measure its limits on an Account's card opens the Account's page (AJ-E3205 R3)", async () => {
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([profile("w1", "Work")]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([account("work", ["w1"], { account_id: "acct:work" })]);
    const inspected = vi.fn();
    const opened = vi.fn();
    const listen = (event: Event) => opened(event instanceof CustomEvent ? event.detail : null);
    window.addEventListener(OPEN_ACCOUNT, listen);
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={inspected} />);
    fireEvent.click(await screen.findByTestId("account-measure"));
    window.removeEventListener(OPEN_ACCOUNT, listen);
    expect(opened).toHaveBeenCalledWith({ id: "acct:work" });
    expect(inspected).not.toHaveBeenCalled();
  });

  it("a measured account keeps its dial, and says no unmeasured line", async () => {
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([profile("w1", "Work")]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([account("work", ["w1"])]);
    vi.spyOn(bridgeQuota, "quotaDisplayAll").mockResolvedValue([measured("w1", 40, Date.now())]);
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} />);
    await waitFor(() => expect(screen.getAllByTestId("window-gauge").length).toBeGreaterThan(0));
    expect(screen.queryByTestId("account-unmeasured")).toBeNull();
  });

  it("an App row says where its sessions are open, and never its own renewal (the 0.4 review, 17 and 64)", async () => {
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([profile("a", "A"), profile("b", "B")]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([
      { ...account("x", []), profiles: [member("a", { renewal_date: "2030-11-12" }), member("b", { renewal_date: "2030-12-03" })], differs: ["renewal"] },
    ]);
    const running = [{ id: "s1", account_id: "a", status: "running", kind: "cli", provider: "claude-code" } as unknown as bridge.InstanceInfo];
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} onInspectAccount={() => {}} sessions={running} />);
    await waitFor(() => expect(screen.getAllByTestId("profile-apps")).toHaveLength(2));
    expect(screen.getAllByTestId("profile-apps").map((n) => n.textContent)).toEqual(["open in AI Juggler", "Claude Code"]);
    for (const row of screen.getAllByTestId("profile-row")) expect(row.textContent).not.toMatch(/Renews|Nov|Dec/);
    // AJ-E2109 R3: how many are open now, or that none is.
    expect(screen.getAllByTestId("account-activity").map((n) => n.textContent)).toEqual(["1 open now", "Nothing open"]);
  });

  it("what the profiles disagree on shows on each profile", async () => {
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([profile("a", "A"), profile("b", "B")]);
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

  it("names an account by the name the backend gives it (D-62), never a folder's", () => {
    // The rule is the backend's (`subscriptions::name_all`, its cases in
    // Rust); every screen reads `display_name`.
    expect(accountName({ ...rows[0]!, display_name: "Day job" })).toBe("Day job");
    // Its line leaves out what its name already says.
    const seats = previewNamed(["a", "b", "c"].map((id) => ({ ...account(id, [id]), organisation: "Acme", email: `${id}…@acme.com` })));
    expect(accountLine(seats[0]!)).toBe("Max");
    expect(accountLine(account("a", ["a"], { organisation: "Acme", email: "a…@acme.com" }))).toBe("a…@acme.com · Max");
  });

  it("the preview names its accounts as the backend would (the galleries' stand-in)", () => {
    // Five seats in one team (the 0.2 walk, F3): each is told apart by its
    // email beside the team's name, never five "Acme"s.
    const seats = previewNamed(["a", "b", "c"].map((id) => ({ ...account(id, [id]), organisation: "Acme", email: `${id}…@acme.com` })));
    expect(seats.map(accountName)).toEqual(["Acme · a…@acme.com", "Acme · b…@acme.com", "Acme · c…@acme.com"]);
    expect(accountName(account("a", ["a"], { organisation: "Acme" }))).toBe("Acme");
    // No plan, so named by its masked email, and two read the same: the
    // tool where theirs differ, else a number among them by id.
    const masked = ["x1", "x2"].map((id) => ({ ...account(id, [id], { email: "c…@example.com", plan: null }), account_id: `acct:${id}` }));
    expect(previewNamed(masked).map(accountName)).toEqual(["c…@example.com (1)", "c…@example.com (2)"]);
    expect(previewNamed([masked[0]!, { ...masked[1]!, tool: "codex" }]).map(accountName)).toEqual(["Claude · c…@example.com", "Codex · c…@example.com"]);
    // Named by you: your name, whatever its folders are called.
    expect(accountName(account("w", ["w1"], { name: "Day job" }))).toBe("Day job");
    // No team: its tool and plan; no plan: its email; nothing: the tool's.
    expect(accountName(account("t", []))).toBe("Claude Max");
    expect(accountName(account("t", [], { plan: "max-20x" }))).toBe("Claude Max 20x");
    expect(accountName(rows[1]!)).toBe("o…@me.com");
    expect(accountName(account("t", [], { plan: null, email: null }))).toBe("Claude account");
    // AJ-E3202 req. 3: one name all its Apps were accepted under at import,
    // after team and plan; never a label the user didn't accept.
    const work = (extra: Partial<UnnamedSubscription> = {}, names = ["Work", "Work"]) =>
      account("w", [], { plan: null, profiles: names.map((name, at) => member(`w${at}`, { label: "Work", accepted_name: name || null })), ...extra });
    expect(accountName(work())).toBe("Work");
    expect(accountName(work({ name: "Day job" }))).toBe("Day job");
    expect(accountName(work({ plan: "max-20x" }))).toBe("Claude Max 20x");
    expect(accountName(work({ organisation: "Acme" }))).toBe("Acme");
    expect(accountName(work({}, ["Work", "Home"]))).toBe("o…@work.com");
    expect(accountName(work({}, ["Work", ""]))).toBe("o…@work.com");
    expect(accountName(work({ apps: ["cli:w0", "app:claude-app:/p"] }))).toBe("Work");
    // A sign-in that can't be read, and folders not signed in.
    expect(accountName(account("u", ["u"], { state: "unverified" }))).toBe("Account unknown");
    expect(accountName(account("o", ["o"], { state: "signed_out" }))).toBe("Not signed in");
  });

  it("never shows a sign-in that records no email (Codex's is an id)", () => {
    expect(accountLine(account("c", ["c"], { tool: "codex", email: null, plan: null }))).toBeNull();
  });

  it("two profiles on one login share one runway: the freshest measured reading", () => {
    expect(shareAccount(rows, "w1", "w2")).toBe(true);
    expect(shareAccount(rows, "w1", "p1")).toBe(false);
    const display = accountDisplay(rows[0]!, [measured("w1", 40, 1), measured("w2", 70, 2)]);
    expect(display?.account).toBe("w2");
  });

  it("AJ-E2108: a profile on the session's own account gives no new room", () => {
    const targets = [
      { profileId: "w2", label: "Work 2", recipeId: "r-w2", blocked: null, setupNeeded: false, badge: null, crossTool: false, room: null, suggested: false },
      { profileId: "p1", label: "Personal", recipeId: "r-p1", blocked: null, setupNeeded: false, badge: null, crossTool: false, room: null, suggested: false },
    ];
    const { others, sameAccount } = byAccount(targets, rows, "w1");
    expect(others.map((group) => group.targets.map((t) => t.profileId))).toEqual([["p1"]]);
    expect(sameAccount.map((t) => t.profileId)).toEqual(["w2"]);
  });

  it("the setup check in one line", () => {
    expect(setupWords({ from: "a", to: "b", already_there: 3, offers: [] })).toBe("Ready: same tools set up");
    const server = (id: string, verdict: string) => ({ id, kind: "mcp-server", name: id, scope: "user", verdict, note: null, suggested: true, about: null, used: null, origin: "own", provided_by: null, get_it: null, can_copy: false });
    expect(setupWords({ from: "a", to: "b", already_there: 0, offers: [server("x", "copy"), server("y", "copy")] })).toBe("Needs 2 MCP servers, set up as it switches");
    expect(setupWords({ from: "a", to: "b", already_there: 0, offers: [server("x", "needs-you")] })).toBe("Needs 1 MCP server, some set up by you");
    // Only what this conversation used counts.
    expect(setupWords({ from: "a", to: "b", already_there: 0, offers: [{ ...server("x", "turn-on"), used: false }] })).toBe(
      "Ready: has what this conversation used",
    );
  });
});
