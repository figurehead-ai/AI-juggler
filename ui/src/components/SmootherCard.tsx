/**
 * AJ-E1527 — the optional permissions, never required: Home's small
 * reminder for the first one still missing, in the shared rows
 * (`OsAccessRows`). Its "Not now" hides it for that permission; AI
 * Juggler then asks only when something needs it, in context. Never a
 * modal. AJ-E1545: the welcome's "Make it smoother" card became the
 * setup wizard's permissions step (and opening at login its Keep it
 * running step); an earlier "Not now" on that card ("all") still puts
 * the reminder away.
 */
import { useEffect, useState, type JSX } from "react";
import { OsAccessRows } from "./OsAccessRows";
import { osAccessList } from "../lib/ipc/settings";
import type { OsAccessInfo } from "../types/generated/OsAccessInfo";

const DISMISSED_KEY = "aij.smoother.dismissed";
/** Permissions Home's reminder has been shown for, in an earlier run. */
const SHOWN_KEY = "aij.smoother.shown";
/** This run's marker: the reminder stays up for the run it first shows in. */
const RUN_KEY = "aij.smoother.run";

/** Home's reminder shows in one run only (the review of 0.1.4: a quiet,
 * one-time spot); afterwards the permission waits in Settings. */
function shownBefore(id: string): boolean {
  try {
    const shown = JSON.parse(localStorage.getItem(SHOWN_KEY) ?? "[]") as unknown;
    const thisRun = sessionStorage.getItem(RUN_KEY) === id;
    return Array.isArray(shown) && shown.includes(id) && !thisRun;
  } catch {
    return false;
  }
}

function markShown(id: string): void {
  try {
    const shown = JSON.parse(localStorage.getItem(SHOWN_KEY) ?? "[]") as unknown;
    const next = [...new Set([...(Array.isArray(shown) ? shown.filter((x): x is string => typeof x === "string") : []), id])];
    localStorage.setItem(SHOWN_KEY, JSON.stringify(next));
    sessionStorage.setItem(RUN_KEY, id);
  } catch {
    /* a private window: shown again next time */
  }
}

/** What "Not now" put away: "all" (the welcome's card) or permission ids. */
function readDismissed(): string[] {
  try {
    const raw = JSON.parse(localStorage.getItem(DISMISSED_KEY) ?? "[]") as unknown;
    return Array.isArray(raw) ? raw.filter((id): id is string => typeof id === "string") : [];
  } catch {
    return [];
  }
}

function dismiss(id: string): string[] {
  const next = [...new Set([...readDismissed(), id])];
  try {
    localStorage.setItem(DISMISSED_KEY, JSON.stringify(next));
  } catch {
    /* a private window: hidden for this visit */
  }
  return next;
}

export function SmootherCard(): JSX.Element | null {
  const [entries, setEntries] = useState<OsAccessInfo[]>([]);
  const [dismissed, setDismissed] = useState<string[]>(readDismissed);
  useEffect(() => {
    void osAccessList().then(setEntries, () => setEntries([]));
  }, []);
  const missing = entries.filter((entry) => entry.status !== "allowed");
  const first = dismissed.includes("all") ? undefined : missing.find((entry) => !dismissed.includes(entry.id) && !shownBefore(entry.id));
  return first ? <Reminder key={first.id} first={first} setEntries={setEntries} setDismissed={setDismissed} /> : null;
}

function Reminder({ first, setEntries, setDismissed }: { first: OsAccessInfo; setEntries: (update: (current: OsAccessInfo[]) => OsAccessInfo[]) => void; setDismissed: (next: string[]) => void }): JSX.Element {
  useEffect(() => markShown(first.id), [first.id]);
  return (
    <div data-testid="smoother-reminder">
      <OsAccessRows
        entries={[first]}
        setEntries={(next) => setEntries((current) => current.map((entry) => next.find((row) => row.id === entry.id) ?? entry))}
        size="compact"
        testIdPrefix="smoother-reminder"
        aside={(entry) => (
          <button type="button" className="px-2 text-meta text-muted hover:text-ink hover:underline" onClick={() => setDismissed(dismiss(entry.id))} data-testid="smoother-reminder-not-now">
            Not now <span className="sr-only">— {entry.title}</span>
          </button>
        )}
      />
    </div>
  );
}
