/**
 * AJ-E1601 vitest (D-34) — accounts through another service.
 *
 * - Add account › More lists the services; picking one asks for its plan,
 *   its key (a masked field, pasted once) and the tools to use it with,
 *   each a profile named for the service ("Go").
 * - The key is sent once and never kept or shown: the preview records the
 *   request without it.
 * - The Accounts page shows the account as one card ("OpenCode Go", its
 *   plan and "estimate", or "usage unknown" for Kimi) with its profiles
 *   beneath, each with its tool's mark; never a Claude runway or a sign-in.
 * - Switching to the same tool through another service is never proven.
 */
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AddProviderSheet } from "../AddProviderSheet";
import { AccountCard } from "./AccountCard";
import { BetaMatrix } from "../../components/BetaMatrix";
import { PREVIEW_SERVICES } from "../../preview/services";
import type { AccountInfo } from "../../lib/bridge";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { pick } from "../../test/pick";

const w = () => window as unknown as Record<string, unknown>;
const KEY = "fixture-service-key-0123456789";

const INSTALLS = [
  { provider: "claude-code", display_name: "Claude Code", install_url: "", status: "installed", version: "2.1.296", binary: "/usr/local/bin/claude", notes: [] },
  { provider: "codex", display_name: "Codex", install_url: "", status: "installed", version: "0.163.0", binary: "/usr/local/bin/codex", notes: [] },
];

beforeEach(() => {
  w().__AIJ_PROVIDER_INSTALLS__ = INSTALLS;
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete w().__AIJ_PROVIDER_INSTALLS__;
  delete w().__AIJ_SERVICE_ADDED__;
  delete w().__AIJ_VALIDATION_MATRIX__;
});

describe("Add account › More › Through another service", () => {
  it("lists the services, then takes the plan, the key once and the tools", async () => {
    const added = vi.fn();
    render(<AddProviderSheet onClose={() => null} onChoose={() => null} onServiceAdded={added} />);
    const more = await screen.findByTestId("add-more");
    const cards = within(more).getAllByTestId("service-card");
    expect(cards.map((card) => card.getAttribute("data-service"))).toEqual(["opencode-go", "kimi"]);
    expect(cards[0]!.textContent).toContain("Claude Code, Codex");
    fireEvent.click(within(cards[0]!).getByTestId("service-choose"));

    const key = (await screen.findByTestId("service-key")) as HTMLInputElement;
    const sheet = screen.getByTestId("service-account-sheet");
    expect(key.type).toBe("password");
    expect(key.autocomplete).toBe("off");
    const add = within(sheet).getByTestId("service-add") as HTMLButtonElement;
    expect(add.disabled).toBe(true);
    // The first tool found starts on, named for the service.
    await waitFor(() => expect((within(sheet).getAllByTestId("service-profile-name")[0] as HTMLInputElement).value).toBe("Go"));
    expect(within(sheet).getByTestId("service-usage").textContent).toContain("Estimate");

    // Codex too: its guide names no model, so one is typed.
    const codex = within(sheet).getAllByTestId("service-tool").find((row) => row.getAttribute("data-tool") === "codex")!;
    fireEvent.click(within(codex).getByTestId("service-tool-on"));
    expect((within(codex).getByTestId("service-profile-name") as HTMLInputElement).value).toBe("Go Codex");
    fireEvent.change(within(codex).getByTestId("service-model"), { target: { value: "a-model" } });
    fireEvent.change(key, { target: { value: KEY } });
    pick(within(sheet).getByTestId("service-plan"), "Go Plus");
    expect(add.disabled).toBe(false);
    fireEvent.click(add);

    await waitFor(() => expect(added).toHaveBeenCalled());
    const asked = w().__AIJ_SERVICE_ADDED__ as Array<Record<string, unknown>>;
    expect(asked).toEqual([
      {
        service: "opencode-go",
        plan: "Go Plus",
        profiles: [
          { tool: "claude-code", label: "Go", model: "minimax-m3" },
          { tool: "codex", label: "Go Codex", model: "a-model" },
        ],
      },
    ]);
    expect(JSON.stringify(asked)).not.toContain(KEY);
  });

  it("isn't offered where nothing takes an account through a service", async () => {
    render(<AddProviderSheet onClose={() => null} onChoose={() => null} />);
    await screen.findAllByTestId("provider-card");
    expect(screen.queryByTestId("add-more")).toBeNull();
  });
});

const profile = (id: string, provider: string, label: string, extra: Partial<AccountInfo> = {}): AccountInfo => ({
  id,
  provider,
  label,
  plan: "Go Plus",
  login_state: "signed_in",
  cleanup_opt_in: false,
  renewal_date: null,
  preview_opt_in: false,
  in_place: false,
  service: "OpenCode Go",
  ...extra,
});

