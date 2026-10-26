/**
 * The rest of the made-up Mac for the Processes view's preview: "Include
 * other apps" (each program's name and memory, nothing to stop, as
 * `query::other_rows`) and the Ports view's port inventory. Split from
 * `processes.ts` (the size budget).
 */
import type { PortInventoryInfo, PortRowInfo } from "../lib/bridge";
import type { ProcessGraphRowInfo } from "../types/generated/ProcessGraphRowInfo";
import type { ProcessQueryInput } from "../types/generated/ProcessQueryInput";
import { CLEAN, mac } from "./processes";
import type { PreviewMac, PreviewProgram } from "./processes-fixture";

/** What a search asks for (as `processes.ts` reads it). */
interface Find {
  ports: number[];
  pids: number[];
  words: string[];
}

/** "Include other apps", in Everything running only, with no other filter:
 * a heading, then each program the search keeps by name, most memory first. */
export function otherRows(source: PreviewMac, input: ProcessQueryInput, find: Find): ProcessGraphRowInfo[] {
  const plain = input.states.length === 0 && !input.has_port && !input.heavy && !input.account_id && !input.project_id && !input.session_id;
  if (!input.others || input.view !== "running" || !plain || (input.app && input.app !== "a:other") || find.ports.length + find.pids.length > 0) return [];
  const kept = (source.others ?? []).filter((program) => find.words.every((word) => program.name.toLowerCase().includes(word)));
  if (kept.length === 0) return [];
  const sum = (pick: (program: PreviewProgram) => number) => kept.reduce((total, program) => total + pick(program), 0);
  const other = (key: string, name: string, kind: "other" | "program", processes: number, rss: number): ProcessGraphRowInfo => ({
    key,
    kind: kind === "other" ? "app" : "program",
    depth: kind === "other" ? 0 : 1,
    parent_key: kind === "other" ? null : "a:other",
    has_children: kind === "other",
    state: "idle",
    origin: "other",
    owner_words: "Not in an AI session",
    cpu_pct: null,
    rss_bytes: rss,
    ports: [],
    app: { app_id: null, name, pid: null, ours: false, kind, sessions: 0 },
    session: null,
    process: null,
    rollup: { processes, rss_bytes: rss, cpu_pct: 0, ports: [], attention: 0, to_clean: 0, worst: null },
    session_key: null,
  });
  return [
    other("a:other", "Other apps on this Mac", "other", sum((p) => p.processes), sum((p) => p.rss_bytes)),
    ...[...kept].sort((a, b) => b.rss_bytes - a.rss_bytes).map((program) => other(`o:${program.name}`, program.name, "program", program.processes, program.rss_bytes)),
  ];
}

const ACCOUNT_WORDS: Record<string, string> = { "claude-work": "Work Max", "claude-personal": "Personal", codex: "ChatGPT Pro" };

/** Every port open on the made-up Mac, as the port inventory reads it
 * (a spec's `window.__AIJ_PORTS__` wins). */
export function previewPorts(): PortInventoryInfo {
  const seeded = (window as unknown as { __AIJ_PORTS__?: PortInventoryInfo }).__AIJ_PORTS__;
  if (seeded) return seeded;
  const rows: PortRowInfo[] = [];
  for (const app of mac().apps) {
    for (const session of app.sessions) {
      for (const node of session.processes) {
        for (const port of node.ports) {
          const ours = session.origin === "ours";
          rows.push({
            port,
            pid: node.pid,
            class: !ours ? "not-managed" : node.state === "picked_up" ? "re-adopted" : CLEAN.has(node.state) ? "managed-orphan" : "managed-running",
            session: ours ? session.session_id : null,
            provider: ours ? session.tool : null,
            account: ours && session.account_id ? (ACCOUNT_WORDS[session.account_id] ?? null) : null,
            project: ours && session.project_id ? session.project_id.replace(/^proj-/, "") : null,
            listener_age_ms: node.age_ms,
            name: node.name.split(" ")[0] ?? null,
            hint: port === 5173 ? "vite dev server port" : port < 9000 && port >= 3000 ? "common dev server port" : null,
            killable: ours,
          });
        }
      }
    }
  }
  rows.sort((a, b) => a.port - b.port);
  return { rows, note: null, hogs: [] };
}
