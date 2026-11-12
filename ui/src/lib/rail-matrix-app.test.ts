/**
 * The rail matrix, App lens (1 of 2): every App state, no search.
 *
 * Spec: D-64 (an App's last inactive sessions under it, "Last 3 inactive";
 * Inactive apps), D-100 and ADR-0032 (Open now = the running Apps, sessions
 * or not; Running, not added; Inactive = the user's Apps not running; each
 * App in exactly one; a row under the App the backend names), amendment 8
 * (a running App unfolds to its open, pinned, then last ones, at once when
 * nothing is open in it, then "Show all"; a fold only when unfolding shows
 * a row; the Out mark is the App's own folder). Checks shared by every row
 * are in `rail-matrix.test-kit.ts` (`invariants`).
 */
import { describe, it } from "vitest";
import { check, claudeApp, cliApp, codexApp, open, past, pasts, type MatrixRow } from "./rail-matrix.test-kit";

const EAST = claudeApp("East", 41);
const WEST = claudeApp("West", 42);
const NORTH = claudeApp("North", 43);
const SOUTH = claudeApp("South", null);
const GONE = claudeApp("Gone", null);
const CLI = cliApp("claude-code", "f-cli", "Side");
const CODEX = codexApp(77);
const CODEX_OFF = codexApp(null);
const CODEX_CLI = cliApp("codex", "f-codex", "Codex home");
const recent = (app: { key: string }) => `${app.key}#recent`;

