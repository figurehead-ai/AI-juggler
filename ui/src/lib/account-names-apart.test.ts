/**
 * SK48-8 (the 0.4.8 soak, K2 step 4): a project's Account filter read
 * "Work · 167, Personal · 167, Personal · 166" when a Claude and a Codex
 * account were both called Personal, and "Who did the work" the same.
 * Wherever accounts are listed together, two that share a name carry
 * their tool beside it; one shared helper.
 */
import { describe, expect, it } from "vitest";
import { accountNamesApart, type SubscriptionInfo } from "./subscriptions";
import { sessionsByAccount, sessionsWords } from "../pages/project/sessions-by-account";
import { accountGroups } from "../components/usage/accounts";
import { accountGroups as runwayGroups } from "../components/HomeRunway";
import { byAccount, type Target } from "../pages/session/ContinueOn";
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

describe("accounts listed together", () => {
  it("tell two of one name apart by their tool, and leave a name only one has", () => {
    const named = accountNamesApart(ROWS);
    expect(ROWS.map(named)).toEqual(["Work", "Personal (Claude)", "Personal (Codex)"]);
    expect(accountNamesApart([ROWS[1]!])(ROWS[1]!)).toBe("Personal");
  });

  it("the project's Who did the work says which Personal", () => {
    const counted = { byAccount: new Map([["acct:work", 167], ["acct:personal", 167], ["acct:codex", 166]]), total: 500 };
    const { items } = sessionsByAccount({ counted, local: [], subscriptions: ROWS });
    expect(sessionsWords(items)).toBe("Personal (Claude) · 167 sessions, Work · 167 sessions, Personal (Codex) · 166 sessions");
  });

  it("Usage's account groups say which Personal", () => {
    const groups = accountGroups(ROWS, new Map());
    expect(groups.get("f-codex")?.label).toBe("Personal (Codex)");
    expect(groups.get("f-work")?.label).toBe("Work");
  });

  // The 0.5.0 walk, F3: Home's "Runway by account" listed two plain
  // "Personal" rows.
  it("Home's Runway by account says which Personal", () => {
    const folder = (id: string, provider: string) => ({ id, label: id, provider }) as unknown as AccountInfo;
    const folders = [folder("f-work", "claude-code"), folder("f-personal", "claude-code"), folder("f-codex", "codex")];
    expect(runwayGroups(folders, ROWS).map((group) => group.name)).toEqual(["Work", "Personal (Claude)", "Personal (Codex)"]);
    // A twin that isn't listed leaves the other's name alone.
    expect(runwayGroups(folders.slice(0, 2), ROWS).map((group) => group.name)).toEqual(["Work", "Personal"]);
  });

  it("the session page's Apps that can take it over says which Personal", () => {
    const target = (profileId: string): Target => ({ profileId, label: profileId, recipeId: `r-${profileId}`, blocked: null, setupNeeded: false, badge: null, crossTool: false, room: null, suggested: false });
    const { others } = byAccount([target("f-personal"), target("f-codex")], ROWS, "f-work");
    expect(others.map((group) => group.name)).toEqual(["Personal (Claude)", "Personal (Codex)"]);
  });
});
