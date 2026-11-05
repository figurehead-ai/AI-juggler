/**
 * The P3 walk: a signed-out group of folders ("Not signed in") was offered
 * as an Account member, with no Account behind it to add.
 */
import { describe, expect, it } from "vitest";
import { broadCandidates } from "./new-pool";

const account = (key: string, state: string, display_name: string) => ({ key, state, display_name }) as never;

describe("Account members to add", () => {
  it("offers the signed-in and the unknown Accounts, never a signed-out group", () => {
    const rows = broadCandidates({ kind: "account", apps: [], subscriptions: [account("work", "signed_in", "Work"), account("out", "signed_out", "Not signed in")], tools: [], sessions: [], thisMac: "this-mac", macName: (id: string) => id } as never);
    expect(rows.map((row) => row.label)).toEqual(["Work"]);
  });
});
