/**
 * 0.5.2 G73 (RW1 Part T, T4): AI Juggler's sessions working now, as their
 * rows read (`sessionState`, the rail's rule): Home's Running chip counted
 * every stored "running" one, a session whose turn had ended (Idle on the
 * rail) too. The backend's equal is `sessions::read_model::working_now`.
 */
import { sessionState } from "./rail-items";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** How many of `rows` are working now. */
export function runningNow(rows: readonly InstanceInfo[]): number {
  return rows.filter((row) => sessionState(row).state === "working").length;
}
