/**
 * The Processes view in the preview (no backend): the made-up Mac of
 * `processes-fixture` answered the way the backend's query answers it
 * (`crates/juggler-app/src/features/processes/query.rs`): a view, filters,
 * search, folding, then a page by cursor. A spec may seed its own Mac in
 * `window.__AIJ_PROCESSES__` (the same shape), or a large one with
 * `?aij_processes=large`.
 */
import type { ProcessQueryInput } from "../types/generated/ProcessQueryInput";
import type { ProcessStatusInfo } from "../types/generated/ProcessStatusInfo";
import type { Page } from "../types/generated/Page";
import type { PageRequest } from "../types/generated/PageRequest";
import type { ProcessCountsInfo } from "../types/generated/ProcessCountsInfo";
import type { ProcessGraphRowInfo } from "../types/generated/ProcessGraphRowInfo";
import type { RollupInfo } from "../types/generated/RollupInfo";
import type { CleanupEventInfo } from "../types/generated/CleanupEventInfo";
import type { RulePreviewInfo } from "../types/generated/RulePreviewInfo";
import type { CleanupPolicyInfo } from "../types/generated/CleanupPolicyInfo";
import type { ProcessSnapshotInfo } from "../types/generated/ProcessSnapshotInfo";
import type { KillExecuteData, KillVerifyData } from "../components/KillSheet";
import { type PreviewApp, type PreviewMac, type PreviewNode, type PreviewSession, previewMac } from "./processes-fixture";
import { otherRows } from "./processes-mac";

export const CLEAN = new Set(["left_behind", "left_behind_closed", "didnt_stop"]);
const RANK = ["didnt_stop", "left_behind", "left_behind_closed", "zombie", "wont_touch", "picked_up", "working", "idle", "ended"];

let made: PreviewMac | null = null;

export function mac(): PreviewMac {
  const seeded = (window as unknown as { __AIJ_PROCESSES__?: PreviewMac }).__AIJ_PROCESSES__;
  made ??= previewMac();
  return seeded ?? made;
}

const unmarked = (node: PreviewNode, now: number) =>
  node.mark === null || (node.mark === "snooze" && (node.mark_until_ms ?? 0) <= now);

const internal = (node: PreviewNode) => node.role === "hook" || node.role === "bridge";

/** What needs the user, as the backend decides it. */
export function attention(node: PreviewNode, now: number): boolean {
  if (internal(node) || !unmarked(node, now)) return false;
  return CLEAN.has(node.state) || node.heavy || node.state === "zombie";
}

function rollup(nodes: PreviewNode[], now: number): RollupInfo {
  const shown = nodes.filter((node) => !internal(node));
  const ports = [...new Set(shown.flatMap((node) => node.ports))].sort((a, b) => a - b);
  const states = shown.map((node) => node.state).sort((a, b) => RANK.indexOf(a) - RANK.indexOf(b));
  return {
    processes: shown.length,
    rss_bytes: shown.reduce((sum, node) => sum + node.rss_bytes, 0),
    cpu_pct: shown.reduce((sum, node) => sum + (node.cpu_pct ?? 0), 0),
    ports: ports.slice(0, 8),
    attention: shown.filter((node) => attention(node, now)).length,
    to_clean: shown.filter((node) => CLEAN.has(node.state) && unmarked(node, now)).length,
    worst: states[0] ?? null,
  };
}

function owner(origin: string, app: string): string {
  return origin === "ours" ? "Started by AI Juggler" : `In a session ${app} runs`;
}

/** A live session's own process AI Juggler started: stopped from its page
 * (the backend's `never::OUR_SESSION`). */
const OUR_SESSION = "The session itself: stop it from its page";

