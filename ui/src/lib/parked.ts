/**
 * AJ-E1003 — a parked Instance's state in words. Only a Measured reset
 * gets a countdown; an Estimated one says so (with "check now"), never a
 * false countdown. The text carries the meaning (WCAG 1.4.1).
 */
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { whenWords } from "./time-words";

function countdown(ms: number): string {
  const minutes = Math.max(0, Math.round(ms / 60_000));
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return hours > 0 ? `${hours}h ${rest}m` : `${rest}m`;
}

export function parkedWords(
  session: Pick<InstanceInfo, "parked" | "parked_resets_at_ms">,
  now: number,
): { short: string; chip: string; long: string; due: boolean; estimated: boolean } | null {
  if (!session.parked) return null;
  const at = session.parked_resets_at_ms ?? null;
  if (session.parked !== "measured" || at === null) {
    return {
      short: "Parked · reset estimated",
      chip: "Parked (est.)",
      long: "Parked until the quota resets — the reset time is estimated, not measured. Check now for a measured time.",
      due: false,
      estimated: true,
    };
  }
  if (now >= at) {
    return { short: "Parked · reset reached", chip: "Parked · ready", long: "Parked — the quota has reset. Resume when you're ready.", due: true, estimated: false };
  }
  return {
    short: `Parked · resets in ${countdown(at - now)}`,
    chip: `Parked ${countdown(at - now)}`,
    long: `Parked until the quota resets in ${countdown(at - now)} (at ${whenWords(at, now)}).`,
    due: false,
    estimated: false,
  };
}
