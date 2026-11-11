/**
 * D-62 (AJ-E3202 req. 4): an Account's terms in words — the one-time
 * question where its Apps disagree, the call to choose, and each value as
 * it's said.
 */
import { describe, expect, it } from "vitest";
import { chooseWords, choiceQuestion, termConsequence, termValueWords } from "./account-terms";
import type { SubscriptionInfo } from "./ipc/subscriptions";
import { previewNamed } from "../preview/names";

const NOW = new Date(2026, 10, 2, 12).getTime();
const member = (id: string, label: string) => ({ id, label, folder: `~/${id}`, plan: null, renewal_date: null, fee_usd: null });
const sub = (choices: SubscriptionInfo["choices"]): SubscriptionInfo =>
  // Named as the backend names it (the preview's stand-in for its rule).
  previewNamed([
    {
      key: "k",
      account_id: "acct:anthropic:k",
      name: null,
      tool: "claude-code",
      state: "signed_in",
      email: null,
      organisation: null,
      reason: null,
      profiles: [member("work", "Work"), member("work-2", "Work 2"), member("team", "Team")],
      plan: null,
      renewal_date: null,
      fee_usd: null,
      differs: choices.map((choice) => choice.term),
      choices,
    },
  ])[0]!;

describe("D-62 an Account's terms, in words", () => {
  it("asks the maintainer's question: who says what, and which", () => {
    const row = sub([{ term: "renewal", options: [{ value: "2026-11-12", folders: ["work"], account: false }, { value: "2026-12-03", folders: ["work-2"], account: false }] }]);
    expect(choiceQuestion(row, row.choices[0]!, NOW)).toBe("Work and Work 2 say different renewal dates: 12 Nov or 3 Dec?");
    expect(chooseWords(row)).toBe("Choose its renewal date");
  });

  it("names the account's own value first, three ways with commas, another year with its year", () => {
    const row = sub([
      { term: "fee", options: [{ value: "200", folders: [], account: true }, { value: "100", folders: ["work"], account: false }, { value: "20", folders: ["work-2", "team"], account: false }] },
      { term: "plan", options: [{ value: "max-20x", folders: ["work"], account: false }, { value: "pro", folders: ["team"], account: false }] },
    ]);
    expect(choiceQuestion(row, row.choices[0]!, NOW)).toBe("This account, Work, Work 2 and Team say different monthly fees: $200/month, $100/month or $20/month?");
    expect(choiceQuestion(row, row.choices[1]!, NOW)).toBe("Work and Team say different plans: Max 20x or Pro?");
    expect(chooseWords(row)).toBe("Choose its monthly fee and plan");
    expect(termValueWords("renewal", "2027-01-05", NOW)).toBe("5 Jan 2027");
  });

  it("says what the choice changes, and that nothing changes until then", () => {
    expect(termConsequence("renewal")).toMatch(/one renewal date: renewal reminders .* every app on it\. Until you choose, each app keeps its own\.$/);
  });
});
