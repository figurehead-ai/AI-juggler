/**
 * D-103.7 (persona R2, P4: a client's repo opened on the agency's
 * Account): a repo's own Account is New session's default. The one bound
 * to it ("Always use for …"), else the only Account that ever ran there;
 * "the last you used here" only when there is neither. Pinned for each
 * case: bound; only one ran there; several; none. And which Accounts ran
 * there: the backend's count by Account, else what the app holds.
 */
import { describe, expect, it } from "vitest";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { appWhyWords, chooseApp, type Room } from "./new-session-choice";
import { accountsRanHere, type UsedSession } from "./new-session-history";

const recipe = (folder: string, label: string): LaunchRecipeInfo => ({ id: `r-${folder}`, name: "Main", account_id: folder, account_label: label, provider: "tool-a", model: null, default_cwd: null, restart_on_crash: null });
const folder = (id: string, extra: Partial<AccountInfo> = {}): AccountInfo => ({ id, provider: "tool-a", label: id, plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false, ...extra });
const project = (id: string, extra: Partial<ProjectInfo> = {}): ProjectInfo => ({
  id,
  root: `/Users/you/Dev/${id}`,
  name: id,
  last_activity_ms: null,
  active_instance: null,
  accounts_used: [],
  instance_count: 0,
  history_sessions: 0,
  history_last_used_ms: null,
  repo: null,
  missing: false,
  ...extra,
});

// Three Accounts: the agency's, and two clients'; Client A has two Apps.
const AGENCY = recipe("agency", "agency");
const CLIENT_A = recipe("client-a", "client-a");
const CLIENT_A_2 = recipe("client-a-2", "client-a-2");
const CLIENT_B = recipe("client-b", "client-b");
const RECIPES = [AGENCY, CLIENT_A, CLIENT_A_2, CLIENT_B];
const ACCOUNT: Record<string, string> = { agency: "acct-agency", "client-a": "acct-a", "client-a-2": "acct-a", "client-b": "acct-b" };
const accountOf = (id: string) => ACCOUNT[id] ?? id;
const room = (left: Record<string, number | null> = {}) => (id: string): Room => ({ left: left[id] ?? 60, resetsAtMs: null, credits: false });

// The agency's App ran in client-a last: the R2 walk's default.
const REPO = project("client-a", { accounts_used: ["agency", "client-a"] });
const SESSIONS: UsedSession[] = [
  { account_id: "client-a", project_id: "client-a", started_at_ms: 1 },
  { account_id: "agency", project_id: "client-a", started_at_ms: 2 },
];
const base = { recipes: RECIPES, folders: RECIPES.map((row) => folder(row.account_id)), roomOf: room(), accountOf, sessions: SESSIONS };

describe("New session's default for a repo (D-103.7)", () => {
  it("bound: the Account bound to it, whatever ran there", () => {
    const choice = chooseApp({ ...base, project: { ...REPO, default_account_id: "client-a" }, ranHere: ["acct-a", "acct-agency"] })!;
    expect([choice.recipe, appWhyWords(choice.why)]).toEqual([CLIENT_A, "your default here"]);
  });

  it("only one ever ran there: that Account, over the last used here and the user's own default", () => {
    const yours = RECIPES.map((row) => folder(row.account_id, { is_default: row === AGENCY }));
    const sessions: UsedSession[] = [{ account_id: "client-b", project_id: "client-b", started_at_ms: 9 }];
    const choice = chooseApp({ ...base, folders: yours, sessions, project: project("client-b", { accounts_used: ["client-b"] }), ranHere: ["acct-b"] })!;
    expect([choice.recipe, appWhyWords(choice.why)]).toEqual([CLIENT_B, "the only account used here"]);
  });

  it("only one ever ran there, with two Apps: the one used there last", () => {
    const sessions: UsedSession[] = [
      { account_id: "client-a", project_id: "client-a", started_at_ms: 1 },
      { account_id: "client-a-2", project_id: "client-a", started_at_ms: 2 },
    ];
    expect(chooseApp({ ...base, sessions, project: REPO, ranHere: ["acct-a"] })).toMatchObject({ recipe: CLIENT_A_2, why: "own-account" });
  });

  it("only one ever ran there: it stands out of room or signed out, warned, another offered; never switched", () => {
    const out = chooseApp({ ...base, roomOf: room({ "client-a": 0, "client-a-2": 0 }), project: REPO, ranHere: ["acct-a"] })!;
    expect(out).toMatchObject({ recipe: CLIENT_A, why: "own-account", kind: "out", skipped: null });
    expect(accountOf(out.roomier!.account_id)).not.toBe("acct-a");
    const folders = RECIPES.map((row) => folder(row.account_id, { login_state: row.account_id.startsWith("client-a") ? "logged_out" : "logged_in" }));
    expect(chooseApp({ ...base, folders, project: REPO, ranHere: ["acct-a"] })).toMatchObject({ why: "own-account", kind: "blocked" });
  });

  it("several ran there: the last you used here, as before", () => {
    const choice = chooseApp({ ...base, project: REPO, ranHere: ["acct-a", "acct-agency"] })!;
    expect([choice.recipe, appWhyWords(choice.why)]).toEqual([AGENCY, "the last you used here"]);
  });

  it("none ran there: the last you used anywhere; nothing claimed for the repo", () => {
    const choice = chooseApp({ ...base, project: project("new-repo"), ranHere: [] })!;
    expect([choice.recipe, appWhyWords(choice.why)]).toEqual([AGENCY, "the last you used"]);
  });

  it("an Account with no App to start it is no default: the next reason", () => {
    expect(chooseApp({ ...base, project: REPO, ranHere: ["acct-c"] })).toMatchObject({ recipe: AGENCY, why: "last-here" });
  });

  it("the user's pick still comes first", () => {
    expect(chooseApp({ ...base, project: REPO, ranHere: ["acct-a"], chosen: "agency" })).toMatchObject({ recipe: AGENCY, why: "chosen" });
  });
});

describe("which Accounts ran there", () => {
  const counted = (byAccount: [string, number][], total: number) => ({ byAccount: new Map(byAccount), total });

  it("the backend's count by Account, every session there", () => {
    expect(accountsRanHere({ project: REPO, sessions: [], recipes: RECIPES, accountOf, counted: counted([["acct-b", 14]], 14) })).toEqual(["acct-b"]);
  });

  it("sessions on no known Account make one more: never 'the only one' on a guess", () => {
    expect(accountsRanHere({ project: REPO, sessions: [], recipes: RECIPES, accountOf, counted: counted([["acct-b", 12]], 14) })).toHaveLength(2);
  });

  it("until it answers, what the app holds: its sessions there and the Apps the history names", () => {
    expect(accountsRanHere({ project: REPO, sessions: SESSIONS, recipes: RECIPES, accountOf }).sort()).toEqual(["acct-a", "acct-agency"]);
    expect(accountsRanHere({ project: project("p", { accounts_used: ["client-a", "client-a-2"] }), sessions: [], recipes: RECIPES, accountOf })).toEqual(["acct-a"]);
    // A name it can't place is another Account.
    expect(accountsRanHere({ project: project("p", { accounts_used: ["client-a", "gone"] }), sessions: [], recipes: RECIPES, accountOf })).toHaveLength(2);
    expect(accountsRanHere({ project: null, sessions: SESSIONS, recipes: RECIPES, accountOf })).toEqual([]);
  });
});
