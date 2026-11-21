/**
 * AJ-E2105 — Home's Needs you: every kind of thing that waits on the user,
 * in one list, each with its own primary. A session asking (its page), a
 * limit reached (Continue in another app…, or Wait), a sign-in, a Mac working
 * hard, processes left behind. (On 0.2: agent messages, agent requests,
 * prepared launches and paused chains join as kinds here.)
 */
import { attentionLevel, counted, itemLevel, type AttentionLevel } from "./attention";
import type { AccountInfo } from "../types/generated/AccountInfo";
import { placeWords, type RailItem } from "./rail-items";
import type { ConversationInfo } from "./ipc/conversations";
import { cliAppName } from "./app-name";

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
  /** D-117: its attention level, when the backend's row said it (else
   * worked out from its item or kind, `homeNeedLevel`). */
  level?: AttentionLevel;
}

/** A Home need's level (D-117): its own, else its rail item's, else its
 * kind's (a failure is broken; the rest wait on you). */
export function homeNeedLevel(need: Pick<HomeNeed, "level" | "item" | "kind">): AttentionLevel {
  if (need.level !== undefined) return need.level;
  if (need.item) return itemLevel(need.item);
  return attentionLevel({ need: need.kind === "mac" ? "room" : need.kind === "session" ? "asks" : need.kind });
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
  // Short, by its label ("lab A needs its sign-in"), unless another App's
  // sign-in reads the same: then each such one by its App's full name (the
  // 0.4.6 RC walk P8: OpenCode's and Kimi Code's both read "default needs
  // its sign-in"), so no two rows read the same.
  const shared = new Set(signIns.map((account) => account.label).filter((label, at, all) => all.indexOf(label) !== at));
  for (const account of signIns) {
    const name = shared.has(account.label) ? cliAppName(account.provider, account.label) : account.label;
    out.push({ key: `signin:${account.id}`, kind: "signin", title: `${name} needs its sign-in`, detail: signInDetail(account), severe: false, account });
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
function isSessionNeed(need: HomeNeed): boolean {
  return need.kind === "session" || need.kind === "limit" || need.kind === "failed";
}

/** The heading's counts (D-117): what waits on you or broke (L2, L3) —
 * Home's own needs and `own` (caps, agents' messages and tasks, all L2),
 * one number — and the notes listed but not counted ("1 to review"). */
export function headCount(needs: HomeNeed[], own: number): { count: number; notes: number } {
  const loud = needs.filter((need) => counted(homeNeedLevel(need))).length;
  return { count: loud + own, notes: needs.length - loud };
}

/** The heading (D-118's mock, "2 need you"): one count of what needs the
 * user, the same number as the rail's Needs you and the bell (AJ-E3602),
 * its parts said to assistive tech (`needsParts`); with nothing, whether
 * anything has run out, then whether anything runs. */
export function needsHeading(count: number, calm: { runOut: string[]; running: boolean; anything: boolean; known?: boolean }): string {
  if (count > 0) return `${count} ${count === 1 ? "needs" : "need"} you`;
  // D-103.6: never "All clear" before the needs have loaded.
  if (calm.known === false) return "Checking what needs you…";
  // QA batch 9: never "All clear" while an account has run out (D-62:
  // `runOut` names Accounts, a folder in none by its own name).
  if (calm.runOut.length === 1) return `${calm.runOut[0]} has run out`;
  if (calm.runOut.length > 1) return `${calm.runOut.length} accounts have run out`;
  return calm.running ? "All clear — nothing needs you" : calm.anything ? "All clear" : "Ready when you are";
}

/** What the heading's count is made of, for assistive tech (the visible
 * words stay "2 need you"): its sessions by count, then each other need
 * by its full name (a sign-in's App, `needName`), caps by their titles,
 * and agents' messages and tasks by their count. D-96 B's named parts,
 * now in the heading's spoken name: the visible "and 2 more things" they
 * explained is gone. */
export function needsParts(needs: HomeNeed[], more: { caps: string[]; messages: number; tasks: number }): string[] {
  const loud = needs.filter((need) => counted(homeNeedLevel(need)));
  const sessions = loud.filter(isSessionNeed).length;
  return [
    ...(sessions > 0 ? [sessions === 1 ? "1 session" : `${sessions} sessions`] : []),
    ...loud.filter((need) => !isSessionNeed(need)).map(needName),
    ...more.caps,
    ...(more.messages > 0 ? [more.messages === 1 ? "1 message from an agent" : `${more.messages} messages from agents`] : []),
    ...(more.tasks > 0 ? [more.tasks === 1 ? "1 task waits on you" : `${more.tasks} tasks wait on you`] : []),
  ];
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

/** The App a sign-in is for, by its full name ("Kimi Code CLI · lab A",
 * its tool's name from its adapter), when the need knows its App. */
function signInApp(need: Pick<HomeNeed, "kind" | "account">): string | null {
  return need.kind === "signin" && need.account ? cliAppName(need.account.provider, need.account.label) : null;
}

/** A need's name for assistive tech (AJ-E1612): its row's group, so each
 * row is told apart. A sign-in names its App in full ("Kimi Code CLI ·
 * lab A needs its sign-in"), where the visible title stays short ("lab A
 * needs its sign-in": several tools' Apps may share that label). */
export function needName(need: Pick<HomeNeed, "kind" | "account" | "title">): string {
  const app = signInApp(need);
  return app ? `${app} needs its sign-in` : need.title;
}

/** The primary's accessible name, its visible words first, so every
 * row's button is its own: "Sign in to Kimi Code CLI · lab A", else
 * "<its words> — <its title>". */
export function primaryName(need: HomeNeed): string {
  const app = signInApp(need);
  return app ? `${primaryWords(need)} to ${app}` : `${primaryWords(need)} — ${need.title}`;
}
