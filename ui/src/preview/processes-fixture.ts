/**
 * The preview's made-up Mac for the Processes view (preview data only —
 * the app reads `processes_page`): AI Juggler's own sessions (a healthy
 * one with a dev server, a watcher and MCP servers; one that ended and
 * left a heavy dev server and a watcher behind; one picked up after a
 * restart; one with a zombie, a failed stop and a database AI Juggler
 * won't touch), the Claude app's and a terminal's sessions, and the
 * ChatGPT app's ended session's leftover. `?aij_processes=large` makes it
 * 50 sessions and 2,000 processes (the budget's Mac).
 */
import type { CleanupEventInfo } from "../types/generated/CleanupEventInfo";
import type { CleanupPolicyInfo } from "../types/generated/CleanupPolicyInfo";
import type { ProcessGraphRowInfo } from "../types/generated/ProcessGraphRowInfo";
import type { ProcRowInfo } from "../types/generated/ProcRowInfo";

export interface PreviewNode extends Omit<ProcRowInfo, "role_sure" | "internal"> {
  key: string;
  parent_pid: number | null;
  depth: number;
  state: string;
  cpu_pct: number | null;
  rss_bytes: number;
  ports: number[];
}

export interface PreviewSession {
  key: string;
  origin: "ours" | "theirs";
  session_id: string;
  tool: string;
  account_id: string | null;
  project_id: string | null;
  title: string;
  alive: boolean;
  state: string;
  root_pid: number | null;
  processes: PreviewNode[];
}

export interface PreviewApp {
  key: string;
  app_id: string | null;
  name: string;
  pid: number | null;
  ours: boolean;
  kind: string | null;
  sessions: PreviewSession[];
}

/** One program on the rest of the Mac: its name and memory only. */
export interface PreviewProgram {
  name: string;
  processes: number;
  rss_bytes: number;
}

export interface PreviewMac {
  now: number;
  version: number;
  apps: PreviewApp[];
  /** The rest of the Mac ("Include other apps"); absent in a seeded Mac. */
  others?: PreviewProgram[];
  ended: ProcessGraphRowInfo[];
  history: CleanupEventInfo[];
  policy: CleanupPolicyInfo;
}

const MIN = 60_000;
const H = 60 * MIN;
const MB = 1_000_000;
const ROLE_WORDS: Record<string, string> = {
  tool: "the session itself",
  hook: "AI Juggler hook",
  bridge: "AI Juggler's tools",
  mcp_server: "MCP server",
  database: "database",
  dev_server: "dev server",
  watcher: "watcher",
  build: "build",
  shell: "shell",
  helper: "helper",
};

/** A steady-ish CPU line ending at `now` (30 samples). */
function spark(level: number, seed: number): number[] {
  return Array.from({ length: 30 }, (_, at) => Math.max(0, level * (0.7 + 0.3 * Math.sin((at + seed) / 3))));
}

interface Shape {
  state?: string;
  cpu?: number;
  rss?: number;
  ports?: number[];
  age?: number;
  label?: string;
  /** AJ-E1520 (0.3): the subagent that probably started it (id, task). */
  subagent?: [string, string];
  stop?: "ours" | "observed" | "never";
  why?: string;
  heavy?: boolean;
  failed?: string;
  waits?: [number, string];
}

function node(now: number, pid: number, parent: number | null, depth: number, name: string, role: string, shape: Shape = {}): PreviewNode {
  const cpu = shape.cpu ?? 0.2;
  const rss = shape.rss ?? 40 * MB;
  const age = shape.age ?? 40 * MIN;
  return {
    key: `p:${pid}:${Math.floor((now - age) / 1000)}`,
    pid,
    start_time: Math.floor((now - age) / 1000),
    parent_pid: parent,
    depth,
    name,
    role,
    role_words: ROLE_WORDS[role] ?? "helper",
    label: shape.label ?? null,
    subagent_id: shape.subagent?.[0] ?? null,
    subagent_title: shape.subagent?.[1] ?? null,
    state: shape.state ?? (cpu >= 2 ? "working" : "idle"),
    cpu_pct: shape.state === "zombie" ? null : cpu,
    spark: shape.state === "zombie" ? [] : spark(cpu, pid % 7),
    rss_bytes: shape.state === "zombie" ? 0 : rss,
    peak_rss: Math.round(rss * 1.3),
    ports: shape.ports ?? [],
    age_ms: age,
    cpu_time_ms: Math.round((cpu / 100) * age),
    waits_for_pid: shape.waits?.[0] ?? null,
    waits_for_name: shape.waits?.[1] ?? null,
    stop: shape.stop ?? "ours",
    stop_reason: shape.why ?? null,
    mark: null,
    mark_until_ms: null,
    heavy: shape.heavy ?? false,
    failed: shape.failed ?? null,
  };
}

