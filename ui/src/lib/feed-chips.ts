/**
 * AJ-E3713 R5 — what the feed's tabs and state chips keep, by an item's
 * kind: the same rules as the read model's (`features/feed/items.rs`,
 * `in_chip` and `in_state`), so the preview's stand-in and the chips'
 * counts agree with the backend. And R3's groups: "the kind of event" a
 * More or Less like this names (`features/feed/commands/prefs.rs`).
 */
import { THIS_MACHINE } from "./machine-words";

export type Tab = "all" | "needs" | "sessions" | "tasks" | "pools" | "switches" | "mac";
export type StateChip = "running" | "finished" | "failed";

/** The tabs, in the mock's order. */
export const TABS: readonly { value: Tab; label: string }[] = [
  { value: "all", label: "All" },
  { value: "needs", label: "Needs you" },
  { value: "sessions", label: "Sessions" },
  { value: "tasks", label: "Tasks" },
  { value: "pools", label: "Pools" },
  { value: "switches", label: "Switches & limits" },
  { value: "mac", label: THIS_MACHINE },
];

/** The state chips. */
export const STATES: readonly { value: StateChip; label: string }[] = [
  { value: "running", label: "Running" },
  { value: "finished", label: "Finished" },
  { value: "failed", label: "Failed" },
];

/** A kind's category (`category_of`). */
function categoryOf(kind: string): string {
  if (["failed", "crashed", "restarted", "answered", "switch_failed"].includes(kind)) return "needs";
  if (["finished", "ended", "stopped", "reviewed", "archived"].includes(kind)) return "finished";
  if (["switch", "quota", "reset", "renewal", "credits"].includes(kind)) return "switches";
  if (["task_done", "task_review", "task_blocked", "message"].includes(kind)) return "pools";
  if (["this_mac", "app_started", "keep_alive_paused", "update", "found"].includes(kind)) return "mac";
  return "other";
}

/** What a session did (the Sessions tab). */
const SESSION_KINDS = ["finished", "failed", "crashed", "ended", "restarted", "stopped", "reviewed", "archived", "answered", "worked"];

/** Whether an item is under `tab` (`in_chip`, and the item's own category). */
export function inTab(item: { kind: string; category: string }, tab: string): boolean {
  if (tab === "all") return true;
  if (item.category === tab) return true;
  if (tab === "needs") return false;
  if (tab === "sessions") return SESSION_KINDS.includes(item.kind);
  if (tab === "tasks") return item.kind.startsWith("task_");
  return categoryOf(item.kind) === tab;
}

/** Whether a kind is under a state chip that needs no live state. */
export function inState(kind: string, state: string): boolean {
  if (state === "finished") return kind === "finished" || kind === "task_done";
  if (state === "failed") return ["failed", "crashed", "switch_failed", "task_blocked"].includes(kind);
  return true;
}

/** The kind of event a lean names, by group, and its words. */
export const GROUPS: Record<string, string> = {
  finish: "finished work",
  failure: "failure",
  need: "answered need",
  switch: "switch",
  usage: "usage",
  task: "task",
  pool: "pool",
  mac: THIS_MACHINE,
  digest: "digest",
  watch: "watch",
};

/** A kind's group. */
export function groupOf(kind: string): string {
  if (["finished", "ended", "stopped", "reviewed", "archived", "worked"].includes(kind)) return "finish";
  if (["failed", "crashed", "restarted"].includes(kind)) return "failure";
  if (kind === "answered") return "need";
  if (kind === "switch" || kind === "switch_failed") return "switch";
  if (["quota", "reset", "renewal", "credits"].includes(kind)) return "usage";
  if (kind.startsWith("task_")) return "task";
  if (kind === "message") return "pool";
  if (kind === "agent_digest") return "digest";
  if (kind === "watch") return "watch";
  return "mac";
}
