/**
 * The preview's stand-in for the backend's Needs you (no backend: the
 * gallery, the tests). The live app works the list out in the backend
 * (features/notifications/needs.rs, D-49) and nothing in the window does;
 * here the same rules run over the preview's own sessions, conversations,
 * accounts and This Mac, and hand the list to the notification stand-in
 * (lib/ipc/notices.ts), which the views then read as they read the
 * backend. Loaded only without a backend.
 */
import { useEffect, useMemo } from "react";
import { alertWords, reasonSentence, type ThisMacInfo } from "../lib/machine";
import { homeNeeds } from "../lib/home-needs";
import { itemsOf, needsYou } from "../lib/rail-items";
import { useSeen } from "../lib/seen";
import { useShownConversations } from "../lib/shown-apps";
import { useConversations } from "../lib/conversations";
import { ownSignIn } from "../lib/providers";
import { placeOf } from "../lib/project-words";
import {
  signedIn,
  type AccountInfo,
  type InstanceInfo,
  type ProjectInfo,
} from "../lib/bridge";
import { useCapNeeds, capTitle } from "../components/BudgetNeeds";
import { useMessageNeeds } from "../lib/ipc/messages";
import { useTaskNeeds } from "../lib/ipc/tasks";
import { standInNeeds, type StandInNeed } from "../lib/ipc/notices";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { BudgetInfo } from "../types/generated/BudgetInfo";
import type { MessageNeedInfo } from "../lib/ipc/messages";
import type { TaskNeedInfo } from "../lib/ipc/tasks";
import { applySummary } from "../lib/notices-store";

/** This computer's own need: its overload alerts, or one already under
 * pressure (needs.rs `room_need`). */
function macNeed(
  thisMac: ThisMacInfo | null,
  nameOf: (id: string) => string | null,
): { title: string; detail: string; severe: boolean } | null {
  if (!thisMac?.machine.sampled) return null;
  const alerts = thisMac.alerts.filter((alert) => alert.kind !== "heavy");
  if (alerts.length > 0)
    return {
      ...alertWords(alerts, nameOf)!,
      severe: thisMac.machine.summary.level === "pressure",
    };
  if (thisMac.machine.summary.level !== "pressure") return null;
  return {
    title: "Your Mac is under pressure",
    detail: reasonSentence(
      thisMac.machine.summary.reasons.slice(0, 2).map((reason) => reason.words),
    ),
    severe: true,
  };
}

const QUOTED: Record<string, (title: string) => string> = {
  asks: (title) => `“${title}” needs your reply`,
  stuck: (title) => `“${title}” may be stuck`,
  limit: (title) => `“${title}” hit its usage limit`,
  failed: (title) => `“${title}” stopped with an error`,
  review: (title) => `“${title}” is done — review it`,
};

const BLANK = {
  session_id: null,
  conversation_id: null,
  account_id: null,
  project_id: null,
  page: null,
  task_id: null,
  pool_id: null,
};
const ACTIONS: Record<string, string> = {
  signin: "sign-in",
  room: "this-computer",
  heavy: "open",
  leftovers: "clean-up",
};

export interface PreviewInputs {
  sessions: InstanceInfo[];
  conversations?: ConversationInfo[];
  projects?: ProjectInfo[];
  seen?: Record<string, number>;
  accounts?: AccountInfo[];
  thisMac?: ThisMacInfo | null;
  caps?: BudgetInfo[];
  messages?: MessageNeedInfo[];
  tasks?: TaskNeedInfo[];
}