function processRow(app: PreviewApp, session: PreviewSession, node: PreviewNode, hasChildren: boolean): ProcessGraphRowInfo {
  const ownRoot = session.origin === "ours" && session.alive && node.role === "tool" && node.stop === "ours";
  return {
    key: node.key,
    kind: "process",
    depth: node.depth + 2,
    parent_key: session.key,
    has_children: hasChildren,
    state: node.state,
    origin: session.origin,
    owner_words: owner(session.origin, app.name),
    cpu_pct: node.cpu_pct,
    rss_bytes: node.rss_bytes,
    ports: node.ports,
    app: null,
    session: null,
    process: {
      pid: node.pid,
      start_time: node.start_time,
      name: node.name,
      role: node.role,
      role_words: node.role_words,
      role_sure: ["tool", "hook", "bridge"].includes(node.role),
      label: node.label,
      spark: node.spark,
      peak_rss: node.peak_rss,
      age_ms: node.age_ms,
      cpu_time_ms: node.cpu_time_ms,
      waits_for_pid: node.waits_for_pid,
      waits_for_name: node.waits_for_name,
      stop: ownRoot ? "never" : node.stop,
      stop_reason: ownRoot ? OUR_SESSION : node.stop_reason,
      mark: node.mark,
      mark_until_ms: node.mark_until_ms,
      heavy: node.heavy,
      failed: node.failed,
      internal: internal(node),
    },
    rollup: null,
    session_key: session.key,
  };
}

function search(text: string) {
  const out = { ports: [] as number[], pids: [] as number[], words: [] as string[] };
  for (const token of text.toLowerCase().split(/\s+/).filter(Boolean)) {
    const port = token.startsWith("port:") ? token.slice(5) : token.startsWith(":") ? token.slice(1) : null;
    if (port && /^\d+$/.test(port)) out.ports.push(Number(port));
    else if (token.startsWith("pid:") && /^\d+$/.test(token.slice(4))) out.pids.push(Number(token.slice(4)));
    else out.words.push(token);
  }
  return out;
}

/** Every row the view shows, in tree order (as `query::rows`). */
export function previewRows(source: PreviewMac, input: ProcessQueryInput): ProcessGraphRowInfo[] {
  const now = source.now;
  const find = search(input.search);
  const narrowed = find.ports.length + find.pids.length + find.words.length > 0 || input.states.length > 0 || input.has_port || input.heavy;
  const running = input.view === "running";
  const out: ProcessGraphRowInfo[] = [];
  for (const app of source.apps) {
    if (input.app && app.key !== input.app && app.name.toLowerCase() !== input.app.toLowerCase()) continue;
    const rows: ProcessGraphRowInfo[] = [];
    const sessions: PreviewSession[] = [];
    for (const session of app.sessions) {
      if (input.session_id && session.session_id !== input.session_id && session.key !== input.session_id) continue;
      if (input.account_id && session.account_id !== input.account_id) continue;
      if (input.project_id && session.project_id !== input.project_id) continue;
      const passes = (node: PreviewNode) =>
        (input.show_internal || !internal(node)) &&
        (input.states.length === 0 || input.states.includes(node.state)) &&
        (!input.has_port || node.ports.length > 0) &&
        (!input.heavy || node.heavy);
      const matches = (node: PreviewNode) =>
        find.ports.every((port) => node.ports.includes(port)) &&
        (find.pids.length === 0 || find.pids.includes(node.pid)) &&
        find.words.every((word) =>
          [node.name, node.role_words, node.label ?? "", session.title, app.name, node.state].some((hay) => hay.toLowerCase().includes(word)),
        );
      const keep = session.processes.filter((node) => passes(node) && matches(node) && (input.session_id || running || attention(node, now)));
      if (keep.length === 0 && !input.session_id) continue;
      const shown = new Set(keep.map((node) => node.key));
      if (!running || narrowed) {
        const byPid = new Map(session.processes.map((node) => [node.pid, node]));
        for (const node of keep) {
          let parent = node.parent_pid;
          for (let hops = 0; parent !== null && hops < 64; hops++) {
            const up = byPid.get(parent);
            if (!up || shown.has(up.key)) break;
            shown.add(up.key);
            parent = up.parent_pid;
          }
        }
      } else {
        for (const node of session.processes) if (passes(node)) shown.add(node.key);
      }
      const total = rollup(session.processes, now);
      const folded = input.session_id || narrowed || input.expanded.includes(session.key)
        ? false
        : input.collapsed.includes(session.key) || (total.attention === 0 && running);
      const listed = session.processes.filter((node) => shown.has(node.key));
      sessions.push(session);
      rows.push({
        key: session.key,
        kind: "session",
        depth: 1,
        parent_key: app.key,
        has_children: listed.length > 0,
        state: session.state,
        origin: session.origin,
        owner_words: owner(session.origin, app.name),
        cpu_pct: total.cpu_pct,
        rss_bytes: total.rss_bytes,
        ports: total.ports,
        app: null,
        session: {
          session_id: session.session_id,
          origin: session.origin,
          tool: session.tool,
          account_id: session.account_id,
          project_id: session.project_id,
          title: session.title,
          folder_name: session.project_id?.replace(/^proj-/, "") ?? null,
          state: session.state,
          alive: session.alive,
          root_pid: session.root_pid,
          folded,
          shared_by: 0,
          started_ms: null,
        },
        process: null,
        rollup: total,
        session_key: session.key,
      });
      if (folded) continue;
      listed.forEach((node, index) => rows.push(processRow(app, session, node, (listed[index + 1]?.depth ?? -1) > node.depth)));
    }
    if (sessions.length === 0) continue;
    const total = rollup(sessions.flatMap((session) => session.processes), now);
    out.push({
      key: app.key,
      kind: "app",
      depth: 0,
      parent_key: null,
      has_children: true,
      state: total.worst ?? "idle",
      origin: app.ours ? "ours" : "theirs",
      owner_words: app.ours ? "Started by AI Juggler" : `Sessions ${app.name} runs`,
      cpu_pct: total.cpu_pct,
      rss_bytes: total.rss_bytes,
      ports: total.ports,
      app: { app_id: app.app_id, name: app.name, pid: app.pid, ours: app.ours, kind: app.kind, sessions: sessions.length },
      session: null,
      process: null,
      rollup: total,
      session_key: null,
    });
    out.push(...rows);
  }
  out.push(...otherRows(source, input, find));
  return out;
}

