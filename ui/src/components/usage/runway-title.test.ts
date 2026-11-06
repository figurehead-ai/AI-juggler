/**
 * Walk A10: a folder that isn't signed in has no Account to name; its
 * Runway card in Usage says "Not signed in", as Home and Accounts do.
 */
import { expect, it } from "vitest";
import { accountGroups, runwayTitle } from "./accounts";
import type { SubscriptionInfo } from "../../lib/subscriptions";

const sub = (over: Partial<SubscriptionInfo>): SubscriptionInfo => ({
  key: "k",
  tool: "claude-code",
  state: "signed_in",
  email: null,
  organisation: null,
  reason: null,
  profiles: [],
  plan: null,
  renewal_date: null,
  fee_usd: null,
  differs: [],
  choices: [],
  service: null,
  display_name: "",
  ...over,
});

it("titles a folder that isn't signed in 'Not signed in', and an Account by its name", () => {
  const subscriptions = [
    sub({ key: "acct", account_id: "acct:anthropic:1", name: "Work", profiles: [{ id: "work", label: "Work", folder: "", plan: null, renewal_date: null, fee_usd: null }] }),
    sub({ key: "out", tool: "codex", state: "signed_out", profiles: [{ id: "codex", label: "Personal · default", folder: "", plan: null, renewal_date: null, fee_usd: null }] }),
  ];
  const groups = accountGroups(subscriptions, new Map([["work", "Work"], ["codex", "Personal · default"]]));
  expect(runwayTitle(groups.get("codex")!, subscriptions)).toBe("Not signed in");
  expect(runwayTitle(groups.get("work")!, subscriptions)).toBe(groups.get("work")!.label);
});
