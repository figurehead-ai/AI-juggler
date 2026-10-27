/**
 * AJ-E2502 — Needs you, worked out once: every kind of thing that waits on
 * the user (lib/home-needs.ts), the monthly caps, agents' messages and
 * tasks, and the accounts almost out of room that AI Juggler itself
 * noticed (the notification model's own needs). Home lists it; the window
 * keeps the notification model in step with it (`NeedsSync`), so the bell,
 * the badge and the menu-bar item show the same count as Home.
 */
import { useMemo } from "react";
import type { ThisMacInfo } from "./machine";
import { homeNeeds, isSessionNeed, type HomeNeed } from "./home-needs";
import { itemsOf, needsYou, openNow, type RailItem } from "./rail-items";
import { isSnoozed, readSnooze } from "./railview";
import { useSeen } from "./seen";
import { useShownConversations } from "./shown-apps";
import { ownSignIn } from "./providers";
import { placeOf } from "./project-words";
import { signedIn, type AccountInfo, type InstanceInfo, type ProjectInfo } from "./bridge";
import { useCapNeeds, capTitle } from "../components/BudgetNeeds";
import { useMessageNeeds, type MessageNeedInfo } from "./ipc/messages";
import { useTaskNeeds, type TaskNeedInfo } from "./ipc/tasks";
import { notificationsPage, type NeedInput, type NoticeInfo, type OpenNowInput } from "./ipc/notices";
import { usePagedList } from "./paged";
import { quotaDetail } from "./notice-words";
import type { BudgetInfo } from "../types/generated/BudgetInfo";

export interface NeedsYou {
  /** The rail's items (Home's other counts read them too). */
  items: RailItem[];
  /** Every item Home lists with a primary, in its order. */
  needs: HomeNeed[];
  caps: BudgetInfo[];
  reloadCaps: () => void;
  messages: MessageNeedInfo[];
  tasks: TaskNeedInfo[];
  /** Sessions among them (as the rail's Needs you counts them). */
  sessions: number;
  /** The one count: everything that needs the user. */
  total: number;
}

/** The notification model's own needs (an account almost out of room). */
export function useModelNeeds(): NoticeInfo[] {
  const list = usePagedList<NoticeInfo>("notices:needs", (request) => notificationsPage("needs", request), { keyOf: (row) => String(row.id) });
  return useMemo(() => list.items.filter((row) => row.kind === "quota"), [list.items]);
}

/** The busiest live session on an account (a hand-over starts there). */
export function liveOn(sessions: InstanceInfo[], accountId: string | null | undefined): InstanceInfo | undefined {
  return sessions.find((s) => s.account_id === accountId && (s.status === "running" || s.status === "waiting_user" || s.status === "stuck"));
}

/** This computer's own need, in Home's words (screens/Home.tsx `macNeed`). */
export type MacNeedOf = (thisMac: ThisMacInfo | null, nameOf: (id: string) => string | null) => { title: string; detail: string; severe: boolean } | null;

export function useNeedsYou({ sessions, accounts, projects, thisMac, now, macNeed }: { sessions: InstanceInfo[]; accounts: AccountInfo[]; projects: ProjectInfo[]; thisMac: ThisMacInfo | null; now: number; macNeed: MacNeedOf }): NeedsYou {
  const capNeeds = useCapNeeds(); // AJ-E1514: a monthly cap at 80 % or 100 %.
  const messages = useMessageNeeds(); // E18
  const tasks = useTaskNeeds(); // E19
  const model = useModelNeeds();
  const { conversations } = useShownConversations();
  const seen = useSeen();
  const items = useMemo(() => {
    const snooze = readSnooze();
    return itemsOf({ sessions: sessions.filter((s) => !s.parked), conversations, projects, seen, snoozed: (row) => isSnoozed(row, snooze, now) });
  }, [sessions, conversations, projects, seen, now]);
  const needs = useMemo(() => {
    const listed = needsYou(items);
    const ids = new Set(listed.flatMap((item) => (item.session ? [item.session.id] : [])));
    // E14 (AJ-E1404 Req.3): a session hogging the Mac, unless listed already.
    const heavy = (thisMac?.heavy ?? []).filter((tree): tree is typeof tree & { session_id: string } => tree.session_id !== null && !ids.has(tree.session_id) && sessions.some((s) => s.id === tree.session_id));
    const nameOf = (id: string) => {
      const session = sessions.find((s) => s.id === id);
      return session ? placeOf(session, projects).title : null;
    };
    const mac = macNeed(thisMac, nameOf);
    const base = homeNeeds({
      items: listed,
      signIns: accounts.filter((account) => !signedIn(account.login_state)),
      signInDetail: (account) => `${ownSignIn(account.provider)}, in a terminal here — about thirty seconds. AI Juggler never sees your password.`,
      mac,
      heavy: heavy.map((tree) => {
        const place = placeOf(sessions.find((s) => s.id === tree.session_id)!, projects);
        const helpers = tree.helpers > 0 ? (tree.helpers === 1 ? "1 more process" : `${tree.helpers} more processes`) : null;
        return { sessionId: tree.session_id, title: tree.heavy_note ?? "a lot of memory or CPU", detail: [place.title, place.detail, helpers].filter(Boolean).join(" · ") };
      }),
      leftovers: thisMac?.leftover_processes ?? 0,
    });
    // AJ-E2501: what AI Juggler noticed itself — an account almost out of room.
    const quota: HomeNeed[] = model.map((notice) => ({
      key: `quota:${notice.id}`,
      kind: "quota",
      title: notice.title,
      detail: quotaDetail(notice, now),
      severe: false,
      account: accounts.find((account) => account.id === notice.target.account_id),
      sessionId: liveOn(sessions, notice.target.account_id)?.id,
    }));
    return [...base, ...quota];
  }, [items, thisMac, sessions, projects, accounts, model, now, macNeed]);
  const sessionCount = needs.filter(isSessionNeed).length;
  return {
    items,
    needs,
    caps: capNeeds.caps,
    reloadCaps: capNeeds.reload,
    messages,
    tasks,
    sessions: sessionCount,
    total: needs.length + capNeeds.caps.length + messages.length + tasks.length,
  };
}

