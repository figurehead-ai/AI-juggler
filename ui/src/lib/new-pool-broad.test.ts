/**
 * The P3 walk: a signed-out group of folders ("Not signed in") was offered
 * as an Account member, with no Account behind it to add. The P6 walk: a
 * paired Mac with no sessions here was never offered as a Mac member.
 */
import { describe, expect, it } from "vitest";
import { broadCandidates } from "./new-pool";

const account = (key: string, state: string, display_name: string) => ({ key, state, display_name }) as never;

describe("Account members to add", () => {
  it("offers the signed-in and the unknown Accounts, never a signed-out group", () => {
    const rows = broadCandidates({ kind: "account", apps: [], subscriptions: [account("work", "signed_in", "Work"), account("out", "signed_out", "Not signed in")], tools: [], sessions: [], thisMac: "this-mac", paired: [] } as never);
    expect(rows.map((row) => row.label)).toEqual(["Work"]);
  });
});

describe("Mac members to add", () => {
  const session = (id: string, machine_id: string) => ({ id, machine_id, status: "running" }) as never;

  it("offers this Mac and every paired Mac by its name; a paired one says why it can't join and has nothing to add", () => {
    const rows = broadCandidates({ kind: "mac", apps: [], subscriptions: [], tools: [], sessions: [session("s1", "this-mac")], thisMac: "this-mac", paired: [{ machine_id: "mini", label: "Build mini" }] } as never);
    expect(rows.map((row) => [row.label, row.blocked === null])).toEqual([
      ["This Mac", true],
      ["Build mini", false],
    ]);
    expect(rows[1].blocked).toMatch(/once pools work across Macs/);
    expect(rows[1].detail).toBe("paired with this Mac");
  });

  it("names a Mac it only knows from a session, without a pairing, plainly", () => {
    const rows = broadCandidates({ kind: "mac", apps: [], subscriptions: [], tools: [], sessions: [session("s1", "this-mac"), session("s2", "other")], thisMac: "this-mac", paired: [] } as never);
    expect(rows.map((row) => row.label)).toEqual(["This Mac", "A paired Mac"]);
  });
});