function session(over: Partial<PreviewSession> & Pick<PreviewSession, "key" | "session_id" | "title" | "processes">): PreviewSession {
  return { origin: "ours", tool: "claude-code", account_id: "claude-work", project_id: null, alive: true, state: "idle", root_pid: over.processes[0]?.pid ?? null, ...over };
}

const THEIR_TOOL = "The session itself: stop it in its app";
const SESSION_RUNS = "Its session is still running in its app";

function ours(now: number): PreviewApp {
  const n = (...args: Parameters<typeof node> extends [number, ...infer R] ? R : never) => node(now, ...args);
  return {
    key: "a:ai-juggler",
    app_id: null,
    name: "AI Juggler",
    pid: 4000,
    ours: true,
    kind: "own",
    sessions: [
      session({
        key: "s:ours:s-pay",
        session_id: "s-pay",
        title: "Payments API",
        project_id: "proj-payments",
        alive: false,
        state: "ended",
        root_pid: null,
        processes: [
          n(8123, 8100, 0, "node (vite)", "dev_server", { state: "left_behind", rss: 3_100 * MB, cpu: 1.8, ports: [5173], age: 2 * H, heavy: true, label: "Started by Bash: npm run dev" }),
          n(8131, 8123, 1, "esbuild", "dev_server", { state: "left_behind", rss: 92 * MB, cpu: 0.4, age: 2 * H }),
          n(8140, 8100, 0, "node (nodemon)", "watcher", { state: "left_behind", rss: 60 * MB, age: 2 * H }),
          n(8155, 8100, 0, "python3 (http.server)", "dev_server", { state: "left_behind_closed", rss: 21 * MB, ports: [8000], age: 26 * H }),
        ],
      }),
      session({
        key: "s:ours:s-odd",
        session_id: "s-odd",
        tool: "codex",
        account_id: "codex",
        title: "Billing webhooks",
        project_id: "proj-api",
        state: "working",
        processes: [
          n(6600, 6500, 0, "codex", "tool", { cpu: 6.5, rss: 310 * MB, age: 55 * MIN }),
          n(6610, 6600, 1, "node (worker)", "helper", { state: "zombie", stop: "never", why: "Only its parent can clear it", waits: [6600, "codex"] }),
          n(6620, 6600, 1, "webpack", "dev_server", { state: "didnt_stop", cpu: 3.1, rss: 640 * MB, ports: [8081], failed: "It ignored the request to stop" }),
          n(6622, 6600, 1, "postgres", "database", { state: "wont_touch", ports: [5432], rss: 75 * MB, stop: "never", why: "AI Juggler can't confirm it's the process it started", age: 3 * H }),
        ],
      }),
      session({
        key: "s:ours:s-web",
        session_id: "s-web",
        title: "Refactor billing",
        project_id: "proj-web",
        state: "working",
        processes: [
          n(61001, 60900, 0, "claude", "tool", { cpu: 4.5, rss: 410 * MB, age: 20 * MIN }),
          n(61020, 61001, 1, "node (mcp-server-filesystem)", "mcp_server", { rss: 90 * MB, age: 19 * MIN }),
          n(61044, 61001, 1, "npm", "helper", { rss: 60 * MB, age: 10 * MIN, label: "Started by Bash: npm run dev" }),
          n(61045, 61044, 2, "node (vite)", "dev_server", { cpu: 1.9, rss: 640 * MB, ports: [5180], age: 10 * MIN, subagent: ["a3", "Run the full test suite and report failures"] }),
          n(61046, 61045, 3, "esbuild", "dev_server", { rss: 30 * MB, age: 9 * MIN }),
          n(61060, 61001, 1, "node (tsc)", "watcher", { cpu: 0.8, rss: 240 * MB, age: 5 * MIN, label: "Started by Bash: npx tsc --watch" }),
          n(61070, 61001, 1, "juggler-agent", "bridge", { rss: 12 * MB, stop: "never", why: "Part of AI Juggler" }),
        ],
      }),
      session({
        key: "s:ours:s-docs",
        session_id: "s-docs",
        account_id: "claude-personal",
        title: "Docs site",
        project_id: "proj-docs",
        processes: [
          n(5500, 1, 0, "claude", "tool", { state: "picked_up", rss: 420 * MB, age: 5 * H }),
          n(5512, 5500, 1, "node (next)", "dev_server", { state: "picked_up", cpu: 0.7, rss: 520 * MB, ports: [8080], age: 5 * H }),
        ],
      }),
    ],
  };
}

