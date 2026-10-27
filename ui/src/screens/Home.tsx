/**
 * D-25, AJ-E2105 — Home: what needs you, what is open where, and the room
 * left, in one glance.
 *
 * 1. **The heading** is one sentence about what needs you ("2 things need
 *    you"), with the other counts under it as links that turn the rail's
 *    views on (open, inactive, ended today; an app turned off in Settings).
 * 2. **Your apps**: one tile per running app and profile (a place): its
 *    sessions, a needs-you badge, its account's runway, Open.
 * 3. **Needs you** holds every kind of item, each with its own primary:
 *    a session asking (its page; ↗ to its app), a limit reached (Continue elsewhere, or
 *    Wait), a sign-in, a Mac working hard, processes left behind.
 * 4. **Runway by account**: each account's room left and its profiles,
 *    or the way to measure it. Unmeasured says so, never a guess.
 *
 * This Mac's line is the top bar's; the rail lists what is open.
 *
 * Plus "Pick up where you left off". Empty means "start here": no
 * Account → bring them in; no session → start one.
 *
 * D-25 amendment (AJ-E1021): Home is also the morning triage. Sessions
 * parked until a reset get their own section (Resume once the reset is
 * reached) instead of reading as "hit its limit"; "Pick up where you
 * left off" lists what ended this week and isn't marked reviewed yet;
 * J/K or the arrows move between rows, Enter opens, A runs the row's
 * action. The ledger-degraded banner moved here from Overview.
 */
import { ProjectMark } from "../components/ProjectMark";
import { useCallback, useEffect, useMemo, useRef, useState, type JSX, type KeyboardEvent } from "react";
import { Button } from "../components/primitives";
import { MorningSummary } from "../components/MorningSummary";
import { newWorkAdvice, runwayOf } from "../components/AccountRunway";
import type { ThisMacInfo } from "../lib/machine";
import { LegendPopover } from "../components/LegendPopover";
import { parkedWords } from "../lib/parked";
import { sessionLabel } from "../lib/stop";
import { matches, SHORTCUTS, type KeyLike } from "../lib/shortcuts";
import { RowPlaceholders } from "../components/Placeholder";
import { Welcome } from "../components/Welcome";
import { SectionTitle } from "../components/SectionTitle";
import { SmootherCard } from "../components/SmootherCard";

import { HomeApps } from "../components/HomeApps";
import { HomeRecent } from "../components/HomeRecent";
import { HomeNeeds } from "../components/HomeNeeds";
import { HomeRunway } from "../components/HomeRunway";
import { isSessionNeed, needsHeading } from "../lib/home-needs";
import { endedToday, itemsOf, openNow } from "../lib/rail-items";
import { showInRail } from "../lib/rail-view";
import { isSnoozed } from "../lib/railview";
import { useShownConversations } from "../lib/shown-apps";
import { useSeen } from "../lib/seen";
import { homeNeedsOf, useBackendNeeds, useRailSnoozes } from "../lib/needs-you";
import { useCapNeeds } from "../components/BudgetNeeds";
import { useMessageNeeds } from "../lib/ipc/messages";
import { useTaskNeeds } from "../lib/ipc/tasks";
import { showToast } from "../components/Toast";
import {
  accountLoginState,
  dashboardStats,
  errorText,
  instancePark,
  instanceMarkReviewed, instanceUnmarkReviewed,
  parkedResume,
  quotaDisplayAll,
  signedIn,
  terminalSpawnLogin,
  type AccountInfo, type InstanceInfo, type ProjectInfo, type QuotaDisplayData,
} from "../lib/bridge";
import { CapNeedItems } from "../components/BudgetNeeds";

const NEEDS = ["waiting_user", "stuck", "quota_blocked", "failed"] as const;
type Need = (typeof NEEDS)[number];
const ENDED = ["finished", "stopped", "handed_over"];
/** Ended sessions older than this are left to History (AJ-E1007). */
const TO_REVIEW_WINDOW_MS = 7 * 24 * 3_600_000;
/** "Pick up where you left off" shows this many, the rest on request. */
const RECENT_SHOWN = 3;

