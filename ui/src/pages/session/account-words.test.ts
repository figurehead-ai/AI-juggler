import { describe, expect, it } from "vitest";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { sessionAccountWords } from "./account-words";

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

describe("F3 the session page names its twin Account by its tool", () => {
  it("the Runway card's title and the Where it runs row", () => {
    expect(sessionAccountWords(ROWS, ROWS[1])).toEqual({ runway: "Personal (Claude)", where: "Personal (Claude)" });
    expect(sessionAccountWords(ROWS, ROWS[2]).runway).toBe("Personal (Codex)");
    // A name only one Account has stays as it is.
    expect(sessionAccountWords(ROWS, ROWS[0]).runway).toBe("Work");
    // Signed out: no Runway title, the row still names it.
    expect(sessionAccountWords(ROWS, { ...ROWS[1]!, state: "signed_out" })).toEqual({ runway: null, where: "Personal (Claude)" });
    expect(sessionAccountWords(ROWS, undefined)).toEqual({ runway: null, where: null });
  });
});
