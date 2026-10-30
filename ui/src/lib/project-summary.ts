/**
 * 0.3 (the redesign, §3) — a project in a few facts, the same on the
 * Projects screen, a project's page header and its Overview: what needs
 * you there (from the backend's Needs you, D-49), its sessions open and
 * working (AI Juggler's and other apps'), and its open tasks.
 */
import type { InstanceInfo } from "./bridge";
import type { ConversationInfo } from "./ipc/conversations";
import type { NoticeInfo } from "./ipc/notices";
import type { TaskCounts } from "./ipc/tasks";
import type { PillTone } from "../components/Pill";
import { isActiveStatus } from "../components/indicator";
import { appOfConversation, appOfSession } from "./app-of";
import { liveOutside } from "./conversations";

/** What waits on you in one project, by kind of need. */
export type ProjectNeeds = { count: number; kinds: Map<string, number> };

/** The backend's Needs you, by project (one with none is left out). */
export function needsByProject(rows: NoticeInfo[]): Map<string, ProjectNeeds> {
  const out = new Map<string, ProjectNeeds>();
  for (const row of rows) {
    const id = row.project_id ?? row.target.project_id;
    if (!id) continue;
    const entry = out.get(id) ?? { count: 0, kinds: new Map() };
    entry.count += 1;
    entry.kinds.set(row.kind, (entry.kinds.get(row.kind) ?? 0) + 1);
    out.set(id, entry);
  }
  return out;
}

/** Each kind of need in words: one and many ("1 asks you", "2 ask you"). */
const NEED_WORDS: Record<string, [string, string, PillTone]> = {
  asks: ["asks you", "ask you", "warning"],
  stuck: ["stuck", "stuck", "warning"],
  limit: ["at its limit", "at their limit", "danger"],
  failed: ["failed", "failed", "danger"],
  review: ["to review", "to review", "accent"],
};

/** A project's needs as chips, the most pressing first. */
export function needChips(needs: ProjectNeeds | undefined): { key: string; words: string; tone: PillTone }[] {
  if (!needs) return [];
  const order = ["asks", "limit", "failed", "stuck", "review"];
  const rank = (kind: string) => (order.includes(kind) ? order.indexOf(kind) : order.length);
  const kinds = [...needs.kinds].sort(([a], [b]) => rank(a) - rank(b));
  let other = 0;
  const chips: { key: string; words: string; tone: PillTone }[] = [];
  for (const [kind, n] of kinds) {
    const words = NEED_WORDS[kind];
    if (!words) {
      other += n;
      continue;
    }
    chips.push({ key: kind, words: `${n} ${n === 1 ? words[0] : words[1]}`, tone: words[2] });
  }
  if (other > 0) chips.push({ key: "other", words: `${other} more for you`, tone: "neutral" });
  return chips;
}

/** A project's sessions now: open (AI Juggler's and other apps'),
 * working, and in how many apps. */
export type ProjectOpen = { open: number; working: number; apps: number };

/** Open sessions by project, from the app's working set and the other
 * apps' live conversations. */
export function openByProject(sessions: InstanceInfo[], conversations: ConversationInfo[]): Map<string, ProjectOpen> {
  const tally = new Map<string, { open: number; working: number; apps: Set<string> }>();
  const add = (projectId: string | null, working: boolean, app: string) => {
    if (!projectId) return;
    const entry = tally.get(projectId) ?? { open: 0, working: 0, apps: new Set<string>() };
    entry.open += 1;
    if (working) entry.working += 1;
    entry.apps.add(app);
    tally.set(projectId, entry);
  };
  for (const session of sessions) {
    if (isActiveStatus(session.status)) add(session.project_id, session.status === "running", appOfSession(session).appId ?? appOfSession(session).name);
  }
  for (const conversation of liveOutside(conversations)) {
    const app = appOfConversation(conversation);
    add(conversation.project_id, false, app.appId ?? app.name);
  }
  return new Map([...tally].map(([id, entry]) => [id, { open: entry.open, working: entry.working, apps: entry.apps.size }]));
}

/** "2 working · 3 open", or nothing open. */
export function openWords(open: ProjectOpen | undefined): string {
  if (!open || open.open === 0) return "Nothing open";
  return open.working > 0 ? `${open.working} working · ${open.open} open` : `${open.open} open`;
}

/** The open work, counted: every open task, ready and in review. */
export function openTasks(counts: TaskCounts | null | undefined): { open: number; ready: number; review: number } | null {
  if (!counts) return null;
  return { open: counts.ready + counts.in_progress + counts.in_review + counts.blocked + counts.todo, ready: counts.ready, review: counts.in_review };
}

/** "4 ready · 1 in review", else "No open tasks". */
export function taskWords(counts: TaskCounts | null | undefined): string {
  const tasks = openTasks(counts);
  if (!tasks) return "…";
  if (tasks.open === 0) return "No open tasks";
  const parts = [tasks.ready > 0 && `${tasks.ready} ready`, tasks.review > 0 && `${tasks.review} in review`].filter(Boolean);
  return parts.length > 0 ? parts.join(" · ") : `${tasks.open} open`;
}