/** Ended this week (not handed over) and not yet marked reviewed or archived. */
export function toReview(session: InstanceInfo, now: number): boolean {
  if (!ENDED.includes(session.status)) return false;
  // A handed-over session goes on elsewhere; its successor is the one to
  // pick up (as Triage had it).
  if (session.status === "handed_over") return false;
  if (session.lifecycle === "reviewed" || session.lifecycle === "archived") return false;
  return now - (session.last_activity_ms ?? session.started_at_ms) <= TO_REVIEW_WINDOW_MS;
}

/** A session's need, from the status or the attention flag. */
export function needOf(session: InstanceInfo): Need | null {
  if ((NEEDS as readonly string[]).includes(session.status)) return session.status as Need;
  return (session.needs_attention as Need | null) ?? null;
}


/** What's wrong, in a sentence, and the one action. */
export function needWords(need: Need, session: InstanceInfo): { what: string; action: string } {
  switch (need) {
    case "waiting_user":
      return { what: "Waiting for your reply", action: "Open" };
    case "stuck":
      return { what: "Looks stuck — no progress for a while", action: "Take a look" };
    case "quota_blocked":
      return { what: `${session.account_label} hit its limit`, action: "Continue elsewhere" };
    case "failed":
      return { what: "Stopped with an error", action: "See what happened" };
  }
}


import { placeOf } from "../lib/project-words";
export { placeOf };


