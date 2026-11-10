/**
 * The P3 walk: a signed-out group of folders ("Not signed in") was offered
 * as an Account member, with no Account behind it to add. The P6 walk: a
 * paired Mac with no sessions here was never offered as a Mac member.
 */
import { describe, expect, it } from "vitest";
import type { AppModel } from "./apps";
import { broadCandidates, memberLabel } from "./new-pool";

const account = (key: string, state: string, display_name: string, account_id: string | null, folders: string[] = []) =>
  ({ key, state, display_name, account_id, profiles: folders.map((id) => ({ id })) }) as never;

describe("Account members to add", () => {
  const rows = (sessions: unknown[] = []) =>
    broadCandidates({
      kind: "account",
      apps: [],
      subscriptions: [account("k1", "signed_in", "Work", "acct:claude:aa", ["claude-work", "claude-work-2"]), account("k2", "unknown", "Was Pro", "acct:claude:bb"), account("k3", "unknown", "Can't read", null), account("k4", "signed_out", "Not signed in", null)],
      tools: [],
      sessions,
      thisMac: "this-mac",
      paired: [],
    } as never);

  it("offers the stored Accounts, signed in or not readable now, never a signed-out group or one never read", () => {
    expect(rows().map((row) => row.label)).toEqual(["Work", "Was Pro"]);
  });

  // The pools L2 on the replica: New pool sent the list's key, and the
  // member it made matched no session.
  it("adds the stored Account, never the list's key", () => {
    expect(rows().map((row) => row.ref)).toEqual(["account:acct:claude:aa", "account:acct:claude:bb"]);
  });

  it("counts what's open in every App signed in to it", () => {
    const session = (id: string, account_id: string, status = "running") => ({ id, account_id, status, machine_id: "this-mac" });
    const [work] = rows([session("s1", "claude-work"), session("s2", "claude-work-2"), session("s3", "claude-other"), session("s4", "claude-work", "finished")]);
    expect(work!.detail).toBe("2 open now");
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
    expect(rows[1]!.blocked).toMatch(/once pools work across Macs/);
    expect(rows[1]!.detail).toBe("paired with this Mac");
  });

  it("names a Mac it only knows from a session, without a pairing, plainly", () => {
    const rows = broadCandidates({ kind: "mac", apps: [], subscriptions: [], tools: [], sessions: [session("s1", "this-mac"), session("s2", "other")], thisMac: "this-mac", paired: [] } as never);
    expect(rows.map((row) => row.label)).toEqual(["This Mac", "A paired Mac"]);
  });
});

// The rc26 P5 walk: an Account member read only "An Account", because
// its name was looked up by the list's key, never the stored Account.
describe("a member's name", () => {
  const input = {
    apps: [{ id: "cli:claude-work", name: "Claude Code · Work · work" }],
    subscriptions: [{ key: "9f1c2a", account_id: "acct:claude:aa", display_name: "Work" }],
    tools: [{ id: "codex", display_name: "Codex" }],
    thisMac: "this-mac",
    paired: [],
  };

  it("names an Account member by its stored Account", () => {
    expect(memberLabel("account:acct:claude:aa", input)).toBe("Work");
  });

  it("says Account unknown for one no App here reads now, never a bare 'An Account'", () => {
    expect(memberLabel("account:acct:claude:gone", input)).toBe("Account unknown");
    expect(memberLabel("account:9f1c2a", input)).toBe("Account unknown");
  });

  it("names the other kinds as before", () => {
    expect(memberLabel("app:cli:claude-work", input)).toBe("Claude Code · Work · work");
    expect(memberLabel("tool:codex", input)).toBe("Codex");
    expect(memberLabel("mac:this-mac", input)).toBe("This Mac");
    expect(memberLabel("all", input)).toBe("Everyone in this pool's projects");
  });
});

// The rc26 P1 walk (step 8): the Claude app read "none open now · new
// ones join" beside Add, as if its sessions would act. A desktop App may
// join (D-62 decision 15: offered, never greyed) and says its sessions
// are watched.
describe("App members to add", () => {
  const appModel = (id: string, kind: AppModel["kind"], app: string, name: string): AppModel => ({ id, kind, tool: null, appId: null, app, profile: null, name, defaultName: name, named: false, railKey: id, folder: null, accountId: null, place: null, slot: 0 });
  const apps = [appModel("cli:claude-work", "cli", "Claude Code", "Claude Code · Work · work"), appModel("app:claude:work-max", "desktop", "Claude app", "Claude app · Work Max")];
  const rows = () => broadCandidates({ kind: "app", apps, subscriptions: [], tools: [], sessions: [], thisMac: "this-mac", paired: [] });

  it("offers a desktop App to add, saying its sessions join watched", () => {
    const desktop = rows().find((row) => row.label === "Claude app · Work Max")!;
    expect(desktop.blocked).toBeNull();
    expect(desktop.note).toBe("Runs in the Claude app: its sessions join watched (they can't reply) until continued in AI Juggler.");
  });

  it("says nothing more for an App whose sessions AI Juggler starts", () => {
    expect(rows().find((row) => row.label === "Claude Code · Work · work")!.note).toBeUndefined();
  });
});
