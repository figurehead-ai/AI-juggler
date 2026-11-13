/**
 * The rail matrix, App lens (2 of 2): pins, archived and unlisted
 * sessions, families, folds and the three orders, then a search and a
 * filter (the View's Only) over the App states.
 *
 * Spec: ADR-0032 amendment 2 (pins never mean open; a pinned ended session
 * shows pinned under its App wherever the App is), amendment 8 (an App's
 * sessions are its records, archived as they say; `app_unlisted` is in no
 * App's list or count; folded, nothing under it), §2 (a family nests only
 * among one App's own sessions, rc4's search rule), E21 AJ-E2104 req. 4
 * with D-47 (Latest activity folds a quiet place; Your order and Name fold
 * only when the user does; pinned rows lead in every order), and the rail
 * search (rc4 walk: every match, under its own App).
 */
import { describe, it } from "vitest";
import { check, claudeApp, cliApp, H, NOW, open, past, pasts, type MatrixRow } from "./rail-matrix.test-kit";

const EAST = claudeApp("East", 41);
const WEST = claudeApp("West", 42);
const NORTH = claudeApp("North", 43);
const SOUTH = claudeApp("South", null);
const CLI = cliApp("claude-code", "f-cli", "Side");
const EAST_CLI = cliApp("claude-code", "f-east", "East");
const recent = (app: { key: string }) => `${app.key}#recent`;
const fold = (app: { key: string }) => `place:${app.key}`;

/** Four open rows in East (one pinned, one working) and West with one past one, for the orders. */
const ORDERED = [
  open("z", EAST, { title: "Zeta", started_ms: NOW - 4 * H, last_activity_ms: NOW - 60_000 }),
  open("a", EAST, { title: "Alpha", started_ms: NOW - 3 * H, last_activity_ms: NOW - 5 * 60_000 }),
  open("m", EAST, { title: "Mid", started_ms: NOW - 2 * H, last_activity_ms: NOW - 3 * 60_000, state: "working" }),
  open("q", EAST, { title: "Quill", started_ms: NOW - H, last_activity_ms: NOW - 10 * 60_000 }),
  past("w1", WEST, 2),
];