export function HomeScreen({
  sessions,
  accounts,
  projects,
  now,
  onOpenSession,
  onNewSession,
  onAddAccounts,
  onFreshAccount,
  onLimit,
  openLoginTerminal,
  onAccountsChanged,
  onChanged,
  thisMac = null,
  onOpenCleanup,
  onOpenThisMac,
  loading = false,
}: {
  /** AJ-E1503: the first load hasn't answered yet (placeholders). */
  loading?: boolean;
  sessions: InstanceInfo[];
  accounts: AccountInfo[];
  projects: ProjectInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onNewSession: () => void;
  onAddAccounts: () => void;
  /** AJ-E1524: the welcome's "Add a fresh account" (first run otherwise). */
  onFreshAccount?: () => void;
  /** D-25: a session stopped by its limit opens the decision dialog. */
  onLimit?: (id: string) => void;
  /** D-25: an account that needs its sign-in is a "Needs you" item; its
   * Sign in opens the vendor's own login in an embedded terminal. */
  openLoginTerminal?: (id: string, title: string) => void;
  /** Called when a sign-in started here completes (the app refreshes). */
  onAccountsChanged?: () => void;
  /** A row's action changed a session (resumed, marked reviewed). */
  onChanged?: () => void;
  /** E14: This Mac (the app's one subscription; `null` before the first sample). */
  thisMac?: ThisMacInfo | null;
  /** E14: opens Cleanup (This Mac's leftovers line). */
  onOpenCleanup?: () => void;
  /** AJ-E1528: the This Mac page. */
  onOpenThisMac?: () => void;
}): JSX.Element {
  const [quota, setQuota] = useState<QuotaDisplayData[]>([]);
  // AJ-E2502, D-49: Needs you is the backend's (the bell shows the same
  // count); caps, messages and tasks keep their own rows and controls.
  const needsNow = useBackendNeeds();
  const [snooze] = useRailSnoozes();
  const capNeeds = useCapNeeds(); // AJ-E1514
  const messageNeeds = useMessageNeeds();
  const taskNeeds = useTaskNeeds();
  const seen = useSeen();
  // 0.0.11 walk (Q3): a tool without a limits feed says so in its row.
  useEffect(() => {
    let alive = true;
    const load = () => void quotaDisplayAll().then((rows) => alive && setQuota(rows), () => null);
    load();
    const timer = window.setInterval(load, 30_000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [accounts.length]);

  // AJ-E501: process tracking degraded — said here, only when it is.
  const [ledgerDegraded, setLedgerDegraded] = useState(false);
  useEffect(() => {
    let alive = true;
    void dashboardStats().then((stats) => alive && setLedgerDegraded(stats.ledger_degraded === true), () => null);
    return () => {
      alive = false;
    };
  }, []);
  const [busy, setBusy] = useState<string | null>(null);
  const [showAll, setShowAll] = useState(false);
  // Set when Mark reviewed or Resume starts; the parent's refresh can
  // remove the row a moment later, so it holds for a few seconds.
  const restoreUntil = useRef(0);
  const act = useCallback(
    async (session: InstanceInfo, what: "resume" | "reviewed") => {
      restoreUntil.current = Date.now() + 3000;
      setBusy(session.id);
      try {
        const label = sessionLabel(session, projects);
        if (what === "resume") {
          await parkedResume(session.id);
          showToast({ message: `Resumed ${label}.` });
        } else {
          await instanceMarkReviewed(session.id);
          // AJ-E1503: Undo puts it back on the list.
          showToast({
            id: `reviewed:${session.id}`,
            message: `Marked ${label} reviewed.`,
            undo: {
              onUndo: async () => {
                await instanceUnmarkReviewed(session.id);
                onChanged?.();
              },
              done: `${label} is back on the list.`,
            },
          });
        }
        onChanged?.();
      } catch (cause) {
        showToast({ tone: "warning", message: `${session.account_label}: ${errorText(cause)}` });
      } finally {
        setBusy(null);
      }
    },
    [onChanged, projects],
  );

  // AJ-E1005 Req.3 on Home: J/K or the arrows move between rows, Enter
  // opens (each row's first control is a button), A runs the row's action
  // — also from the row's own action button (Mark reviewed).
  const rootRef = useRef<HTMLDivElement | null>(null);
  const rowsNow = () => Array.from(rootRef.current?.querySelectorAll<HTMLElement>("[data-home-row]") ?? []);
  const rowIndexOf = (el: Element | null): number => {
    const item = el?.closest("[data-home-item]");
    return item ? rowsNow().findIndex((row) => row.closest("[data-home-item]") === item) : -1;
  };
  // AJ-E1504: the keys come from the registry (lib/shortcuts.ts), with
  // its typing rule: never while typing in a field or the terminal.
  const handleKey = (event: KeyLike & { target: EventTarget | null; preventDefault: () => void }) => {
    if ((event.target as HTMLElement | null)?.tagName === "SELECT") return;
    const rows = rowsNow();
    const at = rowIndexOf(document.activeElement);
    const focusRow = (index: number) => rows[Math.max(0, Math.min(rows.length - 1, index))]?.focus();
    // The arrows move only once a row has focus (J and K start it).
    const arrow = event.key.startsWith("Arrow");
    if (matches("home-next", event) && (!arrow || at >= 0)) {
      event.preventDefault();
      focusRow(at + 1);
    } else if (matches("home-prev", event) && (!arrow || at >= 0)) {
      event.preventDefault();
      focusRow(at < 0 ? 0 : at - 1);
    } else if (matches("home-action", event) && at >= 0) {
      const item = rows[at]!.closest("[data-home-item]");
      const action = item?.querySelector<HTMLElement>("[data-home-action]") ?? rows[at]!;
      event.preventDefault();
      action.click();
    }
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => handleKey(event);
  // Marking a row reviewed or resuming it removes or disables it, and its
  // focus falls to <body>: put focus back on the row now at that place,
  // and keep the keys working from <body> while Home is shown.
  const lastRow = useRef<number | null>(null);
  useEffect(() => {
    if (busy !== null || Date.now() > restoreUntil.current || lastRow.current === null) return;
    if (document.activeElement !== document.body) return;
    const rows = rowsNow();
    if (rows.length > 0) rows[Math.min(lastRow.current, rows.length - 1)]!.focus();
  });
  useEffect(() => {
    const fromBody = (event: globalThis.KeyboardEvent) => {
      // Never behind a dialog: a modal over Home (the limit dialog, the
      // usage-stats question) owns the keys, even when a click on its
      // text dropped focus to <body>.
      if (event.defaultPrevented || document.querySelector('[aria-modal="true"]')) return;
      if (document.activeElement === document.body) handleKey(event);
    };
    window.addEventListener("keydown", fromBody);
    return () => window.removeEventListener("keydown", fromBody);
  });

  // AJ-E2105: the rail's own list (its Needs you, its places), with the
  // apps turned off in Settings left out.
  const { conversations, hidden: hiddenApps } = useShownConversations();
  const items = useMemo(
    () => itemsOf({ sessions: sessions.filter((s) => !s.parked), conversations, projects, seen, snoozed: (row) => isSnoozed(row, snooze, now), needOf: needsNow.needOf }),
    [sessions, conversations, projects, seen, snooze, now, needsNow.needOf],
  );
  const parked = sessions
    .filter((s) => s.parked)
    .sort((a, b) => (a.parked_resets_at_ms ?? Number.MAX_SAFE_INTEGER) - (b.parked_resets_at_ms ?? Number.MAX_SAFE_INTEGER));
  const review = sessions
    .filter((s) => toReview(s, now))
    .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms));
  // D-25: Sign in opens the Provider's own sign-in straight away (first run's
  // rule); the row then says where to finish it.
  const [signingIn, setSigningIn] = useState<string | null>(null);
  const [signInError, setSignInError] = useState<string | null>(null);
  // Every 2 s while waiting (the login-queue rule): the row clears the
  // moment the sign-in completes, even if its terminal stays open.
  useEffect(() => {
    if (!signingIn) return;
    let alive = true;
    const timer = window.setInterval(() => {
      void accountLoginState(signingIn).then(
        (state) => {
          if (alive && signedIn(state)) {
            setSigningIn(null);
            onAccountsChanged?.();
          }
        },
        () => null,
      );
    }, 2000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [signingIn, onAccountsChanged]);
  const startSignIn = (account: AccountInfo) => {
    setSignInError(null);
    setSigningIn(account.id);
    void terminalSpawnLogin(account.id).then(
      (terminal) => openLoginTerminal?.(terminal.id, `Sign in · ${account.label}`),
      (cause) => setSignInError(errorText(cause)),
    );
  };
  const recent = showAll ? review : review.slice(0, RECENT_SHOWN);
  // The one runway decides, a tool that doesn't report its limits
  // included (the backend's unavailable reason, from the same capability).
  const runway = accounts.map((account) => ({
    account,
    ...runwayOf(quota.find((q) => q.account === account.id), sessions.filter((s) => s.account_id === account.id)),
  }));
  const advice = newWorkAdvice(accounts, quota, sessions);

  // AJ-E1503 Req.4: rows shaped like Home's while the first load answers.
  if (loading && accounts.length === 0 && sessions.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-content flex-col gap-6 pb-10" data-testid="home-loading">
        <span className="block h-7 w-64 rounded bg-surface-3 motion-safe:animate-pulse" aria-hidden="true" />
        <RowPlaceholders shape="home" rows={3} label="Loading Home" />
      </div>
    );
  }

  // AJ-E1524: a new install is welcomed — what it's for, what's on this
  // Mac, the way in.
  if (accounts.length === 0) {
    return <Welcome onFindAccounts={onAddAccounts} onFreshAccount={onFreshAccount ?? onAddAccounts} />;
  }

  const allNeeds = homeNeedsOf(needsNow.rows, items, accounts, sessions, thisMac, now);
  // E18, E19: agents' messages and tasks that wait on the user count with
  // the others; AJ-E2502: the total is the bell's count.
  const sessionNeeds = allNeeds.filter(isSessionNeed).length;
  const headline = needsHeading({ sessions: sessionNeeds, others: Math.max(0, needsNow.count - sessionNeeds) }, {
    runOut: runway.filter((r) => r.left === 0).map((r) => r.account.label),
    running: items.some((item) => item.live && item.state === "working") || items.some(openNow),
    anything: sessions.length + conversations.length > 0,
  });
  const openCount = items.filter(openNow).length;
  const counts: { key: string; words: string; run: () => void }[] = [
    { key: "open", words: `${openCount} open`, run: () => document.querySelector<HTMLElement>("[data-testid='rail'] [data-rail-item]")?.focus() },
    { key: "inactive", words: `${items.filter((item) => item.inactive).length} inactive`, run: () => showInRail("inactive") },
    { key: "ended", words: `${items.filter((item) => endedToday(item, now)).length} ended today`, run: () => showInRail("ended") },
    ...hiddenApps.map((part) => ({ key: `hidden-${part.app}`, words: `${part.count} in ${part.app} hidden`, run: () => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "settings" } })) })),
  ].filter((part) => !part.words.startsWith("0 "));
  const runwayFor = (accountId: string | null) => {
    const row = runway.find((r) => r.account.id === accountId);
    return row ? { left: row.left, words: row.left !== null ? `${row.account.label} · ${row.left}% left` : `${row.account.label} · not measured yet` } : null;
  };

  return (
    <div
      ref={rootRef}
      onKeyDown={onKeyDown}
      onFocus={(event) => {
        const index = rowIndexOf(event.target as Element);
        if (index >= 0) lastRow.current = index;
      }}
      className="mx-auto flex w-full max-w-content flex-col gap-6 pb-10" data-testid="home">
      <header className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <h1 className="text-title text-ink" data-testid="home-heading">{headline}</h1>
          {/* AJ-E403 Req.6: the legend's Home surface. */}
          <LegendPopover className="shrink-0" placement="below" />
        </div>
        {counts.length > 0 && (
          <p className="flex flex-wrap items-center gap-x-1.5 text-body text-muted" data-testid="home-counts">
            {counts.map((part, index) => (
              <span key={part.key} className="flex items-center gap-1.5">
                {index > 0 && <span aria-hidden="true">·</span>}
                <button type="button" onClick={part.run} className="rounded outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
                  {part.words}
                </button>
              </span>
            ))}
          </p>
        )}
      </header>

      {ledgerDegraded && (
        <div
          role="alert"
          data-testid="ledger-degraded-banner"
          className="flex items-center gap-2.5 rounded-lg border border-warning/25 bg-warning-soft px-4 py-3 text-meta text-ink"
        >
          <span aria-hidden>⚠</span>
          Process tracking degraded — cleanup is read-only until restart.
        </div>
      )}

      {/* AJ-E1006: what changed while you were away — only when something did. */}
      <MorningSummary onOpenSession={onOpenSession} />

      {sessions.length === 0 && conversations.length === 0 && (
        <section className="flex flex-col items-start gap-3 rounded-2xl border border-hairline bg-surface p-6" data-testid="home-first-session">
          <h2 className="text-section font-semibold text-ink">Start your first session</h2>
          <p className="max-w-xl text-body text-muted">
            Pick a project folder and an account. AI Juggler starts Claude Code there and keeps an eye on it — when it needs you, it shows up here.
          </p>
          <Button variant="primary" onClick={onNewSession}>New session</Button>
        </section>
      )}

      <HomeApps items={items} runwayOf={runwayFor} />

      <div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-[minmax(0,3fr)_minmax(320px,2fr)]">
        <div className="flex min-w-0 flex-col gap-6">
          {allNeeds.length + capNeeds.caps.length + messageNeeds.length + taskNeeds.length > 0 && (
          <HomeNeeds
            needs={allNeeds}
            messages={messageNeeds}
            tasks={{ needs: taskNeeds, projects }}
            handlers={{
              onOpenSession,
              onLimit,
              onWait: (id) => void instancePark(id).then(() => { showToast({ message: "Parked until its reset." }); onChanged?.(); }, (cause) => showToast({ tone: "warning", message: errorText(cause) })),
              onSignIn: (need) => need.account && startSignIn(need.account),
              onOpenThisMac,
              onOpenCleanup,
            }}
          >
            {signingIn && <p className="text-meta text-muted" role="status">Finish signing in in the terminal that opened; this clears by itself when you're done.{signInError ? ` ${signInError}` : ""}</p>}
            <CapNeedItems caps={capNeeds.caps} accounts={accounts} projects={projects} onSeen={capNeeds.reload} />
          </HomeNeeds>
          )}
          {parked.length > 0 && (
            <section aria-labelledby="home-parked" className="flex flex-col gap-2.5" data-testid="home-parked">
              <SectionTitle id="home-parked">Parked until a reset</SectionTitle>
              <ul className="overflow-hidden rounded-xl border border-hairline bg-surface">
                {parked.map((session) => {
                  const place = placeOf(session, projects);
                  const words = parkedWords(session, now)!;
                  return (
                    <li key={session.id} data-home-item className="flex flex-wrap items-center gap-x-3.5 gap-y-2 border-t border-hairline/60 px-4 py-2.5 first:border-t-0">
                      <span aria-hidden="true" className="w-4 shrink-0 text-center text-xs text-muted">⏸</span>
                      <ProjectMark projectId={session.project_id} size={16} />
                      <span className="flex min-w-0 flex-1 basis-40 flex-col gap-0.5">
                        <span className="truncate text-body text-ink">
                          {place.title}
                          {session.git_branch && <span className="text-muted"> · {session.git_branch}</span>}
                        </span>
                        <span className="truncate text-meta text-muted" title={words.long}>
                          {session.account_label} · {words.short.replace(/^Parked · /, "")}
                        </span>
                      </span>
                      <Button
                        size="sm"
                        variant={words.due ? "primary" : "secondary"}
                        className="ml-auto"
                        data-home-row
                        aria-keyshortcuts="A"
                        disabled={busy === session.id}
                        onClick={() => (words.due ? void act(session, "resume") : onOpenSession(session.id))}
                      >
                        {words.due ? "Resume" : "Open"}
                        <span className="sr-only"> — {place.title}</span>
                      </Button>
                    </li>
                  );
                })}
              </ul>
            </section>
          )}
        </div>
        <div className="flex min-w-0 flex-col gap-6">
          <HomeRunway accounts={accounts} quota={quota} sessions={sessions} advice={advice.words} />
        <HomeRecent recent={recent} total={review.length} shown={RECENT_SHOWN} showAll={showAll} onShowAll={() => setShowAll((open) => !open)} projects={projects} now={now} busy={busy} onOpen={onOpenSession} onReviewed={(session) => void act(session, "reviewed")} />
        </div>
      </div>

      {/* The optional access, once and quietly at the foot (the review of
          0.1.4: it took the top of Home until answered). */}
      <SmootherCard variant="reminder" />

      {allNeeds.length + parked.length + recent.length > 0 && (
        <p className="text-label text-muted" data-testid="home-keys">
          {/* AJ-E1504: the same words and keys as the ? sheet. */}
          Keyboard:{" "}
          {(["home-next", "home-prev", "home-open", "home-action", "help"] as const).map((id, index) => {
            const entry = SHORTCUTS.find((row) => row.id === id)!;
            return (
              <span key={id}>
                {index > 0 && " · "}
                <kbd className="font-mono">{entry.keys[0]}</kbd> {entry.label.toLowerCase()}
              </span>
            );
          })}
        </p>
      )}
    </div>
  );
}
