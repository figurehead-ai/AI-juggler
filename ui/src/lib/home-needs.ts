/**
 * AJ-E2105 — Home's Needs you: every kind of thing that waits on the user,
 * in one list, each with its own primary. A session asking (its page), a
 * limit reached (Continue in another app…, or Wait), a sign-in, a Mac working
 * hard, processes left behind. (On 0.2: agent messages, agent requests,
 * prepared launches and paused chains join as kinds here.)
 */
import type { AccountInfo } from "../types/generated/AccountInfo";
import { placeWords, type RailItem } from "./rail-items";
import type { ConversationInfo } from "./ipc/conversations";

export type HomeNeedKind = "session" | "limit" | "failed" | "signin" | "mac" | "heavy" | "leftovers" | "quota";

export interface HomeNeed {
  key: string;
  kind: HomeNeedKind;
  /** What needs the user, in one line. */
  title: string;
  /** Where: "project · app · profile · account", or the reason. */
  detail: string;
  /** Amber, or red for what stopped. */
  severe: boolean;
  item?: RailItem;
  account?: AccountInfo;
  sessionId?: string;
  /** An out-of-room account's open session in another app (F16). */
  conversation?: ConversationInfo;
  /** E3104: another app's session the list hasn't loaded, by its ids
   * (opened as notices open one, `openConversationById`). */
  conversationIds?: { accountId: string; id: string };
  /** E3104: when an unloaded session's need was first written (its
   * notice's time: the wait it shows, `lib/waiting` for a loaded one). */
  since?: number;
}

/** The order kinds come in: what blocks work first. */
const ORDER: HomeNeedKind[] = ["session", "limit", "failed", "quota", "signin", "mac", "heavy", "leftovers"];

export interface NeedInputs {
  /** The rail's Needs you (sessions that ask, are stuck, failed, hit a
   * limit, or finished something to review). */
  items: RailItem[];
  signIns: AccountInfo[];
  /** How a sign-in goes, in words ("Claude Code's own sign-in, …"). */
  signInDetail: (account: AccountInfo) => string;
  mac: { title: string; detail: string; severe: boolean } | null;
  heavy: { sessionId: string; title: string; detail: string }[];
  leftovers: number;
}

function where(item: RailItem): string {
  return [item.project, ...placeWords(item.place)].filter(Boolean).join(" · ");
}

export function homeNeeds({ items, signIns, signInDetail, mac, heavy, leftovers }: NeedInputs): HomeNeed[] {
  const out: HomeNeed[] = [];
  for (const item of items) {
    const kind: HomeNeedKind = item.need === "limit" ? "limit" : item.need === "failed" ? "failed" : "session";
    out.push({ key: item.key, kind, title: item.title, detail: where(item), severe: kind !== "session", item });
  }
  for (const account of signIns) {
    out.push({ key: `signin:${account.id}`, kind: "signin", title: `${account.label} needs its sign-in`, detail: signInDetail(account), severe: false, account });
  }
  if (mac) out.push({ key: "mac", kind: "mac", title: mac.title, detail: mac.detail, severe: mac.severe });
  for (const tree of heavy) {
    out.push({ key: `heavy:${tree.sessionId}`, kind: "heavy", title: `Working your Mac hard — ${tree.title}`, detail: tree.detail, severe: false, sessionId: tree.sessionId });
  }
  if (leftovers > 0) {
    out.push({ key: "leftovers", kind: "leftovers", title: leftovers === 1 ? "1 process left running" : `${leftovers} processes left running`, detail: "after their sessions closed", severe: false });
  }
  return out.sort((a, b) => ORDER.indexOf(a.kind) - ORDER.indexOf(b.kind));
}

/** The needs the rail's Needs you counts too: its sessions. */
export function isSessionNeed(need: HomeNeed): boolean {
  return need.kind === "session" || need.kind === "limit" || need.kind === "failed";
}

/** The heading: one sentence about what needs the user; with nothing,
 * whether anything has run out, then whether anything runs. Sessions are
 * counted as the rail's Needs you counts them, so the two agree ("3
 * sessions need you" beside "Needs you 3"); a sign-in, the Mac, a cap or
 * leftovers are the "more" (the review of 0.1.4: "4 things" beside 3). */
export function needsHeading(count: number | { sessions: number; others: number }, calm: { runOut: string[]; running: boolean; anything: boolean; known?: boolean }): string {
  const { sessions, others } = typeof count === "number" ? { sessions: 0, others: count } : count;
  if (sessions > 0) {
    const them = sessions === 1 ? "1 session" : `${sessions} sessions`;
    if (others === 0) return `${them} ${sessions === 1 ? "needs" : "need"} you`;
    return `${them} and ${others} more ${others === 1 ? "thing" : "things"} need you`;
  }
  if (others > 0) return others === 1 ? "1 thing needs you" : `${others} things need you`;
  // D-103.6: never "All clear" before the needs have loaded.
  if (calm.known === false) return "Checking what needs you…";
  // QA batch 9: never "All clear" while an account has run out (D-62:
  // `runOut` names Accounts, a folder in none by its own name).
  if (calm.runOut.length === 1) return `${calm.runOut[0]} has run out`;
  if (calm.runOut.length > 1) return `${calm.runOut.length} accounts have run out`;
  return calm.running ? "All clear — nothing needs you" : calm.anything ? "All clear" : "Ready when you are";
}