const ROWS: MatrixRow[] = [
  // 1. Running, with open sessions and past ones.
  {
    name: "1a running, 1 open + 5 past: its open row, then Last 3 inactive folded (D-64)",
    lens: "place", apps: [EAST], talks: [open("o1", EAST), ...pasts("p", EAST, 5)],
    expect: {
      open: [EAST.key], idle: [], counts: { open: 1, idle: null, offered: null },
      under: { [EAST.key]: ["o1"] }, links: { [EAST.key]: ["Last 3 inactive"] }, foldable: { [EAST.key]: true }, folder: { [EAST.key]: "f-east" },
    },
  },
  {
    name: "1b running, 1 open + 5 past, Last 3 clicked: the 3 newest, then Show all (amendment 8)",
    lens: "place", apps: [EAST], talks: [open("o1", EAST), ...pasts("p", EAST, 5)], toggled: [recent(EAST)],
    expect: { open: [EAST.key], under: { [EAST.key]: ["o1", "p1", "p2", "p3"] }, links: { [EAST.key]: ["Last 3 inactive", "Show all"] } },
  },
  {
    // ASSUMED: "Show all" only when the App has more than the 3 shown (D-64: the full list is on its page).
    name: "1c running, 1 open + 2 past, clicked: both, no Show all",
    lens: "place", apps: [EAST], talks: [open("o1", EAST), ...pasts("p", EAST, 2)], toggled: [recent(EAST)],
    expect: { open: [EAST.key], under: { [EAST.key]: ["o1", "p1", "p2"] }, links: { [EAST.key]: ["Last 2 inactive"] } },
  },
  // 2. Running, only past sessions.
  {
    name: "2a running, nothing open, 5 past: in Open now, its last 3 at once, then Show all",
    lens: "place", apps: [WEST], talks: pasts("w", WEST, 5),
    expect: {
      open: [WEST.key], idle: [], counts: { open: 1, idle: null },
      under: { [WEST.key]: ["w1", "w2", "w3"] }, links: { [WEST.key]: ["Show all"] }, foldable: { [WEST.key]: true }, folder: { [WEST.key]: "f-west" },
    },
  },
  {
    name: "2b running, nothing open, 2 past: both at once, no Show all (ASSUMED as 1c)",
    lens: "place", apps: [WEST], talks: pasts("w", WEST, 2),
    expect: { open: [WEST.key], under: { [WEST.key]: ["w1", "w2"] }, links: { [WEST.key]: [] }, foldable: { [WEST.key]: true } },
  },
  // 3. Running, no sessions at all.
  {
    name: "3 running, no sessions: in Open now (never Inactive), no fold, nothing under it, its own Out mark",
    lens: "place", apps: [NORTH], talks: [],
    expect: { open: [NORTH.key], idle: [], counts: { open: 1, idle: null }, under: { [NORTH.key]: [] }, links: { [NORTH.key]: [] }, foldable: { [NORTH.key]: false }, folder: { [NORTH.key]: "f-north" } },
  },
  // 4. Closed desktop Apps.
  {
    name: "4a closed, 5 past: in Inactive apps, nothing under it until clicked",
    lens: "place", apps: [SOUTH], talks: pasts("s", SOUTH, 5),
    expect: { open: [], idle: [SOUTH.key], counts: { open: null, idle: 1 }, under: { [SOUTH.key]: [] }, links: { [SOUTH.key]: [] } },
  },
  {
    name: "4b closed, 5 past, clicked: its 3 newest, then Show all",
    lens: "place", apps: [SOUTH], talks: pasts("s", SOUTH, 5), toggled: [recent(SOUTH)],
    expect: { idle: [SOUTH.key], under: { [SOUTH.key]: ["s1", "s2", "s3"] }, links: { [SOUTH.key]: ["Show all"] } },
  },
  {
    name: "4c closed, 2 past, clicked: both, no Show all (ASSUMED as 1c)",
    lens: "place", apps: [SOUTH], talks: pasts("s", SOUTH, 2), toggled: [recent(SOUTH)],
    expect: { idle: [SOUTH.key], under: { [SOUTH.key]: ["s1", "s2"] }, links: { [SOUTH.key]: [] } },
  },
  {
    name: "4d closed, no sessions: still in Inactive apps (it never vanishes, D-64), nothing under it",
    lens: "place", apps: [SOUTH, GONE], talks: [], toggled: [recent(SOUTH)],
    expect: { open: [], idle: [SOUTH.key, GONE.key], counts: { open: null, idle: 2 }, under: { [SOUTH.key]: [], [GONE.key]: [] } },
  },
  // 5. CLI-only Apps.
  {
    name: "5a CLI App with an open session: in Open now, its open row, Last 2 inactive, its folder's Out mark",
    lens: "place", apps: [CLI], talks: [open("c0", CLI), ...pasts("c", CLI, 2)],
    expect: { open: [CLI.key], idle: [], counts: { open: 1 }, under: { [CLI.key]: ["c0"] }, links: { [CLI.key]: ["Last 2 inactive"] }, folder: { [CLI.key]: "f-cli" } },
  },
  {
    name: "5b CLI App with only past sessions: in Inactive apps; clicked, its own",
    lens: "place", apps: [CLI], talks: pasts("c", CLI, 2), toggled: [recent(CLI)],
    expect: { open: [], idle: [CLI.key], counts: { open: null, idle: 1 }, under: { [CLI.key]: ["c1", "c2"] } },
  },
  {
    name: "5c a CLI App the user added, no session read: in Inactive apps (ADR-0032 amendment 6)",
    lens: "place", apps: [], added: [CLI], talks: [],
    expect: { open: [], idle: [CLI.key], counts: { idle: 1 }, under: { [CLI.key]: [] } },
  },
  // 6. Codex app and Codex CLI on one folder.
  {
    name: "6a Codex app running with an open session, Codex CLI with past ones on its folder: two Apps, each its own",
    lens: "place", apps: [CODEX, CODEX_CLI], talks: [open("x1", CODEX), ...pasts("y", CODEX_CLI, 2)],
    expect: {
      open: [CODEX.key], idle: [CODEX_CLI.key], counts: { open: 1, idle: 1 },
      under: { [CODEX.key]: ["x1"], [CODEX_CLI.key]: [] }, links: { [CODEX.key]: [] }, folder: { [CODEX.key]: "f-codex" },
    },
  },
  {
    name: "6b … Codex CLI clicked: its past ones under it, none under the Codex app",
    lens: "place", apps: [CODEX, CODEX_CLI], talks: [open("x1", CODEX), ...pasts("y", CODEX_CLI, 2)], toggled: [recent(CODEX_CLI)],
    expect: { under: { [CODEX.key]: ["x1"], [CODEX_CLI.key]: ["y1", "y2"] } },
  },
  {
    name: "6c Codex app closed, a Codex CLI session open on its folder: the CLI in Open now, the app Inactive",
    lens: "place", apps: [CODEX_OFF, CODEX_CLI], talks: [past("x0", CODEX_OFF, 3), open("y1", CODEX_CLI)], toggled: [recent(CODEX_OFF)],
    expect: { open: [CODEX_CLI.key], idle: [CODEX_OFF.key], under: { [CODEX_CLI.key]: ["y1"], [CODEX_OFF.key]: ["x0"] }, folder: { [CODEX_CLI.key]: "f-codex" } },
  },
  {
    name: "6d Codex app running with nothing open, Codex CLI open on its folder: both in Open now, each its own",
    lens: "place", apps: [CODEX, CODEX_CLI], talks: [past("x0", CODEX, 3), open("y1", CODEX_CLI), past("y2", CODEX_CLI, 1)],
    expect: {
      open: [CODEX.key, CODEX_CLI.key], idle: [], counts: { open: 2 },
      under: { [CODEX.key]: ["x0"], [CODEX_CLI.key]: ["y1"] }, links: { [CODEX_CLI.key]: ["Last inactive"] },
    },
  },
  // The Out mark: the App's own folder, never a session's (amendment 8, UX spec §5 rule 6).
  {
    name: "mark: a running App whose open session runs on another Account's folder still wears its own folder's mark",
    lens: "place", apps: [EAST, CLI], talks: [open("o1", EAST, { account_id: "f-cli", account_label: "Side" }), open("c1", CLI)],
    expect: { open: [EAST.key, CLI.key], under: { [EAST.key]: ["o1"], [CLI.key]: ["c1"] }, folder: { [EAST.key]: "f-east", [CLI.key]: "f-cli" } },
  },
  // Every state at once: three exclusive sections (amendment 1).
  {
    name: "all at once: Open now = the running Apps, Running not added = the offer, Inactive = the rest; Needs you counts the ask",
    lens: "place", apps: [EAST, WEST, NORTH, SOUTH, GONE, CLI, CODEX, CODEX_CLI],
    talks: [open("o1", EAST), ...pasts("p", EAST, 4), ...pasts("w", WEST, 2), ...pasts("s", SOUTH, 2), open("c1", CLI, { state: "asks_you" }), past("y1", CODEX_CLI, 2), open("x1", CODEX)],
    offers: [{ id: "app:claude-app:/p/Stray", label: "Claude app · Stray", running: true }, { id: EAST.id, label: "Claude app · East", running: true }],
    expect: {
      open: [EAST.key, WEST.key, NORTH.key, CLI.key, CODEX.key], offered: ["app:claude-app:/p/Stray"], idle: [SOUTH.key, GONE.key, CODEX_CLI.key],
      counts: { needs: 1, open: 5, offered: 1, idle: 3 },
      under: { [EAST.key]: ["o1"], [WEST.key]: ["w1", "w2"], [NORTH.key]: [], [CLI.key]: ["c1"], [CODEX.key]: ["x1"], [SOUTH.key]: [], [GONE.key]: [], [CODEX_CLI.key]: [] },
    },
  },
  {
    // ASSUMED: D-64 lists every inactive App; the rail shows 6, and the rest leave their count (D-36: never silence).
    name: "Inactive apps past six: six drawn, Show 2 more, the head counts all eight",
    lens: "place", apps: ["A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8"].map((name) => claudeApp(name, null)), talks: [],
    expect: { open: [], counts: { idle: 8 }, idleMore: 2 },
  },
];

describe("rail matrix: App lens, each App state", () => {
  it.each(ROWS.filter((row) => !row.mismatch))("$name", (row) => check(row));
  it.fails.each(ROWS.filter((row) => row.mismatch))("$name (MISMATCH)", (row) => check(row));
});
