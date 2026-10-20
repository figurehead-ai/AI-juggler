/**
 * D-25 — Home: the three questions, every glance.
 *
 * 1. **What needs me?** Sessions waiting for a reply, stuck, blocked by
 *    a limit or failed — each in one plain sentence with the one action
 *    that moves it on.
 * 2. **How much runway?** Per Account: one bar (the tighter of the
 *    5-hour and weekly windows, as measured), when it resets, and the
 *    best Account for new work. Unmeasured says so — never a guess.
 * 3. **What's running where?** Quiet, by project and Account.
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
import { useCallback, useEffect, useMemo, useRef, useState, type JSX, type KeyboardEvent } from "react";
import { Button } from "../components/primitives";
import { hueOf } from "../components/indicator";
import { MorningSummary } from "../components/MorningSummary";
import { ThisMacPanel } from "../components/ThisMacPanel";
import { alertSubject, reasonSentence, type ThisMacInfo } from "../lib/machine";
import { LegendPopover } from "../components/LegendPopover";
import { parkedWords } from "../lib/parked";
import {
  accountLoginState,
  dashboardStats,
  errorText,
  instanceMarkReviewed,
  parkedResume,
  quotaDisplayAll,
  signedIn,
  terminalSpawnLogin,
  type AccountInfo,
  type InstanceInfo,
  type ProjectInfo,
  type QuotaDisplayData,
  type WindowDisplay,
} from "../lib/bridge";

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

function minutes(sinceMs: number, now: number): string {
  const m = Math.max(0, Math.round((now - sinceMs) / 60_000));
  if (m < 1) return "just now";
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return h < 24 ? `${h} h` : `${Math.floor(h / 24)} d`;
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

const NEED_GLYPH: Record<Need, { glyph: string; tone: string; tint: string }> = {
  waiting_user: { glyph: "!", tone: "text-warning", tint: "bg-warning-soft" },
  stuck: { glyph: "?", tone: "text-warning", tint: "bg-warning-soft" },
  quota_blocked: { glyph: "◷", tone: "text-danger", tint: "bg-danger-soft" },
  failed: { glyph: "✕", tone: "text-danger", tint: "bg-danger-soft" },
};

/** A session's place: project name (or folder) and branch. */
export function placeOf(session: InstanceInfo, projects: ProjectInfo[]): { title: string; detail: string } {
  const project = projects.find((p) => p.id === session.project_id);
  const cwd = session.cwd ?? "";
  const folder = cwd.replace(/\/+$/, "").split("/").pop() || cwd || session.account_label;
  const title = project?.name ?? folder;
  const detail = [session.git_branch, session.account_label].filter(Boolean).join(" · ");
  return { title, detail };
}

/** Runway for one Account: % left of the tighter measured window. */
export function runwayOf(
  display: QuotaDisplayData | undefined,
  sessions: InstanceInfo[] = [],
): {
  left: number | null;
  words: string;
  resets: string | null;
} {
  const fromAccount = accountRunway(display);
  if (fromAccount.left !== null) return fromAccount;
  // No account-level reading yet: a session's own quota line is still a
  // real reading (the newest one wins), never a guess.
  const reading = sessions
    .filter((s) => s.quota !== null)
    .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms))[0]?.quota;
  if (!reading) return fromAccount;
  const left = Math.max(0, Math.round(100 - reading.used_percentage));
  return {
    left,
    words: `${left}% left, from its latest session`,
    resets: reading.resets_at_ms
      ? `Resets ${new Date(reading.resets_at_ms).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" })}`
      : null,
  };
}

function accountRunway(display: QuotaDisplayData | undefined): {
  left: number | null;
  words: string;
  resets: string | null;
} {
  const windows: Array<[string, WindowDisplay | undefined]> = [
    ["5-hour window", display?.five_hour],
    ["week", display?.weekly],
  ];
  const measured = windows.filter(([, w]) => w && w.percentage !== undefined && w.state !== "unknown");
  if (measured.length === 0) {
    const learning = windows.some(([, w]) => w?.state === "learning");
    return { left: null, words: learning ? "Measuring — shows after a little use" : "Not measured yet", resets: null };
  }
  const tightest = measured.reduce((a, b) => ((a[1]!.percentage ?? 0) >= (b[1]!.percentage ?? 0) ? a : b));
  const left = Math.max(0, Math.round(100 - (tightest[1]!.percentage ?? 0)));
  const resetsAt = tightest[1]!.resets_at_ms;
  const resets = resetsAt
    ? `${tightest[0] === "week" ? "Week" : "5-hour window"} resets ${new Date(resetsAt).toLocaleString([], { weekday: "short", hour: "2-digit", minute: "2-digit" })}`
    : null;
  return { left, words: `${left}% left ${tightest[0] === "week" ? "this week" : "in the 5-hour window"}`, resets };
}

