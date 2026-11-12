/**
 * AJ-E3505 — the preview's Floor snapshot: a small made-up company (generic
 * names only) with every state the Floor draws, so the Floor list works in
 * the dev preview and the gallery without a backend.
 */
import type { FloorApp, FloorSession, FloorSnapshot, FloorState } from "../lib/ipc/floor";

const NOW = Date.UTC(2026, 10, 4, 14, 0, 0);
const MIN = 60_000;

const WORDS: Record<FloorState, string> = {
  working: "Working",
  idle: "Idle",
  asks: "Asks you",
  stuck: "Probably stuck",
  limit: "Limit reached",
  crashed: "Stopped with an error",
  done: "Done, review it",
  ended: "Ended",
};

function seat(key: string, title: string, state: FloorState, ago: number, extra: Partial<FloorSession> = {}): FloorSession {
  return { key, title, state, word: WORDS[state], subagents: 0, project: "shop", tasks: 0, cpu_pct: state === "working" ? 18 : 1, started_ms: NOW - ago * MIN - 30 * MIN, last_ms: NOW - ago * MIN, ...extra };
}

function area(id: string, name: string, tool: string, family: string, slot: number, sessions: FloorSession[], extra: Partial<FloorApp> = {}): FloorApp {
  return {
    id, name, tool, host_app: null, logo: null, kind: id.startsWith("app:") ? "desktop" : "cli", family, colour_slot: slot, account_id: "acct:work", initials: "WM",
    open: sessions.some((s) => s.state !== "ended"), last_active_ms: Math.max(0, ...sessions.map((s) => s.last_ms)) || NOW - 3 * 24 * 60 * MIN, cpu_pct: 12, hot_since_ms: null, sessions, ...extra,
  };
}

/** The made-up company. */
export function previewFloor(): FloorSnapshot {
  return {
    at_ms: NOW,
    apps: [
      area("cli:work", "Claude Code · Work Max", "claude-code", "workshop", 0, [
        seat("s:p1", "Refund flow", "working", 1, { subagents: 3, tasks: 2 }),
        seat("s:p2", "Checkout copy", "asks", 2),
        seat("s:p3", "Flaky test", "stuck", 9, { project: "api" }),
      ]),
      area("app:claude-app:work", "Claude app · Work", "claude-code", "studio", 1, [
        seat("c:work/t1", "Launch notes", "done", 4, { project: null }),
        seat("c:work/t2", "Pricing page", "idle", 30),
      ]),
      area("cli:codex-home", "Codex · Home", "codex", "industrial", 2, [seat("s:p4", "Search index", "working", 0, { project: "api" }), seat("s:p5", "Migrations", "crashed", 6, { project: "api" })], {
        cpu_pct: 420,
        hot_since_ms: NOW - 4 * MIN,
        account_id: "acct:home",
        initials: "HO",
      }),
      area("cli:opencode-lab", "OpenCode · Lab", "opencode", "maker-garage", 3, [seat("s:p6", "Docs site", "limit", 12, { project: "docs" })], { account_id: "acct:home", initials: "HO" }),
      area("cli:old", "Claude Code · Archive", "claude-code", "workshop", 4, [], { open: false, last_active_ms: NOW - 9 * 24 * 60 * MIN }),
    ],
    accounts: [
      { id: "acct:work", label: "Work Max", left_pct: 62, resets_ms: NOW + 3 * 60 * MIN, last_used_ms: NOW - MIN, out: false },
      { id: "acct:home", label: "Home", left_pct: 0, resets_ms: NOW + 80 * MIN, last_used_ms: NOW - 6 * MIN, out: true },
    ],
    pools: [{ id: "pool-default:shop", name: "shop", sealed: false, sessions: ["s:p1", "s:p2"] }],
    machine: { load: 0.42, alert_since_ms: null, alert: null, room: "Plenty of room", alert_kind: null, alert_critical: false },
    notices: 3,
    needs: 4,
  };
}