/** What the item's primary says, by kind. */
export function primaryWords(need: HomeNeed): string {
  switch (need.kind) {
    case "session":
      return "Open its page";
    // One word for one action (the 0.4 design review, 69): the asking
    // session's second choice says the same.
    case "limit":
      return "Continue in another app…";
    case "failed":
      return "See what happened";
    case "signin":
      return "Sign in";
    case "mac":
      return "See what's using it";
    case "heavy":
      return "Take a look";
    case "leftovers":
      return "Clean up";
    // AJ-E2501: an account almost out of room (the notification model's own).
    // F16: its open session's switch, else its account, named so.
    case "quota":
      return need.sessionId || need.conversation ? "Continue in another app…" : "Open its app";
  }
}

/** AJ-E2910: Home's filter chips (deep/11 §4.1), in the app's words. */
export type HomeChip = "all" | "asks" | "stuck" | "limits" | "review" | "signin";

const CHIP_LABEL: Record<Exclude<HomeChip, "all">, string> = {
  asks: "Asks you",
  stuck: "Probably stuck",
  // The row's own words (the 0.4 design review, 69).
  limits: "Limit reached",
  review: "To review",
  signin: "Sign-in",
};

/** The chip a need sits under (failed, This Mac, heavy work and leftovers
 * show under All only). */
export function chipOf(need: HomeNeed): Exclude<HomeChip, "all"> | null {
  switch (need.kind) {
    case "session":
      return need.item?.need === "asks" ? "asks" : need.item?.need === "stuck" ? "stuck" : need.item?.need === "review" ? "review" : null;
    case "limit":
    case "quota":
      return "limits";
    case "signin":
      return "signin";
    default:
      return null;
  }
}

/** The chips to offer: each kind with something in it (agents' messages
 * count as asks, tasks waiting on your review as review), All first;
 * none at all unless two or more kinds are present. */
export function homeChips(needs: HomeNeed[], extra: { asks: number; review: number } = { asks: 0, review: 0 }): { value: HomeChip; label: string; count: number }[] {
  const counts: Record<Exclude<HomeChip, "all">, number> = { asks: extra.asks, stuck: 0, limits: 0, review: extra.review, signin: 0 };
  for (const need of needs) {
    const chip = chipOf(need);
    if (chip) counts[chip] += 1;
  }
  const present = (Object.keys(CHIP_LABEL) as Exclude<HomeChip, "all">[]).filter((chip) => counts[chip] > 0);
  if (present.length < 2) return [];
  const total = needs.length + extra.asks + extra.review;
  return [{ value: "all", label: `All ${total}`, count: total }, ...present.map((chip) => ({ value: chip, label: `${CHIP_LABEL[chip]} ${counts[chip]}`, count: counts[chip] }))];
}

/** One of the heading's "more things", by name, and where its row is. */
export interface HeadlineExtra {
  key: string;
  words: string;
  /** The row it opens to on Home (a CSS selector inside Needs you). */
  target: string;
}

/** Past this many, the rest are one "N more" (to the Needs you card). */
const EXTRAS_SHOWN = 3;

/**
 * D-96 B (AJ-E3307 M2): the heading's "2 more things", named right under
 * it ("Work is almost out of room · ChatGPT needs its sign-in"), each a
 * click to its row. Every part the heading counts as "more" is named:
 * Home's own non-session needs by their titles, then monthly caps,
 * agents' messages and tasks by their count.
 */
export function headlineExtras(needs: HomeNeed[], more: { caps: string[]; messages: number; tasks: number }): HeadlineExtra[] {
  const all: HeadlineExtra[] = [
    ...needs.filter((need) => !isSessionNeed(need)).map((need) => ({ key: need.key, words: need.title, target: `[data-need-key="${CSS.escape(need.key)}"]` })),
    ...more.caps.map((words, index) => ({ key: `cap:${index}`, words, target: `[data-testid="home-cap"]:nth-of-type(${index + 1})` })),
    ...(more.messages > 0 ? [{ key: "messages", words: more.messages === 1 ? "1 message from an agent" : `${more.messages} messages from agents`, target: '[data-testid="message-needs"]' }] : []),
    ...(more.tasks > 0 ? [{ key: "tasks", words: more.tasks === 1 ? "1 task waits on you" : `${more.tasks} tasks wait on you`, target: '[data-testid="task-needs"]' }] : []),
  ];
  if (all.length <= EXTRAS_SHOWN + 1) return all;
  return [...all.slice(0, EXTRAS_SHOWN), { key: "more", words: `${all.length - EXTRAS_SHOWN} more`, target: '[data-testid="home-needs"]' }];
}
