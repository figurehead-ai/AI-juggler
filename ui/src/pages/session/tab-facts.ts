/**
 * 0.3.4 — what a session's page last learned about its Messages and
 * Tasks tabs, per session, while the app runs: a page opened again has
 * the same tabs in its first frame (they popped in a read later, and the
 * tab it was left on was lost). Each fact is read again whenever its
 * page opens and when its messages or tasks change (the hooks that keep
 * it); at most `KEEP` sessions are remembered.
 */
import { whenForgotten } from "../../lib/shared-read";

const KEEP = 200;
const facts = new Map<string, boolean>();

/** The kept answer of `what` for a session (`undefined`: never read). */
export function keptFact(what: string, sessionId: string | null): boolean | undefined {
  return sessionId === null ? undefined : facts.get(`${what}:${sessionId}`);
}

export function keepFact(what: string, sessionId: string, value: boolean): void {
  const key = `${what}:${sessionId}`;
  facts.delete(key);
  facts.set(key, value);
  while (facts.size > KEEP) facts.delete(facts.keys().next().value as string);
}

whenForgotten(() => facts.clear());