/** The list the backend would work out from these (needs.rs's rules). */
export function previewNeedsOf({
  sessions,
  conversations = [],
  projects = [],
  seen = {},
  accounts = [],
  thisMac = null,
  caps = [],
  messages = [],
  tasks = [],
}: PreviewInputs): StandInNeed[] {
  const items = itemsOf({
    sessions: sessions.filter((s) => !s.parked),
    conversations,
    projects,
    seen,
  });
  const listed = needsYou(items);
  const ids = new Set(
    listed.flatMap((item) => (item.session ? [item.session.id] : [])),
  );
  const heavy = (thisMac?.heavy ?? []).filter(
    (tree): tree is typeof tree & { session_id: string } =>
      tree.session_id !== null &&
      !ids.has(tree.session_id) &&
      sessions.some((s) => s.id === tree.session_id),
  );
  const nameOf = (id: string) => {
    const session = sessions.find((s) => s.id === id);
    return session ? placeOf(session, projects).title : null;
  };
  const base = homeNeeds({
    items: listed,
    signIns: accounts.filter((account) => !signedIn(account.login_state)),
    signInDetail: (account) =>
      `${ownSignIn(account.provider)}, in a terminal here — about thirty seconds. AI Juggler never sees your password.`,
    mac: macNeed(thisMac, nameOf),
    heavy: heavy.map((tree) => {
      const place = placeOf(
        sessions.find((s) => s.id === tree.session_id)!,
        projects,
      );
      const helpers =
        tree.helpers > 0
          ? tree.helpers === 1
            ? "1 more process"
            : `${tree.helpers} more processes`
          : null;
      return {
        sessionId: tree.session_id,
        title: tree.heavy_note ?? "a lot of memory or CPU",
        detail: [place.title, place.detail, helpers]
          .filter(Boolean)
          .join(" · "),
      };
    }),
    leftovers: thisMac?.leftover_processes ?? 0,
  });
  const out: StandInNeed[] = [];
  for (const need of base) {
    const item = need.item;
    if (item && item.need) {
      out.push({
        key: need.key,
        kind: item.need,
        title: QUOTED[item.need]!(item.title),
        detail: need.detail,
        project_id: item.projectId,
        project: item.project,
        target: {
          ...BLANK,
          session_id: item.session?.id ?? null,
          conversation_id: item.conversation?.id ?? null,
          account_id: item.conversation?.account_id ?? item.accountId,
          project_id: item.projectId,
        },
        action: item.need === "limit" ? "limit" : "open",
      });
      continue;
    }
    const kind = need.kind === "mac" ? "room" : need.kind;
    const key = need.kind === "mac" ? "room" : need.key;
    out.push({
      key,
      kind,
      title: need.title,
      detail: need.detail,
      project_id: null,
      project: null,
      target: {
        ...BLANK,
        session_id: need.sessionId ?? null,
        account_id: need.account?.id ?? null,
        page:
          kind === "room"
            ? "this_mac"
            : kind === "leftovers"
              ? "processes"
              : null,
      },
      action: ACTIONS[kind] ?? "open",
    });
  }
  for (const cap of caps)
    out.push({
      key: `cap:${cap.id}`,
      kind: "cap",
      title: capTitle(cap),
      detail: "",
      project_id: null,
      project: null,
      target: { ...BLANK, page: "usage" },
      action: "usage",
    });
  for (const message of messages)
    out.push({
      key: `message:${message.key}`,
      kind: "message",
      title: message.title,
      detail: [message.pool_name, message.detail].filter(Boolean).join(" · "),
      project_id: null,
      project: null,
      target: {
        ...BLANK,
        pool_id: message.pool_id,
        session_id: message.session_id,
      },
      action: "pool",
    });
  for (const task of tasks)
    out.push({
      key: `task:${task.id}`,
      kind: "task",
      title: task.title,
      detail: [task.pool_name, task.detail].filter(Boolean).join(" · "),
      project_id: task.project_id,
      project: null,
      target: {
        ...BLANK,
        pool_id: task.pool_id,
        task_id: task.task_id,
        project_id: task.project_id,
      },
      action: "pool",
    });
  return out;
}

/** Snoozes are the stand-in's, by subject, as they are the backend's. */
export default function PreviewNeeds({
  ready,
  sessions,
  accounts,
  projects,
  thisMac,
}: {
  ready: boolean;
  sessions: InstanceInfo[];
  accounts: AccountInfo[];
  projects: ProjectInfo[];
  thisMac: ThisMacInfo | null;
}): null {
  const capNeeds = useCapNeeds();
  const messages = useMessageNeeds();
  const tasks = useTaskNeeds();
  const { conversations } = useShownConversations();
  const { loaded } = useConversations();
  const seen = useSeen();
  const body = useMemo(
    () =>
      JSON.stringify(
        previewNeedsOf({
          sessions,
          conversations,
          projects,
          seen,
          accounts,
          thisMac,
          caps: capNeeds.caps,
          messages,
          tasks,
        }),
      ),
    [
      sessions,
      conversations,
      projects,
      seen,
      accounts,
      thisMac,
      capNeeds.caps,
      messages,
      tasks,
    ],
  );
  // The caps answer too before the first list goes out, as the backend's
  // one read has them: what needs you comes in one order, never by which
  // read answered first.
  const capsKnown = capNeeds.known;
  useEffect(() => {
    if (!ready || !loaded || !capsKnown) return;
    applySummary(standInNeeds(JSON.parse(body) as StandInNeed[]));
  }, [ready, loaded, capsKnown, body]);
  return null;
}
