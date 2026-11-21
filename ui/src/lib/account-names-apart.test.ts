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
});
