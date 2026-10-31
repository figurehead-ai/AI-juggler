/**
 * AJ-E1527 — "Make it smoother": the optional permissions, early and never
 * required; and (AJ-E1509) opening AI Juggler at login. On the welcome, every one still missing, in the shared rows
 * (`OsAccessRows`); on Home afterwards, a small reminder for the first one
 * still missing. The welcome's "Not now" hides both for good, the
 * reminder's hides it for that permission; AI Juggler then asks only when
 * something needs it, in context. Never a modal.
 */
import { useEffect, useState, type JSX } from "react";
import { SectionTitle } from "./SectionTitle";
import { OsAccessRows } from "./OsAccessRows";
import { errorText, osAccessList, type OsAccessInfo } from "../lib/bridge";
import { appLoginItemGet, appLoginItemSet, type LoginItemInfo } from "../lib/ipc/app";
import { Button } from "./primitives";

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

/** AJ-E1509: "Open AI Juggler at login" on the welcome, in the
 * permission rows' look, while it's available and off. */
function LoginItemRow({ state, setState }: { state: LoginItemInfo["state"]; setState: (next: LoginItemInfo["state"]) => void }): JSX.Element | null {
  const [error, setError] = useState<string | null>(null);
  if (state !== "off") return null;
  return (
    <div className="flex flex-wrap items-center gap-4 rounded-xl border border-hairline bg-surface-2 px-4 py-3" data-testid="smoother-login-item">
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="text-sm font-semibold text-ink">Open AI Juggler at login</span>
        <span className="text-meta leading-relaxed text-muted">
          So it's watching your sessions as soon as you log in. <span className="text-muted">You can turn it off in Settings › Sessions.</span>
        </span>
        {error && (
          <span className="text-xs text-danger" role="alert">
            {error}
          </span>
        )}
      </div>
      <Button
        size="sm"
        variant="secondary"
        onClick={() => {
          setError(null);
          void appLoginItemSet(true).then(
            (info) => setState(info.state),
            (cause) => setError(errorText(cause)),
          );
        }}
      >
        Turn on <span className="sr-only">opening at login</span>
      </Button>
    </div>
  );
}

export function SmootherCard({ variant }: { variant: "welcome" | "reminder" }): JSX.Element | null {
  const [entries, setEntries] = useState<OsAccessInfo[]>([]);
  const [dismissed, setDismissed] = useState<string[]>(readDismissed);
  const [loginItem, setLoginItem] = useState<LoginItemInfo["state"]>("unavailable");
  useEffect(() => {
    void osAccessList().then(setEntries, () => setEntries([]));
    if (variant === "welcome") void appLoginItemGet().then((info) => setLoginItem(info.state), () => null);
  }, [variant]);
  const missing = entries.filter((entry) => entry.status !== "allowed");
  if (variant === "welcome") {
    if ((missing.length === 0 && loginItem !== "off") || dismissed.includes("all")) return null;
    return (
      <section aria-labelledby="smoother-title" className="flex flex-col gap-3" data-testid="smoother">
        <div className="flex items-baseline justify-between gap-3">
          <SectionTitle id="smoother-title">Make it smoother</SectionTitle>
          <button type="button" className="text-meta text-muted hover:text-ink hover:underline" onClick={() => setDismissed(dismiss("all"))} data-testid="smoother-not-now">
            Not now
          </button>
        </div>
        <p className="text-meta leading-relaxed text-ink-2">Optional: AI Juggler works without them. Allow opens the system's own prompt.</p>
        {missing.length > 0 && <OsAccessRows entries={entries} setEntries={setEntries} size="compact" testIdPrefix="smoother" />}
        <LoginItemRow state={loginItem} setState={setLoginItem} />
      </section>
    );
  }
  // The welcome's "Not now" puts Home's reminder away too.
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
