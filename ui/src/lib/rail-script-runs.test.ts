/**
 * 0.4.3 (D-102): a script's run (`claude -p`, `codex exec`) is History
 * only. The backend marks it `app_unlisted` (unless AI Juggler ran it) and
 * the Apps list has no CLI App that only scripts ran in, so the rail shows
 * no CLI App for such a folder: not under Inactive apps, not by its
 * sessions. Codex threads sit under exactly one App by who started them:
 * the Codex app's under it, AI Juggler's and the terminal's under Codex
 * CLI, whose count is theirs alone.
 */
import { describe, expect, it } from "vitest";
import { check, claudeApp, cliApp, codexApp, past, PROJECTS, type FixtureApp, type MatrixRow } from "./rail-matrix.test-kit";
import { itemsOf } from "./rail-items";
import { appByConversation, appByFolder, idleApps, idleSessionsByApp } from "./rail-apps";
import type { ConversationInfo } from "./ipc/conversations";

const EAST = claudeApp("East", null);
const EAST_CLI = cliApp("claude-code", "f-east", "East");
const CODEX = codexApp(null);
const CODEX_CLI = cliApp("codex", "f-codex", "Codex home");
/** A script's run in `app`'s folder, as the backend lists it. */
const script = (id: string, app: FixtureApp, hours: number) => past(id, app, hours, { app_unlisted: true });

const ROWS: MatrixRow[] = [
  {
    name: "a folder only scripts ran in (claude -p, codex exec): no CLI App in Inactive apps, its runs under no App",
    lens: "place", apps: [EAST, CODEX],
    talks: [script("r1", EAST_CLI, 1), script("r2", EAST_CLI, 2), script("x1", CODEX_CLI, 3)],
    toggled: [`${EAST.key}#recent`, `${CODEX.key}#recent`],
    expect: { open: [], idle: [EAST.key, CODEX.key], counts: { open: null, idle: 2 }, under: { [EAST.key]: [], [CODEX.key]: [] } },
  },
  {
    name: "Codex: the app's threads under the Codex app, AI Juggler's under Codex CLI, the exec run under neither",
    lens: "place", apps: [CODEX, CODEX_CLI],
    talks: [past("d1", CODEX, 1), past("d2", CODEX, 2), past("a1", CODEX_CLI, 3), script("e1", CODEX_CLI, 4)],
    expect: { open: [], idle: [CODEX.key, CODEX_CLI.key], counts: { idle: 2 } },
  },
];

const byName = (a: string, b: string) => a.localeCompare(b);

/** Inactive apps as the rail works it out: each App's key → its sessions. */
function idle(talks: ConversationInfo[]) {
  const known = [EAST.desktop!, CODEX.desktop!];
  const items = itemsOf({ sessions: [], conversations: talks, projects: PROJECTS, seen: {}, pins: new Set() });
  const byApp = idleSessionsByApp(items, appByConversation(known), appByFolder(known));
  return new Map(idleApps({ openKeys: new Set(), known, idle: byApp }).map((app) => [app.key, app.sessions.map((item) => item.conversation?.id)]));
}

describe("D-102: scripts' runs are History only", () => {
  it.each(ROWS)("$name", (row) => check(row));

  it("idleApps: no CLI App for a folder only scripts ran in", () => {
    const apps = idle([script("r1", EAST_CLI, 1), script("x1", CODEX_CLI, 2)]);
    expect([...apps.keys()].sort(byName)).toEqual([EAST.key, CODEX.key].sort(byName));
    expect([...apps.values()]).toEqual([[], []]);
  });

  it("idleApps: each Codex thread under one App, Codex CLI's count without the exec run", () => {
    const talks = [past("d1", CODEX, 1), past("a1", CODEX_CLI, 2), past("t1", CODEX_CLI, 3), script("e1", CODEX_CLI, 4)];
    const apps = idle(talks);
    expect(apps.get(CODEX.key)).toEqual(["d1"]);
    expect(apps.get(CODEX_CLI.key)).toEqual(["a1", "t1"]);
    expect([...apps.keys()].sort(byName)).toEqual([CODEX.key, CODEX_CLI.key, EAST.key].sort(byName));
  });
});
