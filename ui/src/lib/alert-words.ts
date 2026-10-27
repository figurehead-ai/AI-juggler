/**
 * D-25 (AJ-E1025), wireframe 27: a quota alert in plain words, with its
 * reset in this Mac's local time — "Mia has 7% left in its 5-hour window,
 * until 9:09 PM". Never a window id (`five_hour`) or a tier tag
 * ("(Measured)"). The backend sends the numbers; an alert without them
 * (a digest line, one from an older build) says the backend's own words.
 */
import type { AlertEventData } from "./bridge";

export function windowWords(window: string): string {
  return window === "weekly" ? "weekly window" : "5-hour window";
}

/** "9:09 PM" today; "Thu 9:09 PM" further off. */
export function untilWords(atMs: number, now: number = Date.now()): string {
  const at = new Date(atMs);
  const time = at.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  const sameDay = at.toDateString() === new Date(now).toDateString();
  return sameDay ? time : `${at.toLocaleDateString([], { weekday: "short" })} ${time}`;
}

function minutesWords(minutes: number): string {
  const m = Math.max(1, Math.round(minutes));
  const hours = Math.floor(m / 60);
  if (hours === 0) return `${m} min`;
  return m % 60 === 0 ? `${hours} h` : `${hours} h ${m % 60} min`;
}

export function alertEventWords(event: AlertEventData, now: number = Date.now()): string {
  const quota = event.quota;
  if (!quota) return event.text;
  const who = quota.account_label;
  const window = windowWords(quota.window);
  const until = quota.resets_at_ms !== null && quota.resets_at_ms > now ? `, until ${untilWords(quota.resets_at_ms, now)}` : "";
  switch (event.kind) {
    case "tray-amber":
      return quota.minutes_left !== null
        ? `${who} has about ${minutesWords(quota.minutes_left)} of work left in its ${window}`
        : `${who} is running low in its ${window}${until}`;
    case "actionable":
      return quota.left_pct !== null
        ? `${who} has ${Math.round(quota.left_pct)}% left in its ${window}${until}`
        : `${who} is about to run out of its ${window}${until}`;
    case "exhausted":
      return `${who} has used up its ${window}${until}`;
    default:
      return event.text;
  }
}
