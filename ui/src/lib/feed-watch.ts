/**
 * AJ-E3713 R3 — a post's Follow, made the Watches way (the same calls as
 * the Watches form, `ipc/watch-view`): what the post's thing can be
 * watched for is the catalogue's (only what its Apps report, D-127).
 * AJ-E2027 (ADR-0039 §11.1) — its Remind me is a reminder on the one
 * scheduler (`ipc/schedules`), offered once the scheduler answers. Both
 * prime the Notifications prompt first, at the user's click (PAT-OSP,
 * AJ-E1548 #16); what the user makes is theirs to change on the Watches
 * page (its Reminders list).
 */
import { sharedRead, useSharedRead } from "./shared-read";
import { osPromptRequest } from "./ipc/os-prompts";
import { watchCatalogue, watchCreate, watchesPage, type WatchInfo } from "./ipc/watch-view";
import { scheduleRemind, schedulesSummary } from "./ipc/schedules";
import { switchSourceOf } from "./ipc/conversations";
import type { FeedItemInfo } from "./ipc/feed";
import type { WatchTarget } from "../types/generated/WatchTarget";

/** What following a post watches: its session, else its pool, its
 * project, its Account, its App. 0.5.2 G57 (RW1 F-RW2-1): a conversation
 * AI Juggler didn't start (a Codex thread) is a session too, by its key
 * where a session id goes (0.5.2 G45): a Remind me on its post was about
 * its project ("About web"), the toast about the thread. */
export function followTarget(item: FeedItemInfo): Exclude<WatchTarget, { kind: "any" }> | null {
  const { target } = item;
  if (target.session_id) return { kind: "session", id: target.session_id };
  if (target.conversation_id && target.account_id) return { kind: "session", id: switchSourceOf({ session_id: null, account_id: target.account_id, id: target.conversation_id }) };
  if (target.pool_id ?? item.facts.pool_id) return { kind: "pool", id: (target.pool_id ?? item.facts.pool_id)! };
  if (target.project_id ?? item.facts.project_id) return { kind: "project", id: (target.project_id ?? item.facts.project_id)! };
  if (target.account_id) return { kind: "account", id: target.account_id };
  if (item.author.kind === "app" && item.author.folder) return { kind: "app", id: item.author.folder };
  return null;
}

/** What a follow listens for, when offered: a thing's ending, its asks,
 * its failures, an Account's room. */
const FOLLOWED = ["finishes", "asks_you", "fails", "hits_limit", "runway_resets", "pool_message"];

/** Follows `target`: a watch for what it can report, until a session
 * ends, else until the user removes it. */
export async function follow(target: Exclude<WatchTarget, { kind: "any" }>): Promise<WatchInfo> {
  const offered = await watchCatalogue(target);
  const codes = FOLLOWED.filter((code) => offered.some((entry) => entry.code === code && (entry.param === null || !entry.param.required))).slice(0, 4);
  if (codes.length === 0) throw new Error("Nothing it does can be followed yet.");
  await osPromptRequest("notifications").catch(() => undefined);
  const made = await watchCreate({
    targets: [target],
    matching: "each",
    conditions: codes.map((code) => ({ code })),
    lifetime: target.kind === "session" ? { kind: "until_ends" } : { kind: "always" },
    evenInQuietHours: false,
  });
  void LIVE.refresh();
  return made;
}

/** The old reminder condition's code (a watch made before the scheduler
 * never counts as following). */
const REMIND = "remind";

/** Remind me's times: in 30 min, 1 h, 3 h, tomorrow at 9. */
export function REMIND_CHOICES(now: number): { label: string; minutes: number }[] {
  const tomorrow = new Date(now);
  tomorrow.setDate(tomorrow.getDate() + 1);
  tomorrow.setHours(9, 0, 0, 0);
  return [
    { label: "In 30 minutes", minutes: 30 },
    { label: "In 1 hour", minutes: 60 },
    { label: "In 3 hours", minutes: 180 },
    { label: "Tomorrow at 9 AM", minutes: Math.round((tomorrow.getTime() - now) / 60_000) },
  ];
}

/** A reminder about `target` (called `name`) in `minutes`: its id (the
 * toast opens it, 0.5.2 G23) and when it is due. */
export async function remindAt(target: Exclude<WatchTarget, { kind: "any" }>, name: string, minutes: number): Promise<{ id: string; due: number }> {
  await osPromptRequest("notifications").catch(() => undefined);
  const made = await scheduleRemind({ at_ms: Date.now() + minutes * 60_000, title: remindTitle(name), about: target, chat_turn: null });
  return { id: made.id, due: made.next_due_ms ?? made.created_ms + minutes * 60_000 };
}

/** A reminder's words: "Look at “api”". */
function remindTitle(name: string): string {
  return `Look at “${name}”`;
}

/** The live watches (at most 50), shared by every post. */
const LIVE = sharedRead(async () => (await watchesPage("live", { cursor: null, limit: 50 })).items, "the live watches");

/** Whether reminders can be set (D-127: Remind me shows only then): the
 * scheduler answers; read once for every post. */
const REMIND_OFFERED = sharedRead(async () => {
  await schedulesSummary();
  return true;
}, "the reminder's offer");

/** A followed target, and the watch that follows it. */
export interface Followed {
  target: WatchTarget;
  watch: WatchInfo;
}

const keyOf = (target: WatchTarget) => (target.kind === "any" ? "any" : `${target.kind}:${target.id}`);

/** The live watch that follows `target` (one that isn't a reminder). */
export function useFollowing(target: WatchTarget | null): Followed | null {
  const live = useSharedRead(LIVE, { maxAgeMs: 30_000 });
  if (!target || !live) return null;
  const key = keyOf(target);
  const watch = live.find((info) => info.watch.targets.some((one) => keyOf(one) === key) && info.watch.conditions.some((one) => one.code !== REMIND));
  return watch ? { target, watch } : null;
}

/** Whether Remind me can be offered. */
export function useRemindOffered(): boolean {
  return useSharedRead(REMIND_OFFERED, { maxAgeMs: 5 * 60_000 }) === true;
}