function theirs(now: number): PreviewApp[] {
  const n = (...args: Parameters<typeof node> extends [number, ...infer R] ? R : never) => node(now, ...args);
  const live = (shape: Shape = {}): Shape => ({ stop: "never", why: SESSION_RUNS, ...shape });
  return [
    {
      key: "a:claude-app",
      app_id: "claude-app",
      name: "Claude",
      pid: 4100,
      ours: false,
      kind: "desktop",
      sessions: [
        session({
          key: "s:theirs:claude-code:blog",
          origin: "theirs",
          session_id: "blog",
          title: "Tidy the blog's images",
          account_id: "claude-personal",
          state: "working",
          processes: [
            n(4242, 4100, 0, "claude", "tool", live({ why: THEIR_TOOL, cpu: 7.2, rss: 380 * MB, age: 70 * MIN })),
            n(4250, 4242, 1, "node (mcp-server-github)", "mcp_server", live({ rss: 61 * MB, age: 70 * MIN })),
            n(4261, 4242, 1, "node (sharp)", "helper", live({ cpu: 22, rss: 410 * MB, age: 2 * MIN, label: "Started by Bash: npm run optimize" })),
          ],
        }),
        session({
          key: "s:theirs:claude-code:research",
          origin: "theirs",
          session_id: "research",
          title: "Read the pricing pages",
          processes: [n(4343, 4101, 0, "claude", "tool", live({ why: THEIR_TOOL, rss: 290 * MB, age: 3 * H }))],
        }),
      ],
    },
    {
      key: "a:codex-app",
      app_id: "codex-app",
      name: "ChatGPT",
      pid: 6100,
      ours: false,
      kind: "desktop",
      sessions: [
        session({
          key: "s:theirs:codex:landing",
          origin: "theirs",
          session_id: "landing",
          tool: "codex",
          account_id: "codex",
          title: "Landing page",
          alive: false,
          state: "ended",
          root_pid: null,
          processes: [
            n(7301, 1, 0, "node (astro)", "dev_server", { state: "left_behind", stop: "observed", rss: 180 * MB, ports: [4321], age: 3 * H }),
            n(7302, 7301, 1, "esbuild", "dev_server", { state: "left_behind", stop: "observed", rss: 24 * MB, age: 3 * H }),
          ],
        }),
      ],
    },
    {
      key: "a:terminal",
      app_id: "terminal",
      name: "Terminal",
      pid: 5100,
      ours: false,
      kind: "terminal",
      sessions: [
        session({
          key: "s:theirs:claude-code:login",
          origin: "theirs",
          session_id: "login",
          title: "Fix the flaky login test",
          state: "idle",
          processes: [
            n(5150, 5100, 0, "claude", "tool", live({ why: THEIR_TOOL, rss: 330 * MB, age: 25 * MIN })),
            n(5161, 5150, 1, "node (jest)", "watcher", live({ rss: 410 * MB, age: 24 * MIN, label: "Started by Bash: npx jest --watch" })),
          ],
        }),
      ],
    },
  ];
}