/** The counts, as the backend's. */
export function previewCounts(source: PreviewMac = mac()): ProcessCountsInfo {
  const nodes = source.apps.flatMap((app) => app.sessions.flatMap((session) => session.processes)).filter((node) => !internal(node));
  const count = (test: (node: PreviewNode) => boolean) => nodes.filter(test).length;
  return {
    processes: nodes.length,
    sessions: source.apps.reduce((sum, app) => sum + app.sessions.length, 0),
    attention: count((node) => attention(node, source.now)),
    to_clean: count((node) => CLEAN.has(node.state) && unmarked(node, source.now)),
    left_behind: count((node) => node.state === "left_behind" || node.state === "left_behind_closed"),
    zombies: count((node) => node.state === "zombie"),
    didnt_stop: count((node) => node.state === "didnt_stop"),
    wont_touch: count((node) => node.state === "wont_touch"),
  };
}

/** One page, as the backend's. */
export function previewPage(input: ProcessQueryInput, page: PageRequest): Page<ProcessGraphRowInfo> {
  const source = mac();
  const rows = input.view === "ended" ? source.ended : previewRows(source, input);
  const limit = Math.min(Math.max(page.limit ?? 50, 1), 200);
  let start = 0;
  if (page.cursor) {
    const at = page.cursor.indexOf(":");
    const key = page.cursor.slice(at + 1);
    const found = rows.findIndex((row) => row.key === key);
    start = found >= 0 ? found + 1 : Number(page.cursor.slice(0, at)) || 0;
  }
  const items = rows.slice(start, start + limit);
  const end = start + items.length;
  return { items, next_cursor: end < rows.length ? `${end}:${rows[end - 1]!.key}` : null, total: rows.length };
}

/** The view's state, as the backend's. */
export function previewStatus(): ProcessStatusInfo {
  const source = mac();
  return { version: source.version, counts: previewCounts(source), built_ms: source.now, ledger_degraded: false };
}