function SectionTitle({ id, children }: { id: string; children: string }): JSX.Element {
  return (
    <h2 id={id} className="text-[11px] font-semibold uppercase tracking-wider text-muted">
      {children}
    </h2>
  );
}

function AccountDot({ id }: { id: string | null }): JSX.Element {
  return <span aria-hidden="true" className="size-2 shrink-0 rounded-full" style={{ background: `hsl(${hueOf(id)} 70% 66%)` }} />;
}

export function HomeScreen({
  sessions,
  accounts,
  projects,
  now,
  onOpenSession,
  onNewSession,
  onAddAccounts,
  onLimit,
  openLoginTerminal,
  onAccountsChanged,
  onChanged,
  thisMac = null,
  onOpenCleanup,
}: {
  sessions: InstanceInfo[];
  accounts: AccountInfo[];
  projects: ProjectInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onNewSession: () => void;
  onAddAccounts: () => void;
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
}): JSX.Element {
  const [quota, setQuota] = useState<QuotaDisplayData[]>([]);
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
  const [status, setStatus] = useState("");
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
        if (what === "resume") await parkedResume(session.id);
        else await instanceMarkReviewed(session.id);
        setStatus(`${session.account_label}: ${what === "resume" ? "resumed" : "marked reviewed"}.`);
        onChanged?.();
      } catch (cause) {
        setStatus(`${session.account_label}: ${errorText(cause)}`);
      } finally {
        setBusy(null);
      }
    },
    [onChanged],
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
  const handleKey = (event: { key: string; metaKey: boolean; ctrlKey: boolean; altKey: boolean; target: EventTarget | null; preventDefault: () => void }) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target as HTMLElement | null;
    if (target && (target.tagName === "SELECT" || target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;
    const rows = rowsNow();
    const at = rowIndexOf(document.activeElement);
    const focusRow = (index: number) => rows[Math.max(0, Math.min(rows.length - 1, index))]?.focus();
    if (event.key === "j" || (event.key === "ArrowDown" && at >= 0)) {
      event.preventDefault();
      focusRow(at + 1);
    } else if (event.key === "k" || (event.key === "ArrowUp" && at >= 0)) {
      event.preventDefault();
      focusRow(at < 0 ? 0 : at - 1);
    } else if ((event.key === "a" || event.key === "A") && at >= 0) {
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

  const needs = useMemo(
    () =>
      sessions
        // A parked session already has its decision: it waits for the reset.
        .filter((session) => !session.parked)
        .map((session) => ({ session, need: needOf(session) }))
        .filter((row): row is { session: InstanceInfo; need: Need } => row.need !== null)
        .sort((a, b) => NEEDS.indexOf(a.need) - NEEDS.indexOf(b.need)),
    [sessions],
  );
  const running = sessions.filter((s) => (s.status === "running" || s.status === "starting") && needOf(s) === null && !s.parked);
  const parked = sessions
    .filter((s) => s.parked)
    .sort((a, b) => (a.parked_resets_at_ms ?? Number.MAX_SAFE_INTEGER) - (b.parked_resets_at_ms ?? Number.MAX_SAFE_INTEGER));
  const review = sessions
    .filter((s) => toReview(s, now))
    .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms));
  // E14 (AJ-E1404 Req.3): a session hogging the Mac needs you too — unless
  // it is already listed for another reason — and so does a Mac under
  // pressure. Tools AI Juggler didn't start are shown in This Mac only.
  const listed = new Set(needs.map((row) => row.session.id));
  const heavy = (thisMac?.heavy ?? []).filter(
    (tree): tree is typeof tree & { session_id: string } =>
      tree.session_id !== null && !listed.has(tree.session_id) && sessions.some((s) => s.id === tree.session_id),
  );
  const heavyIds = new Set(heavy.map((tree) => tree.session_id));
  const calmRunning = running.filter((session) => !heavyIds.has(session.id));
  // AJ-E1408: the Mac's own overload alerts (sustained pressure, swap
  // thrash, a busy or hot Mac, a disk filling) — or, before any has held
  // long enough, a Mac already under pressure.
  const machineAlerts = (thisMac?.alerts ?? []).filter((alert) => alert.kind !== "heavy");
  const nameOfSession = (id: string) => {
    const session = sessions.find((s) => s.id === id);
    return session ? placeOf(session, projects).title : null;
  };
  const macNeed: { title: string; detail: string; severe: boolean } | null = !thisMac?.machine.sampled
    ? null
    : machineAlerts.length > 0
      ? {
          title: machineAlerts[0]!.title,
          detail: [reasonSentence(machineAlerts.map((alert) => alert.detail)), alertSubject(machineAlerts[0]!, nameOfSession)]
            .filter(Boolean)
            .join(". "),
          severe: thisMac.machine.summary.level === "pressure",
        }
      : thisMac.machine.summary.level === "pressure"
        ? {
            title: "Your Mac is under pressure",
            detail: reasonSentence(thisMac.machine.summary.reasons.slice(0, 2).map((reason) => reason.words)),
            severe: true,
          }
        : null;
  const signIns = accounts.filter((account) => !signedIn(account.login_state));
  // D-25: Sign in opens Claude's own sign-in straight away (first run's
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
  const runway = accounts.map((account) => ({
    account,
    ...runwayOf(quota.find((q) => q.account === account.id), sessions.filter((s) => s.account_id === account.id)),
  }));
  const best = runway.filter((r) => r.left !== null).sort((a, b) => (b.left ?? 0) - (a.left ?? 0))[0];
  const low = runway.find((r) => r.left !== null && r.left <= 25);

  if (accounts.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-2xl flex-col items-start gap-4 py-16" data-testid="home-empty-accounts">
        <h1 className="text-2xl font-semibold tracking-tight">Bring in your accounts to start.</h1>
        <p className="text-[15px] leading-relaxed text-muted">
          AI Juggler finds the Claude Code setups already on this computer and brings them in as copies; your originals stay as they are.
        </p>
        <Button variant="primary" onClick={onAddAccounts}>Find my accounts</Button>
      </div>
    );
  }

  const sessionNeeds = needs.length + heavy.length;
  const headline =
    sessionNeeds > 0
      ? sessionNeeds === 1
        ? "1 session needs you"
        : `${sessionNeeds} sessions need you`
      : macNeed
        ? macNeed.title
      : signIns.length > 0
        ? signIns.length === 1
          ? `${signIns[0]!.label} needs its sign-in`
          : `${signIns.length} accounts need their sign-in`
      : running.length > 0
        ? "All clear — nothing needs you"
        : sessions.length === 0
          ? "Ready when you are"
          : "All clear";
  const subline = [
    running.length > 0 ? `${running.length} running` : null,
    low ? `${low.account.label} has ${low.left}% left — start new work on ${best && best.account.id !== low.account.id ? best.account.label : "another account"}` : best ? `Most runway: ${best.account.label}` : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <div
      ref={rootRef}
      onKeyDown={onKeyDown}
      onFocus={(event) => {
        const index = rowIndexOf(event.target as Element);
        if (index >= 0) lastRow.current = index;
      }}
      className="mx-auto flex w-full max-w-5xl flex-col gap-7 py-2" data-testid="home">
      <header className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <h1 className="text-2xl font-semibold tracking-tight">{headline}</h1>
          {/* AJ-E403 Req.6: the legend's Home surface (the "All clear"
              explainer), the same panel as the rail footer's "?". */}
          <LegendPopover className="shrink-0" placement="below" />
        </div>
        {subline && <p className="text-sm text-muted">{subline}</p>}
      </header>
      <p className="sr-only" aria-live="polite" data-testid="home-status">
        {status}
      </p>

      {ledgerDegraded && (
        <div
          role="alert"
          data-testid="ledger-degraded-banner"
          className="flex items-center gap-2.5 rounded-lg border border-warning/25 bg-warning-soft px-4 py-3 text-[12.5px] text-ink"
        >
          <span aria-hidden>⚠</span>
          Process tracking degraded — cleanup is read-only until restart.
        </div>
      )}

      {/* AJ-E1006: what changed while you were away — only when something did. */}
      <MorningSummary onOpenSession={onOpenSession} />

      {sessions.length === 0 && (
        <section className="flex flex-col items-start gap-3 rounded-2xl border border-hairline bg-surface p-6" data-testid="home-first-session">
          <h2 className="text-lg font-semibold">Start your first session</h2>
          <p className="max-w-xl text-sm leading-relaxed text-muted">
            Pick a project folder and an account. AI Juggler starts Claude Code there and keeps an eye on it — when it needs you, it shows up here.
          </p>
          <Button variant="primary" onClick={onNewSession}>New session</Button>
        </section>
      )}

      <div className="flex flex-wrap gap-6">
        <div className="flex min-w-0 flex-[3_1_420px] flex-col gap-6">
          {needs.length + signIns.length + heavy.length + (macNeed ? 1 : 0) > 0 && (
            <section aria-labelledby="home-needs" className="flex flex-col gap-2.5" data-testid="home-needs">
              <SectionTitle id="home-needs">Needs you</SectionTitle>
              <ul className="flex flex-col gap-2.5">
                {signIns.map((account) => (
                  <li key={`signin-${account.id}`} className="flex flex-col gap-3 rounded-xl border border-hairline bg-surface px-4 py-3.5" data-testid="home-signin">
                    <span className="flex flex-wrap items-center gap-x-3.5 gap-y-2">
                      <span aria-hidden="true" className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-accent-soft font-bold text-accent-hi">
                        →
                      </span>
                      <span className="flex min-w-0 flex-1 basis-40 flex-col gap-0.5">
                        <span className="text-[15px] font-medium text-ink">{account.label} needs its sign-in</span>
                        <span className="text-xs text-muted" role="status">
                          {signingIn === account.id
                            ? "Finish signing in in the terminal that opened — this clears by itself when you're done."
                            : "Claude's own sign-in, in a terminal here — about thirty seconds. AI Juggler never sees your password."}
                        </span>
                        {signingIn === account.id && signInError && <span className="text-xs text-danger">{signInError}</span>}
                      </span>
                      <Button size="sm" variant={signingIn === account.id ? "ghost" : "secondary"} className="ml-auto" onClick={() => startSignIn(account)}>
                        {signingIn === account.id ? "Open again" : "Sign in"}{" "}
                        <span className="sr-only">— {account.label}</span>
                      </Button>
                    </span>
                  </li>
                ))}
                {macNeed && (
                  <li className="flex flex-wrap items-center gap-x-3.5 gap-y-2 rounded-xl border border-hairline bg-surface px-4 py-3.5" data-testid="home-mac-pressure">
                    <span
                      aria-hidden="true"
                      className={
                        "flex size-8 shrink-0 items-center justify-center rounded-lg font-bold " +
                        (macNeed.severe ? "bg-danger-soft text-danger" : "bg-warning-soft text-warning")
                      }
                    >
                      {macNeed.severe ? "■" : "▲"}
                    </span>
                    <span className="flex min-w-0 flex-1 basis-40 flex-col gap-0.5">
                      <span className="text-[15px] font-medium text-ink">{macNeed.title}</span>
                      <span className="text-xs text-muted">{macNeed.detail}</span>
                    </span>
                    <Button
                      size="sm"
                      variant="secondary"
                      className="ml-auto"
                      onClick={() => {
                        const panel = document.getElementById("this-mac");
                        panel?.scrollIntoView({ block: "start" });
                        panel?.focus({ preventScroll: true });
                      }}
                    >
                      See what's using it
                    </Button>
                  </li>
                )}
                {needs.map(({ session, need }) => {
                  const words = needWords(need, session);
                  const glyph = NEED_GLYPH[need];
                  const place = placeOf(session, projects);
                  return (
                    <li key={session.id} data-home-item className="flex flex-wrap items-center gap-x-3.5 gap-y-2 rounded-xl border border-hairline bg-surface px-4 py-3.5">
                      <span aria-hidden="true" className={`flex size-8 shrink-0 items-center justify-center rounded-lg font-bold ${glyph.tint} ${glyph.tone}`}>
                        {glyph.glyph}
                      </span>
                      <span className="flex min-w-0 flex-1 basis-40 flex-col gap-0.5">
                        <span className="text-[15px] font-medium text-ink">{words.what}</span>
                        <span className="truncate text-xs text-muted">
                          {place.title}
                          {place.detail ? ` · ${place.detail}` : ""}
                          {` · ${minutes(session.attention?.since_ms ?? session.last_activity_ms ?? session.started_at_ms, now)}`}
                        </span>
                      </span>
                      <Button
                        size="sm"
                        variant="secondary"
                        className="ml-auto"
                        data-home-row
                        aria-keyshortcuts="A"
                        onClick={() => (need === "quota_blocked" && onLimit ? onLimit(session.id) : onOpenSession(session.id))}
                      >
                        {words.action}{" "}
                        <span className="sr-only">— {place.title}</span>
                      </Button>
                    </li>
                  );
                })}
                {heavy.map((tree) => {
                  const session = sessions.find((s) => s.id === tree.session_id)!;
                  const place = placeOf(session, projects);
                  return (
                    <li key={`heavy-${tree.session_id}`} className="flex flex-wrap items-center gap-x-3.5 gap-y-2 rounded-xl border border-hairline bg-surface px-4 py-3.5" data-testid="home-heavy">
                      <span aria-hidden="true" className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-warning-soft font-bold text-warning">
                        ▲
                      </span>
                      <span className="flex min-w-0 flex-1 basis-40 flex-col gap-0.5">
                        <span className="text-[15px] font-medium text-ink">Working your Mac hard — {tree.heavy_note}</span>
                        <span className="truncate text-xs text-muted">
                          {place.title}
                          {place.detail ? ` · ${place.detail}` : ""}
                          {tree.helpers > 0 ? ` · ${tree.helpers === 1 ? "1 helper" : `${tree.helpers} helpers`}` : ""}
                        </span>
                      </span>
                      <Button size="sm" variant="secondary" className="ml-auto" onClick={() => onOpenSession(tree.session_id)}>
                        Take a look <span className="sr-only">— {place.title}</span>
                      </Button>
                    </li>
                  );
                })}
              </ul>
            </section>
          )}

          {calmRunning.length > 0 && (
            <section aria-labelledby="home-running" className="flex flex-col gap-2.5" data-testid="home-running">
              <SectionTitle id="home-running">Running</SectionTitle>
              <ul className="overflow-hidden rounded-xl border border-hairline bg-surface">
                {calmRunning.map((session) => {
                  const place = placeOf(session, projects);
                  return (
                    <li key={session.id} data-home-item className="border-t border-hairline/60 first:border-t-0">
                      <button
                        type="button"
                        data-home-row
                        className="flex w-full items-center gap-3.5 px-4 py-2.5 text-left hover:bg-surface-2"
                        onClick={() => onOpenSession(session.id)}
                      >
                        <span aria-hidden="true" className="size-2 shrink-0 rounded-full bg-live shadow-[0_0_0_4px_var(--color-live-soft)]" />
                        <span className="min-w-0 flex-1 truncate text-sm text-ink">
                          {place.title}
                          {session.git_branch && <span className="text-faint"> · {session.git_branch}</span>}
                        </span>
                        <span className="flex items-center gap-1.5 text-xs text-muted">
                          <AccountDot id={session.account_id} />
                          {session.account_label}
                        </span>
                        <span className="w-14 text-right font-mono text-xs text-faint">{minutes(session.started_at_ms, now)}</span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            </section>
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
                      <span className="flex min-w-0 flex-1 basis-40 flex-col gap-0.5">
                        <span className="truncate text-sm text-ink">
                          {place.title}
                          {session.git_branch && <span className="text-faint"> · {session.git_branch}</span>}
                        </span>
                        <span className="truncate text-xs text-muted" title={words.long}>
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

        <div className="flex min-w-0 flex-[2_1_280px] flex-col gap-6">
        <section aria-labelledby="home-runway" className="flex min-w-0 flex-col gap-2.5" data-testid="home-runway">
          <SectionTitle id="home-runway">Runway</SectionTitle>
          <div className="flex flex-col rounded-xl border border-hairline bg-surface px-4 py-1.5">
            {runway.map((row) => (
              <div key={row.account.id} className="flex flex-col gap-1.5 border-t border-hairline/60 py-3 first:border-t-0" data-testid="home-runway-account">
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span className="flex min-w-0 items-center gap-2 font-medium text-ink">
                    <AccountDot id={row.account.id} />
                    <span className="truncate">{row.account.label}</span>
                  </span>
                  <span className={"shrink-0 font-mono text-xs " + (row.left !== null && row.left <= 25 ? "text-warning" : "text-muted")}>
                    {row.left !== null ? `${row.left}% left` : "—"}
                  </span>
                </div>
                {row.left !== null ? (
                  <div
                    className="h-1.5 overflow-hidden rounded-full bg-surface-3"
                    role="meter"
                    aria-valuemin={0}
                    aria-valuemax={100}
                    aria-valuenow={row.left}
                    aria-label={`${row.account.label}: ${row.words}`}
                  >
                    <div
                      className={"h-full rounded-full " + (row.left <= 25 ? "bg-warning" : "bg-accent")}
                      style={{ width: `${row.left}%` }}
                    />
                  </div>
                ) : null}
                <span className="text-[11px] text-faint">{row.left !== null ? (row.resets ?? row.words) : row.words}</span>
              </div>
            ))}
            {best && runway.length > 1 && (
              <p className="border-t border-hairline/60 py-3 text-xs text-muted">
                Best for new work: <span className="font-semibold text-ink">{best.account.label}</span>, {best.left}% left.
              </p>
            )}
          </div>
        </section>
        <ThisMacPanel
          info={thisMac}
          nameOf={(id) => {
            const session = sessions.find((s) => s.id === id);
            return session ? placeOf(session, projects).title : "A session";
          }}
          onOpenSession={onOpenSession}
          onOpenCleanup={onOpenCleanup}
        />
        </div>
      </div>

      {recent.length > 0 && (
        <section aria-labelledby="home-recent" className="flex flex-col gap-2.5" data-testid="home-recent">
          <SectionTitle id="home-recent">Pick up where you left off</SectionTitle>
          <ul className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
            {recent.map((session) => {
              const place = placeOf(session, projects);
              return (
                <li key={session.id} data-home-item className="flex flex-col rounded-xl border border-hairline bg-surface hover:border-accent/40" data-testid="home-recent-item">
                  <button
                    type="button"
                    data-home-row
                    className="flex w-full flex-col items-start gap-1.5 rounded-t-xl px-4 pb-1.5 pt-3 text-left"
                    onClick={() => onOpenSession(session.id)}
                  >
                    <span className="w-full truncate text-sm font-medium text-ink">{place.title}</span>
                    <span className="w-full truncate text-xs text-faint">
                      {place.detail} · {minutes(session.last_activity_ms ?? session.started_at_ms, now)} ago
                    </span>
                    <span className="text-xs text-accent-hi">Open →</span>
                  </button>
                  {/* AJ-E1005 Req.4: it leaves this list only when you say so. */}
                  <button
                    type="button"
                    data-home-action
                    aria-keyshortcuts="A"
                    disabled={busy === session.id}
                    onClick={() => void act(session, "reviewed")}
                    className="self-end px-4 pb-2.5 text-[11px] text-muted hover:text-ink hover:underline disabled:opacity-60"
                  >
                    Mark reviewed<span className="sr-only"> — {place.title}</span>
                  </button>
                </li>
              );
            })}
          </ul>
          {review.length > RECENT_SHOWN && (
            <button
              type="button"
              className="self-start text-xs text-muted hover:text-ink hover:underline"
              aria-expanded={showAll}
              onClick={() => setShowAll((open) => !open)}
              data-testid="home-recent-all"
            >
              {showAll ? "Show fewer" : `Show all ${review.length}`}
            </button>
          )}
        </section>
      )}

      {needs.length + running.length + parked.length + recent.length > 0 && (
        <p className="text-[11px] text-faint" data-testid="home-keys">
          Keyboard: <kbd className="font-mono">J</kbd> / <kbd className="font-mono">K</kbd> move between sessions ·{" "}
          <kbd className="font-mono">Enter</kbd> opens · <kbd className="font-mono">A</kbd> runs the row's action
        </p>
      )}
    </div>
  );
}