const QUOTED: Record<string, (title: string) => string> = {
  asks: (title) => `“${title}” needs your reply`,
  stuck: (title) => `“${title}” may be stuck`,
  limit: (title) => `“${title}” hit its usage limit`,
  failed: (title) => `“${title}” stopped with an error`,
  review: (title) => `“${title}” is done — review it`,
};

/** The list as the notification model keeps it: every need but the
 * model's own (those it keeps itself). Words only, never an id. */
export function needInputsOf(needs: NeedsYou): NeedInput[] {
  const out: NeedInput[] = [];
  const blank = { session_id: null, conversation_id: null, account_id: null, project_id: null, page: null, task_id: null, pool_id: null };
  for (const need of needs.needs) {
    const item = need.item;
    if (need.kind === "quota") continue;
    if (item && item.need) {
      out.push({
        key: need.key,
        kind: item.need,
        title: QUOTED[item.need]!(item.title),
        detail: need.detail,
        project_id: item.projectId,
        project: item.project,
        target: { ...blank, session_id: item.session?.id ?? null, conversation_id: item.conversation?.id ?? null, account_id: item.conversation?.account_id ?? item.accountId, project_id: item.projectId },
        action: item.need === "limit" ? "limit" : "open",
      });
      continue;
    }
    const kind = need.kind === "mac" ? "room" : need.kind;
    const actions: Record<string, string> = { signin: "sign-in", room: "this-computer", heavy: "open", leftovers: "clean-up" };
    const action = actions[kind] ?? "open";
    const target = { ...blank, session_id: need.sessionId ?? null, account_id: need.account?.id ?? null, page: kind === "room" ? "this_mac" : kind === "leftovers" ? "processes" : null };
    out.push({ key: need.key, kind, title: need.title, detail: need.detail, project_id: null, project: null, target, action });
  }
  for (const cap of needs.caps) out.push({ key: `cap:${cap.id}`, kind: "cap", title: capTitle(cap), detail: "", project_id: null, project: null, target: { ...blank, page: "usage" }, action: "usage" });
  for (const message of needs.messages) out.push({ key: `message:${message.key}`, kind: "message", title: message.title, detail: [message.pool_name, message.detail].filter(Boolean).join(" · "), project_id: null, project: null, target: { ...blank, pool_id: message.pool_id, session_id: message.session_id }, action: "pool" });
  for (const task of needs.tasks) out.push({ key: `task:${task.id}`, kind: "task", title: task.title, detail: [task.pool_name, task.detail].filter(Boolean).join(" · "), project_id: task.project_id, project: null, target: { ...blank, pool_id: task.pool_id, task_id: task.task_id, project_id: task.project_id }, action: "pool" });
  return out;
}

/** What is open now, as Home counts it, by app (the menu-bar item). */
export function openNowOf(items: RailItem[]): OpenNowInput {
  const open = items.filter(openNow);
  const byApp = new Map<string, number>();
  for (const item of open) byApp.set(item.place.app, (byApp.get(item.place.app) ?? 0) + 1);
  return {
    total: open.length,
    working: open.filter((item) => item.state === "working").length,
    apps: [...byApp].map(([app, count]) => ({ app, count })).sort((a, b) => b.count - a.count || a.app.localeCompare(b.app)),
  };
}