const ROWS: MatrixRow[] = [
  // 7. Pinned, archived, unlisted.
  {
    name: "7a pinned ended session of a closed App: the App stays Inactive, the pin shows under it unclicked (no Recent over a pin)",
    lens: "place", apps: [SOUTH], talks: pasts("s", SOUTH, 3), pins: ["s2"],
    expect: { open: [], idle: [SOUTH.key], counts: { open: null, idle: 1 }, under: { [SOUTH.key]: ["s2"] }, recent: [] },
  },
  {
    name: "7b pinned ended session of a running App with one open: open row, the pin, then '+2 recent' on its header",
    lens: "place", apps: [EAST], talks: [open("o1", EAST), ...pasts("p", EAST, 3)], pins: ["p2"],
    expect: { open: [EAST.key], under: { [EAST.key]: ["o1", "p2"] }, links: { [EAST.key]: ["+2 recent"] } },
  },
  {
    name: "7c … Last 2 clicked: open, pinned, then the rest newest first, no Show all",
    lens: "place", apps: [EAST], talks: [open("o1", EAST), ...pasts("p", EAST, 3)], pins: ["p2"], toggled: [recent(EAST)],
    expect: { under: { [EAST.key]: ["o1", "p2", "p1", "p3"] }, links: { [EAST.key]: ["+2 recent"] } },
  },
  {
    name: "7d pinned past session of a running App with nothing open: the pin first, then Recent over its last 3, then Show all",
    lens: "place", apps: [WEST], talks: pasts("w", WEST, 5), pins: ["w4"],
    expect: { open: [WEST.key], under: { [WEST.key]: ["w4", "w1", "w2", "w3"] }, links: { [WEST.key]: ["Show all"] }, recent: [WEST.key] },
  },
  {
    name: "7e archived (in its app, or here) and unlisted past sessions: never listed, never counted",
    lens: "place", apps: [EAST, SOUTH],
    talks: [past("p1", EAST, 1, { app_archived: true }), past("p2", EAST, 2, { archived_here: true }), past("p3", EAST, 3, { app_unlisted: true }), past("p4", EAST, 4), past("s1", SOUTH, 1, { app_archived: true })],
    toggled: [recent(SOUTH)],
    expect: { open: [EAST.key], idle: [SOUTH.key], counts: { open: 1, idle: 1 }, under: { [EAST.key]: ["p4"], [SOUTH.key]: [] }, links: { [EAST.key]: [] } },
  },
  {
    name: "7f a running App whose only past sessions are unlisted or archived: nothing to unfold, no fold",
    lens: "place", apps: [WEST], talks: [past("w1", WEST, 1, { app_unlisted: true }), past("w2", WEST, 2, { app_archived: true })],
    expect: { open: [WEST.key], under: { [WEST.key]: [] }, foldable: { [WEST.key]: false } },
  },
  {
    // RULED (0.4.3): Open now is everything that runs (D-100), so a session that is open shows whatever its archive marks say;
    // archived and unlisted ones are left out only once ended (7e).
    name: "7g an open session archived in its app or here still shows: it runs",
    lens: "place", apps: [EAST], talks: [open("o1", EAST, { app_archived: true }), open("o2", EAST, { archived_here: true }), open("o3", EAST)],
    expect: { open: [EAST.key], under: { [EAST.key]: ["o1", "o2", "o3"] } },
  },
  // 8. Families.
  {
    name: "8a open sessions started from one another in one App: nested under each other",
    lens: "place", apps: [EAST],
    talks: [open("o1", EAST, { started_ms: NOW - 3 * H }), open("o2", EAST, { started_ms: NOW - 2 * H, started_from: "o1" }), open("o3", EAST, { started_ms: NOW - H, started_from: "o2" })],
    expect: { under: { [EAST.key]: ["o1", "o2", "o3"] }, depth: { o1: 0, o2: 1, o3: 2 } },
  },
  {
    name: "8b past ones of a running App, nothing open: each under the one it was started from, siblings newest first",
    lens: "place", apps: [WEST], talks: [past("q1", WEST, 5), past("q2", WEST, 4, { started_from: "q1" }), past("q3", WEST, 3, { started_from: "q1" })],
    expect: { under: { [WEST.key]: ["q1", "q3", "q2"] }, depth: { q1: 0, q3: 1, q2: 1 } },
  },
  {
    name: "8c started from a session of another App: under its own App, at the top level (never across Apps)",
    lens: "place", apps: [EAST, EAST_CLI], talks: [open("o1", EAST), open("c1", EAST_CLI, { started_from: "o1" })],
    expect: { open: [EAST.key, EAST_CLI.key], under: { [EAST.key]: ["o1"], [EAST_CLI.key]: ["c1"] }, depth: { c1: 0 } },
  },
  {
    name: "8d a subsession the backend gives its root's App, though it runs in a terminal: under that App, nested (ADR-0032 §1)",
    lens: "place", apps: [EAST, EAST_CLI], talks: [open("o1", EAST, { started_ms: NOW - 2 * H }), open("o2", EAST, { started_ms: NOW - H, started_from: "o1", places: [{ ...EAST_CLI.place, pid: 501 }] })],
    expect: { open: [EAST.key], under: { [EAST.key]: ["o1", "o2"] }, depth: { o2: 1 } },
  },
  // 9. Folds and orders.
  {
    name: "9a folded by the user: a foldable App shows nothing under it; one with nothing to show has no fold",
    lens: "place", apps: [EAST, NORTH, WEST], talks: [open("o1", EAST), past("p1", EAST, 1), past("w1", WEST, 1)], pins: ["p1"],
    order: { folded: [fold(EAST), fold(NORTH)] },
    expect: {
      open: [EAST.key, NORTH.key, WEST.key], folded: [EAST.key],
      under: { [EAST.key]: [], [NORTH.key]: [], [WEST.key]: ["w1"] }, links: { [EAST.key]: [] }, foldable: { [EAST.key]: true, [NORTH.key]: false, [WEST.key]: true },
    },
  },
  {
    name: "9b Your order: pinned first, then by when each started; nothing folds by itself",
    lens: "place", apps: [EAST, WEST, NORTH], talks: ORDERED, pins: ["q"], order: { mode: "yours" },
    expect: { open: [EAST.key, WEST.key, NORTH.key], folded: [], under: { [EAST.key]: ["q", "z", "a", "m"], [WEST.key]: ["w1"], [NORTH.key]: [] } },
  },
  {
    name: "9c Name: pinned first, then A to Z; nothing folds by itself",
    lens: "place", apps: [EAST, WEST, NORTH], talks: ORDERED, pins: ["q"], order: { mode: "name" },
    expect: { open: [EAST.key, WEST.key, NORTH.key], folded: [], under: { [EAST.key]: ["q", "a", "m", "z"], [WEST.key]: ["w1"], [NORTH.key]: [] } },
  },
  {
    name: "9d Latest activity: pinned, working, then newest; a quiet App folds to its header, nothing under it",
    lens: "place", apps: [EAST, WEST, NORTH], talks: ORDERED, pins: ["q"], order: { mode: "latest" },
    expect: { open: [EAST.key, WEST.key, NORTH.key], folded: [WEST.key], under: { [EAST.key]: ["q", "m", "z", "a"], [WEST.key]: [], [NORTH.key]: [] }, foldable: { [NORTH.key]: false } },
  },
  // Search: every match under its own App (rc4); an App with none isn't listed (ASSUMED).
  {
    name: "S1 search: an open App's open and past matches, a closed App's past match, each under its own App",
    lens: "place", query: "fix", apps: [EAST, SOUTH, NORTH, CLI],
    talks: [
      open("o1", EAST, { title: "Fix login" }), open("o2", EAST, { title: "Docs pass" }), past("p1", EAST, 1, { title: "Fix tests" }), past("p2", EAST, 2, { title: "Review" }),
      past("s1", SOUTH, 1, { title: "Fix deploy" }), past("s2", SOUTH, 2, { title: "Notes" }), past("c1", CLI, 1, { title: "Cleanup" }),
    ],
    expect: { open: [EAST.key], idle: [SOUTH.key], counts: { open: 1, idle: 1 }, under: { [EAST.key]: ["o1", "p1"], [SOUTH.key]: ["s1"] }, links: { [EAST.key]: [], [SOUTH.key]: [] } },
  },
  {
    name: "S2 search: a CLI App's open and past matches, all of them (no Last 3 while searching)",
    lens: "place", query: "fix", apps: [CLI], talks: [open("c0", CLI, { title: "Fix cli" }), ...[1, 2, 3, 4].map((n) => past(`c${n}`, CLI, n, { title: `Fix part ${n}` }))],
    expect: { open: [CLI.key], under: { [CLI.key]: ["c0", "c1", "c2", "c3", "c4"] }, links: { [CLI.key]: [] } },
  },
  {
    name: "S3 search: archived and unlisted matches are never listed; no App holds a match, so no section",
    lens: "place", query: "fix", apps: [EAST, SOUTH], talks: [past("p1", EAST, 1, { title: "Fix old", app_archived: true }), past("s1", SOUTH, 1, { title: "Fix gone", app_unlisted: true })],
    expect: { open: [], idle: [], counts: { open: null, idle: null } },
  },
  {
    // FIXED (0.4.3): a running App with a past match joins Open now while searching.
    name: "S4 search: a running App with nothing open, its past session matches: in Open now, the match under it (a match, not Recent)",
    lens: "place", query: "fix", apps: [WEST], talks: [past("w1", WEST, 1, { title: "Fix build" }), past("w2", WEST, 2, { title: "Notes" })],
    expect: { open: [WEST.key], counts: { open: 1 }, under: { [WEST.key]: ["w1"] }, recent: [] },
  },
  {
    // FIXED (0.4.3), as S4.
    name: "S5 search: a running App whose open session doesn't match but a past one does: the match under it",
    lens: "place", query: "fix", apps: [EAST], talks: [open("o1", EAST, { title: "Docs pass" }), past("p1", EAST, 1, { title: "Fix tests" })],
    expect: { open: [EAST.key], under: { [EAST.key]: ["p1"] } },
  },
  // Filter (the View's Only): the Apps with a match, each match under its own App (ASSUMED, as a search).
  {
    name: "F1 Only project api: the open App's api row and its api past one behind '+1 recent'; the closed App's api one; no others",
    lens: "place", only: { project: "api" }, apps: [EAST, SOUTH, NORTH],
    talks: [
      open("o1", EAST, { project_id: "p-api" }), open("o2", EAST, { project_id: "p-web" }), past("p1", EAST, 1, { project_id: "p-api" }), past("p2", EAST, 2, { project_id: "p-web" }),
      past("s1", SOUTH, 1, { project_id: "p-api" }), past("s2", SOUTH, 2, { project_id: "p-web" }),
    ],
    toggled: [recent(SOUTH)],
    expect: { open: [EAST.key], idle: [SOUTH.key], counts: { open: 1, idle: 1 }, under: { [EAST.key]: ["o1"], [SOUTH.key]: ["s1"] }, links: { [EAST.key]: ["+1 recent"] } },
  },
  {
    // ASSUMED: Only › App is one App (D-62), whose sessions are the ones the backend names it owner of (ADR-0032 §1).
    // FIXED (0.4.3): items are their owner App's before Only › App reads them.
    name: "F2 Only app East (open + 2 past): its open row and '+2 recent'",
    lens: "place", only: { app: "Claude app · East" }, apps: [EAST, WEST], talks: [open("o1", EAST), ...pasts("p", EAST, 2), open("w1", WEST)],
    expect: { open: [EAST.key], counts: { open: 1 }, under: { [EAST.key]: ["o1"] }, links: { [EAST.key]: ["+2 recent"] } },
  },
  {
    // ASSUMED as F2; FIXED (0.4.3).
    name: "F3 Only app West (running, only past): in Open now, its past ones at once",
    lens: "place", only: { app: "Claude app · West" }, apps: [EAST, WEST], talks: [open("o1", EAST), ...pasts("w", WEST, 2)],
    expect: { open: [WEST.key], counts: { open: 1 }, under: { [WEST.key]: ["w1", "w2"] } },
  },
];

describe("rail matrix: App lens, pins, families, folds, orders, search and filter", () => {
  it.each(ROWS.filter((row) => !row.mismatch))("$name", (row) => check(row));
  it.fails.each(ROWS.filter((row) => row.mismatch))("$name (MISMATCH)", (row) => check(row));
});