/** Keep / Remind me later in the preview: held in the seeded Mac. */
export function previewMark(pid: number, startTime: number, mark: "keep" | "snooze" | null, minutes?: number): void {
  const source = mac();
  for (const node of source.apps.flatMap((app) => app.sessions.flatMap((session) => session.processes))) {
    if (node.pid === pid && node.start_time === startTime) {
      node.mark = mark;
      node.mark_until_ms = mark === "snooze" ? source.now + (minutes ?? 60) * 60_000 : null;
    }
  }
  source.version += 1;
  (window as unknown as { __AIJ_PROCESSES__?: PreviewMac }).__AIJ_PROCESSES__ = source;
  window.dispatchEvent(new CustomEvent("aij:processes-changed", { detail: { version: source.version, counts: previewCounts(source) } }));
}

/** What the sheet reads from the live process (shown, never kept). */
const COMMANDS: Record<number, string> = {
  7301: "node node_modules/.bin/astro dev --port 4321",
  7302: "esbuild --service=0.21.5 --ping",
};

export function previewObservedVerify(pid: number, _start: number | null): KillVerifyData {
  const node = mac().apps.flatMap((app) => app.sessions.flatMap((session) => session.processes)).find((n) => n.pid === pid);
  const seeded = (window as unknown as { __AIJ_OBSERVED_VERIFY__?: (pid: number) => KillVerifyData }).__AIJ_OBSERVED_VERIFY__;
  if (seeded) return seeded(pid);
  return {
    machine_id: "mac-local-1",
    pid,
    session: "conv-ended",
    verified: node?.stop === "observed",
    refusal: node?.stop === "observed" ? null : (node?.stop_reason ?? "AI Juggler never saw this process in a session, so it won't touch it."),
    exe_path: "/usr/local/bin/node",
    cmdline: COMMANDS[pid] ?? node?.name ?? "",
    cwd: "/Users/you/Dev/landing",
    ports: node?.ports ?? [],
    pgid: null,
    escaped_group: false,
    members: [{ pid, pgid: null, exe: "node", cmdline: COMMANDS[pid] ?? node?.name ?? "", role_guess: "DevServer", unsaved_state_warning: null, from_ledger: false }],
    tree_pids: [pid],
    platform: "TERM → grace → KILL",
    grace_secs: 5,
    provider: "",
    account_id: null,
    project_id: null,
  };
}

export function previewObservedStop(pid: number): KillExecuteData {
  return { refusal: null, members: [{ pid, outcome: "exited on TERM" }], cancelled: false, all_succeeded: true, still_holding: [], reoffer_pid: null };
}

export { previewPorts } from "./processes-mac";

export function previewHistory(sessionId: string | null, projectId: string | null, _cursor: string | null): Page<CleanupEventInfo> {
  const items = mac().history.filter(
    (event) => (!sessionId || event.session_id === sessionId) && (!projectId || event.project_id === projectId),
  );
  return { items, next_cursor: null, total: items.length };
}

export function previewDryRun(accountId: string, tier: 1 | 2): RulePreviewInfo {
  const items = mac()
    .apps.filter((app) => app.ours)
    .flatMap((app) => app.sessions.filter((session) => session.account_id === accountId))
    .flatMap((session) =>
      session.processes
        .filter((node) => CLEAN.has(node.state) && node.state !== "didnt_stop")
        .map((node) => {
          const ok = node.role === "dev_server" || node.role === "watcher";
          const ports = node.ports.map((port) => ` :${port}`).join("");
          return {
            pid: node.pid,
            name: `${node.name}${ports}`,
            session_id: session.session_id,
            when: node.heavy || !ok ? "never" : "now",
            reason: node.heavy ? "It is heavy: you decide, never a rule" : ok ? null : "Not a dev server or a watcher",
          };
        }),
    );
  const now = items.filter((item) => item.when === "now").map((item) => item.name);
  return { account_id: accountId, tier, items, words: now.length ? `Would tidy ${now.join(", ")} now` : "Nothing to tidy now" };
}

export function previewPolicy(): CleanupPolicyInfo {
  return mac().policy;
}

export function previewSnapshot(): ProcessSnapshotInfo {
  const source = mac();
  return { file_name: `processes-${source.now}.json`, json: JSON.stringify({ taken_at_ms: source.now, counts: previewCounts(source) }, null, 2) };
}
