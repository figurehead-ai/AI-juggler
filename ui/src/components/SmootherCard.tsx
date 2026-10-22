/**
 * AJ-E1527 — "Make it smoother": the optional permissions, early and never
 * required. On the welcome, every one still missing, in the shared rows
 * (`OsAccessRows`); on Home afterwards, a small reminder for the first one
 * still missing. The welcome's "Not now" hides both for good, the
 * reminder's hides it for that permission; AI Juggler then asks only when
 * something needs it, in context. Never a modal.
 */
import { useEffect, useState, type JSX } from "react";
import { SectionTitle } from "./SectionTitle";
import { OsAccessRows } from "./OsAccessRows";
import { osAccessList, type OsAccessInfo } from "../lib/bridge";

const DISMISSED_KEY = "aij.smoother.dismissed";

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

export function SmootherCard({ variant }: { variant: "welcome" | "reminder" }): JSX.Element | null {
  const [entries, setEntries] = useState<OsAccessInfo[]>([]);
  const [dismissed, setDismissed] = useState<string[]>(readDismissed);
  useEffect(() => {
    void osAccessList().then(setEntries, () => setEntries([]));
  }, []);
  const missing = entries.filter((entry) => entry.status !== "allowed");
  if (variant === "welcome") {
    if (missing.length === 0 || dismissed.includes("all")) return null;
    return (
      <section aria-labelledby="smoother-title" className="flex flex-col gap-3" data-testid="smoother">
        <div className="flex items-baseline justify-between gap-3">
          <SectionTitle id="smoother-title">Make it smoother</SectionTitle>
          <button type="button" className="text-[13px] text-muted hover:text-ink hover:underline" onClick={() => setDismissed(dismiss("all"))} data-testid="smoother-not-now">
            Not now
          </button>
        </div>
        <p className="text-[13px] leading-relaxed text-muted">Optional: AI Juggler works without them. Allow opens the system's own prompt.</p>
        <OsAccessRows entries={entries} setEntries={setEntries} size="compact" testIdPrefix="smoother" />
      </section>
    );
  }
  // The welcome's "Not now" puts Home's reminder away too.
  const first = dismissed.includes("all") ? undefined : missing.find((entry) => !dismissed.includes(entry.id));
  if (!first) return null;
  return (
    <div data-testid="smoother-reminder">
      <OsAccessRows
        entries={[first]}
        setEntries={(next) => setEntries((current) => current.map((entry) => next.find((row) => row.id === entry.id) ?? entry))}
        size="compact"
        testIdPrefix="smoother-reminder"
        aside={(entry) => (
          <button type="button" className="px-2 text-[13px] text-muted hover:text-ink hover:underline" onClick={() => setDismissed(dismiss(entry.id))} data-testid="smoother-reminder-not-now">
            Not now <span className="sr-only">— {entry.title}</span>
          </button>
        )}
      />
    </div>
  );
}