function ended(now: number): ProcessGraphRowInfo[] {
  const row = (pid: number, name: string, role: string, origin: "ours" | "theirs", ago: number, ran: number, owner: string): ProcessGraphRowInfo => ({
    key: `e:${pid}`,
    kind: "process",
    depth: 0,
    parent_key: null,
    has_children: false,
    state: "ended",
    origin,
    owner_words: owner,
    cpu_pct: null,
    rss_bytes: 0,
    ports: [],
    app: null,
    session: null,
    process: { ...node(now - ago, pid, null, 0, name, role, { stop: "never", why: "It has ended", age: ran }), spark: [], peak_rss: origin === "ours" ? Math.round((40 + (pid % 7) * 90) * 2 ** 20) : 0, role_sure: role === "tool", internal: false },
    rollup: null,
    session_key: null,
  });
  return [
    row(7710, "node (vite)", "dev_server", "ours", 25 * MIN, 3 * H, "Started by AI Juggler"),
    row(7702, "node (nodemon)", "watcher", "ours", 3 * H, 5 * H, "Started by AI Juggler"),
    row(7650, "python3 (http.server)", "dev_server", "theirs", 26 * H, 2 * H, "In a session Claude runs"),
    row(7600, "claude", "tool", "ours", 2 * 24 * H, 9 * H, "Started by AI Juggler"),
  ];
}

function history(now: number): CleanupEventInfo[] {
  const event = (id: number, ago: number, actor: string, pid: number, name: string, session: string, title: string, outcome: string, ports: string | null): CleanupEventInfo => ({
    id, at_ms: now - ago, actor, automatic: actor.startsWith("auto:"), pid, name, session_id: session, session_title: title, tool: "claude-code", account_id: "claude-work", project_id: "proj-payments", outcome, ports_before: ports, ports_after: null,
  });
  return [
    event(44, 12 * MIN, "user:observed", 7290, "node", "landing", "Landing page", "pid 7290: exited on TERM", "4322"),
    event(41, 25 * MIN, "user:kill_sheet", 7710, "node", "s-old", "Checkout redesign", "pid 7710: exited on TERM", "5174"),
    event(40, 3 * H, "auto:tier-1", 7702, "nodemon", "s-old", "Checkout redesign", "pid 7702: needed KILL", "4000"),
    event(39, 26 * H, "user:kill_sheet", 7650, "python3", "s-old2", "Search spike", "pid 7650: already gone", null),
  ];
}

/** A Mac of `sessions` AI Juggler sessions, `per` processes each. */
function large(now: number, sessions = 50, per = 40): PreviewApp[] {
  const app = ours(now);
  app.sessions = Array.from({ length: sessions }, (_, s) => {
    const base = 20_000 + s * 1_000;
    const left = s % 7 === 0;
    return session({
      key: `s:ours:big-${s}`,
      session_id: `big-${s}`,
      title: `Session ${s + 1}`,
      alive: !left,
      state: left ? "ended" : "idle",
      processes: Array.from({ length: per }, (_, p) =>
        node(now, base + p, p === 0 ? null : base + Math.floor((p - 1) / 3), Math.min(4, Math.floor(Math.log2(p + 1))), p === 0 ? "claude" : `node (worker-${p})`, p === 0 ? "tool" : p % 5 === 1 ? "dev_server" : "helper", {
          state: left && p > 0 ? "left_behind" : undefined,
          cpu: (p % 9) / 3,
          ports: p % 13 === 1 ? [3000 + s * 10 + p] : [],
        }),
      ),
    });
  });
  return [app];
}

/** The preview's Mac now. */
export function previewMac(): PreviewMac {
  const now = Date.now();
  const big = typeof location !== "undefined" && new URLSearchParams(location.search).get("aij_processes") === "large";
  return {
    now,
    version: 1,
    apps: big ? large(now) : [ours(now), ...theirs(now)],
    others: [
      { name: "Safari", processes: 14, rss_bytes: 2.4 * 2 ** 30 },
      { name: "Figma", processes: 6, rss_bytes: 1.6 * 2 ** 30 },
      { name: "Slack", processes: 5, rss_bytes: 820 * 2 ** 20 },
      { name: "Docker Desktop", processes: 4, rss_bytes: 610 * 2 ** 20 },
      { name: "Spotify", processes: 3, rss_bytes: 380 * 2 ** 20 },
      { name: "Finder", processes: 1, rss_bytes: 96 * 2 ** 20 },
    ],
    ended: ended(now),
    history: history(now),
    policy: {
      accounts: [
        { account_id: "claude-work", label: "Work Max", tier1: true, tier2: false },
        { account_id: "claude-personal", label: "Personal", tier1: false, tier2: false },
        { account_id: "codex", label: "ChatGPT Pro", tier1: false, tier2: false },
      ],
      paused: false,
      suspended_until_ms: null,
    },
  };
}
