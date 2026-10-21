/**
 * What runs on an account, in words (Accounts' cards and the account page).
 */
import type { InstanceInfo } from "./bridge";

const LIVE = ["running", "starting", "waiting_user", "stuck"];

/** What runs on an account, in a few words ("2 running · 1 needs you").
 * `elsewhere`: its sessions started outside AI Juggler running now
 * (AJ-E1529), which count as running too. */
export function activityWords(sessions: InstanceInfo[], elsewhere = 0): string {
  const running = sessions.filter((s) => LIVE.includes(s.status)).length + elsewhere;
  const needs = sessions.filter((s) => s.status === "waiting_user" || s.status === "stuck" || s.status === "quota_blocked" || s.status === "failed");
  const parts = [running > 0 ? `${running} running` : null, needs.length > 0 ? `${needs.length} need${needs.length === 1 ? "s" : ""} you` : null].filter(Boolean);
  return parts.length > 0 ? parts.join(" · ") : "Nothing running";
}
