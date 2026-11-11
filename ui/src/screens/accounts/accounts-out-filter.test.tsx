/**
 * 0.4.2: the top bar's "N accounts out" lands on what it says: Accounts
 * on its Accounts tab, only the out ones, under an "Out · N" chip that ×
 * clears; each out card says when it's back and its best action; with no
 * filter the out ones come first. The filter travels by its id
 * (lib/landing): the event carries it, never a broadcast to the page.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProfilesScreen } from "../Profiles";
import * as bridgeLaunch from "../../lib/ipc/launch";
import * as bridgeAccounts from "../../lib/ipc/accounts";
import * as bridgeProviders from "../../lib/ipc/providers";
import * as bridgeQuota from "../../lib/ipc/quota";
import * as subs from "../../lib/ipc/subscriptions";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { previewNamed, type UnnamedSubscription } from "../../preview/names";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { QuotaDisplayData } from "../../lib/ipc/quota";
import { setToolsForTests } from "../../lib/tools";
import { PREVIEW_TOOLS } from "../../preview/tools";
import { accountsFilterOf, accountsShown, openAccounts } from "../../lib/accounts-filter";
import { landedOn, landingOf, openScreenOn } from "../../lib/landing";

setToolsForTests(PREVIEW_TOOLS);

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const folder = (id: string, label: string): AccountInfo => ({ id, provider: "claude-code", label, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: true });
const account = (key: string, id: string, extra: Partial<UnnamedSubscription> = {}): SubscriptionInfo =>
  previewNamed([
    {
      key, tool: "claude-code", state: "signed_in", email: null, organisation: key, reason: null,
      profiles: [{ id, label: key, folder: `~/p/${id}`, plan: null, renewal_date: null, fee_usd: null }],
      plan: "Max", renewal_date: null, fee_usd: null, differs: [], choices: [], ...extra,
    },
  ])[0]!;
const reading = (id: string, used: number): QuotaDisplayData => ({
    account: id,
    five_hour: { state: used >= 100 ? "exhausted" : "tracking", percentage: used, badge: null, stale_note_ms: null, resets_at_ms: Date.now() + 2 * 3_600_000, reset_basis: null, measured_at_ms: Date.now(), burned_tokens: 0, burned_credits: null, headroom_tokens: null },
    five_hour_omitted_reason: null, weekly: null, weekly_omitted_reason: null,
    renewal: { state: "none", label: "", entered_by_user: false },
    forecast: { state: "learning", tte_min: null, range_min: null, range_max: null, badge: "" },
    unavailable_reason: null, credits: null,
});

describe("0.4.2 the filter, by its id", () => {
  it("is read only from the event's detail, and lands only on its own screen", () => {
    expect(accountsFilterOf("out")).toBe("out");
    expect(accountsFilterOf("low")).toBe("low");
    expect(accountsFilterOf("everything")).toBeNull();
    expect(landingOf({ id: "accounts", filter: "out" })).toEqual({ id: "accounts", filter: "out" });
    expect(landingOf({ id: "accounts" })).toBeNull();
    expect(landingOf({ id: "nowhere", filter: "out" })).toBeNull();
    expect(landedOn({ id: "accounts", filter: "out" }, "cleanup")).toBeNull();
  });

  it("opening carries it on the screen's own event", () => {
    const seen: unknown[] = [];
    const on = (event: Event) => seen.push(event instanceof CustomEvent ? event.detail : null);
    window.addEventListener("aij:open-screen", on);
    openAccounts("out");
    openAccounts();
    openScreenOn("cleanup", "attention");
    window.removeEventListener("aij:open-screen", on);
    expect(seen).toEqual([{ id: "accounts", filter: "out" }, { id: "accounts" }, { id: "cleanup", filter: "attention" }]);
  });

  it("shows only the Accounts it names; with none, out ones first, each side in order", () => {
    const room = (row: string) => (row.startsWith("out") ? "out" : row.startsWith("low") ? "low" : null);
    const rows = ["a", "out1", "low1", "b", "out2"];
    expect(accountsShown(rows, room, "out")).toEqual(["out1", "out2"]);
    expect(accountsShown(rows, room, "low")).toEqual(["low1"]);
    expect(accountsShown(rows, room, null)).toEqual(["out1", "out2", "a", "low1", "b"]);
  });
});

describe("0.4.2 Accounts, filtered to the out ones", () => {
  beforeEach(() => {
    // Last chosen: the Apps tab. The filter opens on Accounts all the same.
    localStorage.setItem("aij.accounts.view", "apps");
    vi.spyOn(bridgeLaunch, "launchRecipesList").mockResolvedValue([]);
    vi.spyOn(bridgeProviders, "providersCapabilities").mockResolvedValue([]);
    vi.spyOn(bridgeAccounts, "accountsList").mockResolvedValue([folder("f-room", "Room"), folder("f-out", "Spent"), folder("f-out2", "Spent too")]);
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([account("Roomy", "f-room"), account("Spent", "f-out"), account("Spent too", "f-out2")]);
    vi.spyOn(bridgeQuota, "quotaDisplayAll").mockResolvedValue([reading("f-room", 20), reading("f-out", 100), reading("f-out2", 100)]);
  });
  afterEach(() => localStorage.clear());

  it("on the Accounts tab, only the out ones under an Out · 2 chip; each says when it's back and where to go; × shows all, out first", async () => {
    const cleared: unknown[] = [];
    const view = render(<ProfilesScreen filter="out" onFilter={(next) => cleared.push(next)} openTerminal={() => {}} onAddAccounts={() => {}} />);
    const chip = await screen.findByTestId("accounts-filter");
    expect(chip.textContent).toContain("Out · 2");
    expect(screen.getByRole("radio", { name: /Accounts/ }).getAttribute("aria-checked")).toBe("true");
    const cards = await screen.findAllByTestId("account-card");
    expect(cards.map((card) => within(card).getByTestId("account-name").textContent)).toEqual(["Spent", "Spent too"]);
    for (const card of cards) {
      expect(within(card).getByTestId("account-out-mark").textContent).toMatch(/^Out · resets /);
      expect(within(card).getByTestId("account-out-action").textContent).toBe("New session on Roomy");
    }
    // Its best action starts New session on the Account with room, by its folder's id.
    const asked: unknown[] = [];
    const on = (event: Event) => asked.push(event instanceof CustomEvent ? event.detail : null);
    window.addEventListener("aij:new-session", on);
    fireEvent.click(within(cards[0]!).getByTestId("account-out-action"));
    window.removeEventListener("aij:new-session", on);
    expect(asked).toEqual([{ accountId: "f-room" }]);
    fireEvent.click(screen.getByRole("button", { name: "Clear the Out filter: show every account" }));
    expect(cleared).toEqual([null]);
    // Unfiltered: every Account, the out ones first.
    view.rerender(<ProfilesScreen filter={null} openTerminal={() => {}} onAddAccounts={() => {}} />);
    expect(screen.queryByTestId("accounts-filter")).toBeNull();
  });

  it("with no filter, out Accounts sort first", async () => {
    localStorage.setItem("aij.accounts.view", "accounts");
    render(<ProfilesScreen openTerminal={() => {}} onAddAccounts={() => {}} />);
    const cards = await screen.findAllByTestId("account-card");
    expect(cards.map((card) => within(card).getByTestId("account-name").textContent)).toEqual(["Spent", "Spent too", "Roomy"]);
  });
});
