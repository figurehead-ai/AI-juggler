/**
 * The rail matrix, Account lens: the same App states, grouped by Account.
 *
 * Spec: ADR-0032 §2 ("each lens groups strictly by its own entity …
 * Account by Account"); D-64 (Inactive apps and an App's last inactive
 * sessions are the App grouping's: by Account, inactive and ended ones
 * stay behind the Also line, D-36 and E21 AJ-E2103 req. 1); amendment 2
 * (pins never mean open); UX spec §5 rule 6 (an Account out of room
 * carries its mark wherever it appears). The world: East, West and North
 * are profiles on Account "Team"; South and the CLI App on "Solo"
 * (`ACCOUNT_OF` in the kit).
 */
import { describe, it } from "vitest";
import { check, claudeApp, cliApp, open, past, H, NOW, type MatrixRow } from "./rail-matrix.test-kit";

const EAST = claudeApp("East", 41);
const WEST = claudeApp("West", 42);
const NORTH = claudeApp("North", 43);
const SOUTH = claudeApp("South", null);
const CLI = cliApp("claude-code", "f-cli", "Side");
const TEAM = "acct-team";
const SOLO = "acct-solo";
const STRAY = { id: "app:claude-app:/p/Stray", label: "Claude app · Stray", running: true };

/** Open in East, West and the CLI App; past in East (ended today) and South (yesterday); North runs empty. */
const WORLD = [
  open("o1", EAST, { title: "Fix login", started_ms: NOW - 3 * H }),
  open("w1", WEST, { title: "Docs pass", started_ms: NOW - 2 * H }),
  open("c1", CLI, { title: "Fix cli" }),
  past("p1", EAST, 1, { title: "Fix tests" }),
  past("s1", SOUTH, 20, { title: "Fix deploy" }),
];

const ROWS: MatrixRow[] = [
  {
    // ASSUMED: by Account, Open now's head counts the sessions open (the spec says only that by App it holds the running Apps).
    // ASSUMED: no Running, not added, and no Inactive apps by Account (ADR-0032 amendment 1 and D-64 are about Apps).
    name: "A1 no search: one group per Account with something open, each holding only its Account's open rows",
    lens: "account", apps: [EAST, WEST, NORTH, SOUTH, CLI], talks: WORLD, offers: [STRAY],
    expect: {
      open: [TEAM, SOLO], idle: [], offered: [], counts: { open: 3, idle: null, offered: null },
      under: { [TEAM]: ["o1", "w1"], [SOLO]: ["c1"] }, links: { [TEAM]: [], [SOLO]: [] }, foldable: { [TEAM]: true, [SOLO]: true },
      also: { ended: 1, inactive: 0 },
    },
  },
  {
    // ASSUMED: the past match waits behind the Also line, counted there (D-36: never silence), as by Account it does unsearched.
    name: "A2 search: each open match under its own Account; the past match counted on the Also line",
    lens: "account", query: "fix", apps: [EAST, WEST, NORTH, SOUTH, CLI], talks: WORLD,
    expect: { open: [TEAM, SOLO], counts: { open: 2 }, under: { [TEAM]: ["o1"], [SOLO]: ["c1"] }, also: { ended: 1 } },
  },
  {
    name: "A3 Only account Team: Team's open rows, from both of its profiles; nothing of Solo's",
    lens: "account", only: { account: "Team" }, apps: [EAST, WEST, NORTH, SOUTH, CLI], talks: WORLD,
    expect: { open: [TEAM], counts: { open: 2 }, under: { [TEAM]: ["o1", "w1"] }, also: { ended: 1 } },
  },
  {
    name: "A4 an ended pinned session never opens its Account (pins never mean open)",
    lens: "account", apps: [EAST], talks: [past("p1", EAST, 1)], pins: ["p1"],
    expect: { open: [], counts: { open: null } },
  },
  {
    name: "A5 archived and unlisted past sessions: not counted on the Also line",
    lens: "account", apps: [EAST], talks: [open("o1", EAST), past("p1", EAST, 1, { app_archived: true }), past("p2", EAST, 2, { archived_here: true }), past("p3", EAST, 3, { app_unlisted: true })],
    expect: { open: [TEAM], under: { [TEAM]: ["o1"] }, also: { ended: 0 } },
  },
  {
    // RULED as App lens 7g (0.4.3): what runs shows, whatever its archive marks.
    name: "A6 an open session archived in its app or here still shows under its Account: it runs",
    lens: "account", apps: [EAST], talks: [open("o1", EAST), open("o2", EAST, { app_archived: true }), open("o3", EAST, { archived_here: true })],
    expect: { open: [TEAM], counts: { open: 3 }, under: { [TEAM]: ["o1", "o2", "o3"] } },
  },
  {
    name: "A7 folded by the user: the Account's header only",
    lens: "account", apps: [EAST, CLI], talks: [open("o1", EAST), open("c1", CLI)], order: { folded: [`account:${TEAM}`] },
    expect: { open: [TEAM, SOLO], folded: [TEAM], under: { [TEAM]: [], [SOLO]: ["c1"] } },
  },
];

describe("rail matrix: Account lens", () => {
  it.each(ROWS.filter((row) => !row.mismatch))("$name", (row) => check(row));
  it.fails.each(ROWS.filter((row) => row.mismatch))("$name (MISMATCH)", (row) => check(row));
});
