/**
 * F3 (SK48-8, the 0.5.0 re-walk): Settings › Accounts › Renewal dates
 * lists every Account together, one row each; a Claude Code and a Codex
 * Account both called Personal read "Personal" twice. They carry their
 * tool, as every list of Accounts says them.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SubscriptionInfo } from "../lib/subscriptions";
import type { AccountInfo } from "../types/generated/AccountInfo";

const account = (key: string, name: string, tool: string): SubscriptionInfo => ({
  key,
  account_id: `acct:${key}`,
  display_name: name,
  tool,
  state: "signed_in",
  email: null,
  organisation: null,
  reason: null,
  profiles: [{ id: `f-${key}`, label: key, folder: `~/.config/${key}`, plan: null, renewal_date: null, fee_usd: null }],
  plan: null,
  renewal_date: null,
  fee_usd: null,
  differs: [],
  choices: [],
});
const ROWS = [account("work", "Work", "claude-code"), account("personal", "Personal", "claude-code"), account("codex", "Personal", "codex")];

vi.mock("../lib/subscriptions", async (original) => ({ ...(await original<typeof import("../lib/subscriptions")>()), useSubscriptions: () => ROWS }));

afterEach(cleanup);

describe("F3 renewal dates", () => {
  it("names twin Accounts by their tool", async () => {
    const { RenewalSettingsCard } = await import("./SettingsSections");
    const folders: AccountInfo[] = ROWS.map((row) => ({ id: `f-${row.key}`, label: row.key, provider: row.tool, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false }));
    render(<RenewalSettingsCard accounts={folders} />);
    const names = screen.getAllByTestId("settings-renewal").map((row) => row.querySelector("label")?.textContent ?? row.textContent);
    expect(names.join(" | ")).toContain("Personal (Claude)");
    expect(names.join(" | ")).toContain("Personal (Codex)");
  });
});