const account = (service: (typeof PREVIEW_SERVICES)[number]["service"], ids: string[], extra: Partial<SubscriptionInfo> = {}): SubscriptionInfo => ({
  key: `k-${service.id}`,
  tool: "claude-code",
  state: "signed_in",
  email: null,
  organisation: null,
  reason: null,
  profiles: ids.map((id) => ({ id, label: `Profile ${id}`, folder: `~/AI Juggler/${id}`, plan: "Go Plus", renewal_date: null, fee_usd: null })),
  plan: "Go Plus",
  renewal_date: null,
  fee_usd: null,
  differs: [],
  service,
  ...extra,
});

function card(sub: SubscriptionInfo, profiles: AccountInfo[]) {
  return render(
    <AccountCard
      subscription={sub}
      profiles={profiles}
      recipes={[]}
      displays={[]}
      sessions={[]}
      conversations={[]}
      now={Date.now()}
      canUseInPlace={() => false}
      reportsLimits={() => true}
      onLaunch={() => null}
      onDeleteRecipe={async () => undefined}
      onNewRecipe={() => null}
      onLogin={() => null}
      onRechecked={async () => undefined}
    />,
  );
}

describe("the account card", () => {
  const go = PREVIEW_SERVICES[0]!.service;
  const kimi = PREVIEW_SERVICES[1]!.service;

  it("shows OpenCode Go as one account, an estimate, with both tools' profiles beneath", () => {
    const view = card(account(go, ["go", "go-codex"]), [profile("go", "claude-code", "Go"), profile("go-codex", "codex", "Go Codex")]);
    // Your names are the title; the service says which on the line.
    expect(view.getByTestId("account-name").textContent).toBe("Profile go and Profile go-codex");
    expect(view.getByTestId("account-line").textContent).toBe("OpenCode Go · Go Plus · estimate");
    const usage = view.getByTestId("account-service-usage");
    expect(usage.getAttribute("data-usage")).toBe("estimate");
    expect(usage.textContent).toContain("a fifth of the month's amount");
    // Never Claude's runway or its "measure" offer: they describe nothing here.
    expect(view.queryByTestId("account-unmeasured")).toBeNull();
    const rows = view.getAllByTestId("profile-row");
    expect(rows).toHaveLength(2);
    expect(rows[1]!.textContent).toContain("Go Codex");
    expect(within(rows[0]!).getByTestId("profile-apps").textContent).toContain("Claude Code through OpenCode Go");
    expect(view.getByTestId("account-new-key").textContent).toBe("Replace key…");
  });

  it("says Kimi's usage is unknown", () => {
    const view = card(account(kimi, ["kimi"], { plan: "Pro" }), [profile("kimi", "claude-code", "Kimi", { service: "Kimi", plan: "Pro" })]);
    expect(view.getByTestId("account-line").textContent).toBe("Kimi · Pro · usage unknown");
    expect(view.getByTestId("account-service-usage").getAttribute("data-usage")).toBe("unknown");
  });

  it("without its key here, asks for the key, never a sign-in", () => {
    const sub = account(go, ["go"], { state: "unverified", reason: "its OpenCode Go API key isn't in this Mac's Keychain" });
    const view = card(sub, [profile("go", "claude-code", "Go", { login_state: "logged_out" })]);
    expect(view.getByTestId("account-line").textContent).toContain("Keychain");
    expect(view.getByTestId("profile-key-missing")).toBeTruthy();
    expect(view.queryByTestId("profile-signed-out")).toBeNull();
    expect(view.queryByRole("button", { name: "Sign in" })).toBeNull();
    fireEvent.click(view.getByTestId("account-new-key"));
    const modal = view.getByTestId("service-key-modal");
    expect((within(modal).getByTestId("service-new-key") as HTMLInputElement).type).toBe("password");
  });
});

describe("the switching matrix", () => {
  it("names the same tool through another service, and never calls it proven", async () => {
    w().__AIJ_VALIDATION_MATRIX__ = [
      {
        from: "claude-code",
        to: "claude-code+service",
        from_tool: "claude-code",
        to_tool: "claude-code",
        from_service: false,
        to_service: true,
        mechanism: "transplant",
        researched_ceiling: "F3",
        advertised: "F3",
        badge: "Same conversation, new account · Beta",
        validated: false,
        mechanism_green: true,
        ci_green: false,
        dogfood: null,
        status: "Automatic tests with the real tools aren't in place; not tried by hand yet",
        notes: ["Another service answers on one side"],
      },
    ];
    render(<BetaMatrix />);
    const row = await screen.findByTestId("beta-matrix-row");
    expect(row.textContent).toContain("Claude Code → Claude Code through another service");
    expect(row.getAttribute("data-validated")).toBe("false");
    expect(row.textContent).toContain("Beta — not proven yet");
  });
});
