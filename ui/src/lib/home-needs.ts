/**
 * AJ-E2105 — Home's Needs you: every kind of thing that waits on the user,
 * in one list, each with its own primary. A session asking (Go to it), a
 * limit reached (Continue elsewhere, or Wait), a sign-in, a Mac working
 * hard, processes left behind. (On 0.2: agent messages, agent requests,
 * prepared launches and paused chains join as kinds here.)
 */
import type { AccountInfo } from "./bridge";
import type { RailItem } from "./rail-items";

export type HomeNeedKind = "session" | "limit" | "failed" | "signin" | "mac" | "heavy" | "leftovers";

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
}

/** The order kinds come in: what blocks work first. */
const ORDER: HomeNeedKind[] = ["session", "limit", "failed", "signin", "mac", "heavy", "leftovers"];

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
  return [item.project, item.place.app, item.place.profile, item.accountLabel !== item.place.profile ? item.accountLabel : null].filter(Boolean).join(" · ");
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

/** The heading: one sentence about what needs the user; with nothing,
 * whether anything has run out, then whether anything runs. */
export function needsHeading(count: number, calm: { runOut: string[]; running: boolean; anything: boolean }): string {
  if (count > 0) return count === 1 ? "1 thing needs you" : `${count} things need you`;
  // QA batch 9: never "All clear" while an account has run out.
  if (calm.runOut.length === 1) return `${calm.runOut[0]} has run out`;
  if (calm.runOut.length > 1) return `${calm.runOut.length} accounts have run out`;
  return calm.running ? "All clear — nothing needs you" : calm.anything ? "All clear" : "Ready when you are";
}

/** What the item's primary says, by kind. */
export function primaryWords(need: HomeNeed): string {
  switch (need.kind) {
    case "session":
      return "Go to it";
    case "limit":
      return "Continue elsewhere";
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
  }
}
