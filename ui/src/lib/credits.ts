/**
 * Credits that carry an account's work on once a window is used up (the
 * maintainer's report, 0.3.2: Codex kept working on pre-bought credits
 * while AI Juggler said it had "run out of room"). The one rule, as the
 * backend's (`features::quota::CreditsDisplay::cover`): unlimited, or a
 * balance above zero; credits with no balance reported, probably (D-36's
 * word for an inferred state); a balance of 0, or no credits, don't.
 * Tool-neutral: whichever tool reports credits.
 */
import type { CreditsDisplay, QuotaDisplayData, WindowDisplay } from "./ipc/quota";
import { measured } from "./quotaWindow";
import { resetsWords, whenWords } from "./time-words";

/** Credits that cover: a balance, unlimited, or only probably. */
export type CreditsCover = { kind: "balance"; balance: number } | { kind: "unlimited" } | { kind: "probably" };

/** Whether `credits` carry the work on, by the one rule. */
export function creditsCover(credits: CreditsDisplay | null | undefined): CreditsCover | null {
  if (credits == null) return null;
  if (credits.unlimited) return { kind: "unlimited" };
  if (!credits.has_credits) return null;
  const raw = credits.balance?.trim() ?? "";
  if (raw === "") return { kind: "probably" };
  const balance = Number(raw);
  if (!Number.isFinite(balance) || balance <= 0) return null;
  // 0.3.2: a balance read hours ago may have run out since.
  return credits.stale ? { kind: "probably" } : { kind: "balance", balance };
}

/** A window used up: spent until its reset, or no whole percent left
 * (what every screen rounds to "0% left"). */
function usedUp(window: WindowDisplay | null | undefined): boolean {
  if (!window) return false;
  return window.state === "exhausted" || (measured(window) && Math.round(100 - window.percentage) <= 0);
}

/** A window of the account used up while its credits cover: the cover,
 * else `null` (no window used up, or credits that don't cover). */
export function usingCredits(display: QuotaDisplayData | null | undefined): CreditsCover | null {
  if (!display || display.unavailable_reason) return null;
  if (!usedUp(display.five_hour) && !usedUp(display.weekly)) return null;
  return creditsCover(display.credits);
}

/** When the credits stop being needed: the latest reset of a used-up
 * window (`null` when none is known). */
export function creditsUntil(display: QuotaDisplayData | null | undefined): number | null {
  const resets = [display?.five_hour, display?.weekly].filter(usedUp).map((window) => window!.resets_at_ms).filter((at): at is number => at != null);
  return resets.length > 0 ? Math.max(...resets) : null;
}

/** A balance as a whole number with thousands separators ("14,150"). */
export function wholeNumber(value: number): string {
  return Math.max(0, Math.round(value)).toLocaleString("en-US");
}

/** What's left, said short: "14,150 left", "unlimited"; `null` when the
 * tool didn't say. */
function leftWords(cover: CreditsCover): string | null {
  if (cover.kind === "balance") return `${wholeNumber(cover.balance)} left`;
  if (cover.kind === "unlimited") return "unlimited";
  return null;
}

/** A reading's age worth saying (0.3.2): past an hour, "as of 2:10 PM". */
export const AS_OF_AFTER_MS = 3_600_000;

/** The room, in plain short words: "Using credits · 14,150 left", or
 * "Probably using credits" when the balance isn't reported or is old;
 * with "as of …" once the reading is over an hour old. */
export function usingCreditsWords(cover: CreditsCover, asOfMs?: number | null, now: number = Date.now()): string {
  const left = leftWords(cover);
  const words = left ? `Using credits · ${left}` : "Probably using credits";
  return asOfMs != null && asOfMs > 0 && now - asOfMs > AS_OF_AFTER_MS ? `${words} (as of ${whenWords(asOfMs, now)})` : words;
}

/** What a session there does, in one line: "A session there goes on with
 * its credits until the window resets Wed 1:35 AM." `lead` names who
 * goes on ("Sessions on it go on"). */
export function creditsConsequence(resetsAtMs: number | null | undefined, now: number = Date.now(), lead = "A session there goes on"): string {
  const until = resetsAtMs != null && resetsAtMs > now ? `until the window ${resetsWords(resetsAtMs, now)}` : "until the window resets";
  return `${lead} with its credits ${until}.`;
}
