/**
 * D-25, AJ-E2105; D-118 (AJ-E3701, AJ-E3702) — Home: what needs you, what
 * happened, and your apps and their room, in one glance.
 *
 * For someone with an Account, two columns under a fixed head that scroll
 * on their own (lib/home-layout): on the left, the open needs pinned on
 * top ("Needs you now", each loud once there), then the activity feed,
 * newest first, rolled up, with the away digest at the last look; on the
 * right, Your apps and Runway by account. Below 860 px of main area they stack,
 * the feed first (one page, then Show more) and the head gains "Apps &
 * runway ↓". The head's title is in ink: the pinned zone's count carries
 * the colour (D-117). No Account yet: today's welcome and first run.
 *
 * The cards are a registry (screens/home-cards). Keys (AJ-E1005 Req.3):
 * J/K or the arrows move between rows, Enter opens, A runs the row's
 * action; r, h, s and e as the ? sheet says. The ledger-degraded banner
 * sits over both columns.
 */
import { ListBuilding } from "../components/ListBuilding";
import { useKeys } from "../lib/use-keys";
import { useHomeKeys } from "../lib/use-home-keys";
import { useResource } from "../lib/use-resource";
import { instancesMarkReviewed } from "../lib/ipc/review";
import { useCallback, useEffect, useMemo, useRef, useState, type JSX, type UIEvent } from "react";
import { runwayOf } from "../components/AccountRunway";
import { useThisMacInfo, type ThisMacInfo } from "../lib/machine";
import { LegendPopover } from "../components/LegendPopover";
import { sessionLabel } from "../lib/stop";
import { RowPlaceholders } from "../components/Placeholder";
import { Welcome } from "../components/Welcome";
import { useKnownPlaces } from "../lib/places";
import { HomeFeed } from "../components/HomeFeed";
import { COLUMNS, useHomeLayout } from "../lib/home-layout";
import { openFeedItem, railRowOf } from "../lib/feed-open";
import { useActionHost } from "../components/ItemMenu";
import { usePools } from "../lib/pools";
import { useScrollMemory } from "../lib/scroll-memory";
import { shortName } from "../lib/session-titles";
import type { FeedItemInfo } from "../lib/ipc/feed";
import { headCount, needsHeading } from "../lib/home-needs";
import { HomeExtras } from "../components/HomeExtras";
import { endedToday, itemsOf, openNow } from "../lib/rail-items";
import { showInRail } from "../lib/rail-view";
import { isSnoozed } from "../lib/railview";
import { useShownConversations } from "../lib/shown-apps";
import { useSeen } from "../lib/seen";
import { homeNeedsOf, listedOnly, notSnoozed, useBackendNeeds, useRailSnoozes, useSnoozedSubjects } from "../lib/needs-you";
import { capTitle, useCapNeeds } from "../components/BudgetNeeds";
import { useMessageNeeds } from "../lib/ipc/messages";
import { useTaskNeeds } from "../lib/ipc/tasks";
import { showToast } from "../components/Toast";
import { accountLoginState } from "../lib/ipc/accounts";
import { dashboardStats } from "../lib/ipc/instances";
import { errorText, signedIn } from "../lib/bridge";
import { instanceMarkReviewed, instanceUnmarkReviewed, parkedResume } from "../lib/ipc/park";
import { instanceArchive, instanceUnarchive } from "../lib/ipc/session-history";
import { terminalSpawnLogin } from "../lib/ipc/terminal-io";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { useQuotaDisplays } from "../lib/kept-reads";
import { accountNameOf, useSubscriptions, type SubscriptionInfo } from "../lib/subscriptions";
import { HomeCards, type HomeCardContext } from "./home-cards";
import { placeOf } from "../lib/project-words";

const NEEDS = ["waiting_user", "stuck", "quota_blocked", "failed"] as const;
type Need = (typeof NEEDS)[number];
const ENDED = ["finished", "stopped", "handed_over"];
/** Ended sessions older than this are left to History (AJ-E1007). */
const TO_REVIEW_WINDOW_MS = 7 * 24 * 3_600_000;

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

/** What's wrong, in a sentence, and the one action. A limit is the
 * Account's (D-62), so it's named by the Account (`accounts`, as read);
 * a folder in none keeps its own name. */
export function needWords(need: Need, session: InstanceInfo, accounts?: readonly SubscriptionInfo[] | null): { what: string; action: string } {
  switch (need) {
    case "waiting_user":
      return { what: "Waiting for your reply", action: "Open" };
    case "stuck":
      return { what: "Looks stuck — no progress for a while", action: "Take a look" };
    case "quota_blocked":
      return { what: `${accountNameOf(accounts, session.account_id, session.account_label)} hit its limit`, action: "Continue in another app…" };
    case "failed":
      return { what: "Stopped with an error", action: "See what happened" };
  }
}

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
  thisMac: given,
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
  openLoginTerminal?: (id: string, title: string, options?: { account?: string }) => void;
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
  /** ⌘K's search (the shell's). */
  onSearch?: () => void;
}): JSX.Element {
  const thisMac = useThisMacInfo(given);
  const actionKeys = useKeys("home-action");
  // 0.0.11 walk (Q3): a tool without a limits feed says so in its row.
  // The runways kept from the last read (any screen's) show at once; read
  // again on opening, each half minute, and when an account comes or goes.
  const { displays: quota, known: quotaKnown } = useQuotaDisplays({ version: accounts.length });
  // D-62: limits are the Accounts' (one read, shared and cached).
  const subscriptions = useSubscriptions();
  // AJ-E2502, D-49: Needs you is the backend's (the bell shows the same
  // count); caps, messages and tasks keep their own rows and controls.
  const needsNow = useBackendNeeds();
  const [snooze] = useRailSnoozes();
  // AJ-E3104: the bell's count leaves snoozed ones out; so do these.
  const snoozedNow = useSnoozedSubjects(now);
  const capsRead = useCapNeeds(); // AJ-E1514
  const messagesRead = useMessageNeeds();
  const tasksRead = useTaskNeeds();
  const shownRows = useMemo(() => listedOnly(needsNow.subjects, notSnoozed(snoozedNow, { caps: capsRead.caps, messages: messagesRead, tasks: tasksRead })), [capsRead.caps, messagesRead, tasksRead, snoozedNow, needsNow.subjects]);
  const capNeeds = useMemo(() => ({ ...capsRead, caps: shownRows.caps }), [capsRead, shownRows.caps]);
  const { messages: messageNeeds, tasks: taskNeeds } = shownRows;
  const seen = useSeen();
  // AJ-E501: process tracking degraded — said here, only when it is.
  const ledgerDegraded = useResource("dashboard-stats", dashboardStats).data?.ledger_degraded === true;
  const [busy, setBusy] = useState<string | null>(null);
  const host = useActionHost();
  const pools = usePools();
  const [layout, measure] = useHomeLayout();
  const stacked = layout !== "columns";
  const feedScroll = useScrollMemory("home-feed", { active: !stacked });
  const feedBox = useRef<HTMLElement | null>(null);
  const scroller = useCallback(() => (stacked ? document.querySelector<HTMLElement>("[data-testid='home-scroll']") : feedBox.current), [stacked]);
  // Set when Mark reviewed or Resume starts; the parent's refresh can
  // remove the row a moment later, so it holds for a few seconds.
  const restoreUntil = useRef(0);
  // AJ-E2910: Mark all reviewed (every one on the list, not only those
  // shown), one call and one Undo; e archives an ended row, with Undo.
  const archive = async (session: InstanceInfo) => {
    const label = sessionLabel(session, projects);
    try {
      await instanceArchive(session.id);
      showToast({ id: `archive:${session.id}`, message: `${label}: archived — nothing deleted.`, actionBelow: true, undo: { onUndo: async () => { await instanceUnarchive(session.id); onChanged?.(); } } });
      onChanged?.();
    } catch (cause) {
      showToast({ tone: "error", message: `${label}: ${errorText(cause)}` });
    }
  };
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

  // AJ-E1005 Req.3, AJ-E2910: Home's row keys (lib/use-home-keys).
  const keys = useHomeKeys({ busy, restoreUntil, onOpenSession, onArchive: (id) => {
    const session = sessions.find((one) => one.id === id);
    if (session) void archive(session);
  } });

  // AJ-E2105: the rail's own list (its Needs you, its places), with the
  // apps turned off in Settings left out.
  const { conversations, hidden: hiddenApps } = useShownConversations();
  const known = useKnownPlaces(); // ADR-0032: Needs you names each session by the App that owns it
  const items = useMemo(
    () => itemsOf({ sessions: sessions.filter((s) => !s.parked), conversations, projects, seen, snoozed: (row) => isSnoozed(row, snooze, now), needOf: needsNow.needOf }),
    [sessions, conversations, projects, seen, snooze, now, needsNow.needOf],
  );
  const byId = useMemo(() => new Map(sessions.map((session) => [session.id, session])), [sessions]);
  // D-118: a session in the feed is named as every list names it.
  const nameOf = useCallback((item: FeedItemInfo) => {
    const session = item.target.session_id ? byId.get(item.target.session_id) : undefined;
    return session ? shortName(session, now) : null;
  }, [byId, now]);
  const parked = sessions
    .filter((s) => s.parked)
    .sort((a, b) => (a.parked_resets_at_ms ?? Number.MAX_SAFE_INTEGER) - (b.parked_resets_at_ms ?? Number.MAX_SAFE_INTEGER));
  const review = sessions
    .filter((s) => toReview(s, now))
    .sort((a, b) => (b.last_activity_ms ?? b.started_at_ms) - (a.last_activity_ms ?? a.started_at_ms));
  const reviewAll = async () => {
    const ids = review.map((session) => session.id);
    try {
      await instancesMarkReviewed(ids);
      showToast({ id: "reviewed:all", message: `Marked ${ids.length} reviewed.`, undo: { onUndo: async () => { await instancesMarkReviewed(ids, true); onChanged?.(); }, done: "They're back on the list." } });
      onChanged?.();
    } catch (cause) {
      showToast({ tone: "error", message: `Couldn't mark them reviewed: ${errorText(cause)}` });
    }
  };
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
      (terminal) => openLoginTerminal?.(terminal.id, `Sign in · ${account.label}`, { account: account.id }),
      (cause) => setSignInError(errorText(cause)),
    );
  };
  // The one runway decides, a tool that doesn't report its limits
  // included (the backend's unavailable reason, from the same capability).
  const runway = accounts.map((account) => ({
    account,
    ...runwayOf(quota.find((q) => q.account === account.id), sessions.filter((s) => s.account_id === account.id)),
  }));

  // AJ-E1503 Req.4: rows shaped like Home's while the first load answers.
  if (loading && accounts.length === 0 && sessions.length === 0) {
    return (
      <div className="mx-auto flex w-full max-w-content flex-col gap-6 pb-10" data-testid="home-loading">
        <span className="block h-7 w-64 rounded bg-skeleton motion-safe:animate-pulse" aria-hidden="true" />
        <RowPlaceholders shape="home" rows={3} label="Loading Home" />
      </div>
    );
  }

  // AJ-E1524: a new install is welcomed — what it's for, what's on this
  // Mac, the way in.
  if (accounts.length === 0) {
    return <Welcome onFindAccounts={onAddAccounts} onFreshAccount={onFreshAccount ?? onAddAccounts} />;
  }

  const allNeeds = homeNeedsOf(needsNow.rows, items, accounts, sessions, thisMac, now, known ?? []);
  // The heading counts the rows it sits over (the 0.2 walk, F4: "2
  // sessions need you" over four rows): the backend's needs Home lists,
  // and the caps, agents' messages and tasks with their own rows.
  const { sessions: sessionNeeds, others: otherNeeds, notes } = headCount(allNeeds, capNeeds.caps.length + messageNeeds.length + taskNeeds.length);
  const headline = needsHeading({ sessions: sessionNeeds, others: otherNeeds }, {
    // 0.3.2: one carrying on with its credits hasn't run out. D-62: the
    // room is the Account's, so it's named once by it, however many of
    // its folders show it.
    runOut: [...new Set(runway.filter((r) => r.left === 0 && !r.credits).map((r) => accountNameOf(subscriptions, r.account.id, r.account.label)))],
    running: items.some((item) => item.live && item.state === "working") || items.some(openNow),
    anything: sessions.length + conversations.length > 0, known: needsNow.known,
  });
  const openCount = items.filter(openNow).length;
  const sessionsWords = (count: number, what: string) => `${count} ${what} ${count === 1 ? "session" : "sessions"}`;
  const counts: { key: string; words: string; run: () => void }[] = [
    { key: "open", words: `${openCount} open`, run: () => document.querySelector<HTMLElement>("[data-testid='rail'] [data-rail-item]")?.focus() },
    { key: "review", words: `${notes} to review`, run: () => document.querySelector<HTMLElement>("[data-testid='home-needs']")?.scrollIntoView?.({ block: "nearest" }) },
    // "23 inactive" read as apps or anything (the maintainer, 0.3.3).
    { key: "inactive", words: sessionsWords(items.filter((item) => item.inactive).length, "inactive"), run: () => showInRail("inactive") },
    { key: "ended", words: `${items.filter((item) => endedToday(item, now)).length} ended today`, run: () => showInRail("ended") },
    ...hiddenApps.map((part) => ({ key: `hidden-${part.app}`, words: `${part.count} in ${part.app} hidden`, run: () => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "settings" } })) })),
  ].filter((part) => !part.words.startsWith("0 "));
  // D-118: the feed (its handlers are Home's: the session's page, Mark
  // reviewed as Home marks it, the rail's row for ↗).
  const feed = (
    <HomeFeed
      stacked={stacked}
      now={now}
      // Filled only once the needs are read and none waits (D-118: else the
      // first pinned need holds the one fill; an unread list counts as one).
      quiet={needsNow.known && allNeeds.length + capNeeds.caps.length + messageNeeds.length + taskNeeds.length === 0}
      scroller={scroller}
      nameOf={nameOf}
      facets={{ projects, accounts, subscriptions, pools }}
      toReview={review.length}
      onReviewAll={() => void reviewAll()}
      onNewSession={onNewSession}
      onOpenApp={() => document.querySelector<HTMLElement>("[data-testid='home-apps'] li button")?.focus()}
      actions={{
        open: (item) => openFeedItem(item, { host, onOpenSession, onOpenThisMac }),
        review: (id) => {
          const session = sessions.find((one) => one.id === id);
          if (session) void act(session, "reviewed");
          else void instanceMarkReviewed(id).then(() => onChanged?.(), (cause) => showToast({ tone: "warning", message: errorText(cause) }));
        },
        railOf: (item) => railRowOf(item, items),
      }}
    />
  );
  // AJ-E2018: the cards themselves are a registry (screens/home-cards).
  const cards: HomeCardContext = {
    sessions, conversations, accounts, projects, subscriptions, now, items, allNeeds, capNeeds, messageNeeds, taskNeeds,
    parked, quota, quotaKnown, ledgerDegraded, needsKnown: needsNow.known, feed, strip: layout === "strip", busy, actionKeysAria: actionKeys.aria,
    signingIn, signInError, act, startSignIn, onOpenSession, onNewSession, onLimit, onChanged, onOpenThisMac, onOpenCleanup,
  };

  // Each column's hairline once it is scrolled (`.scroll-edge`).
  const edge = (event: UIEvent<HTMLDivElement>) => event.currentTarget.toggleAttribute("data-scrolled", event.currentTarget.scrollTop > 0);
  const side = (
    <aside aria-label="Apps and runway" id="home-side" className="flex min-w-0 flex-col gap-5" data-testid="home-side" data-layout={layout}>
      <HomeCards area="right" ctx={cards} />
    </aside>
  );
  return (
    <div
      ref={(node) => {
        keys.attach(node);
        return measure(node);
      }}
      onKeyDown={keys.onKeyDown}
      onFocus={keys.onFocus}
      className={"mx-auto flex w-full max-w-content flex-col gap-4 " + (stacked ? "pb-10" : "h-full min-h-0")} data-testid="home" data-layout={layout}>
      <header className="flex shrink-0 flex-col gap-1">
        <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
          {/* D-117: the title in ink; the pinned zone's count carries the colour. */}
          <h1 className="text-title text-ink" data-testid="home-heading">{headline}</h1>
          {counts.length > 0 && (
            <p className="flex flex-wrap items-center gap-x-1.5 text-body text-ink-2" data-testid="home-counts">
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
          {/* AJ-E403 Req.6: the legend's Home surface. */}
          <LegendPopover className="shrink-0 self-center" placement="below" />
          {stacked && (
            <a href="#home-side" onClick={(event) => { event.preventDefault(); document.getElementById("home-side")?.scrollIntoView?.({ block: "start" }); }} className="ml-auto rounded text-meta text-ink-2 outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="home-side-link">
              Apps &amp; runway ↓
            </a>
          )}
        </div>
        <HomeExtras pending={!needsNow.known} shown={sessionNeeds > 0 && otherNeeds > 0} needs={allNeeds} more={{ caps: capNeeds.caps.map(capTitle), messages: messageNeeds.length, tasks: taskNeeds.length }} />
        <ListBuilding place="home" />
      </header>

      <HomeCards area="top" ctx={cards} />

      {stacked ? (
        <>
          <div className="flex min-w-0 flex-col gap-5" data-testid="home-main">
            <HomeCards area="left" ctx={cards} />
          </div>
          {side}
          <HomeCards area="foot" ctx={cards} />
        </>
      ) : (
        // Each column is `relative`: an absolutely placed child (a row's
        // sr-only words, its hover actions) then takes the column as its
        // box and is clipped there. Without it they escaped to the page's
        // scroller (pages.tsx, `relative`) and gave it a scroll of its own,
        // so a wheel moved both columns up and left a gap (CI run
        // 43857640404, home-scrolled).
        <div className={"grid min-h-0 flex-1 gap-6 " + COLUMNS} data-testid="home-columns">
          <div
            ref={(node) => {
              feedBox.current = node;
              return feedScroll(node);
            }}
            onScroll={edge}
            className="scroll-edge relative min-h-0 min-w-0 overflow-y-auto overscroll-contain pb-10 pr-1"
            data-testid="home-main"
          >
            <div className="flex w-full max-w-[720px] flex-col gap-5">
              <HomeCards area="left" ctx={cards} />
              <HomeCards area="foot" ctx={cards} />
            </div>
          </div>
          <div onScroll={edge} className="scroll-edge relative min-h-0 min-w-0 overflow-y-auto overscroll-contain pb-10 pr-1" data-testid="home-side-scroll">
            {side}
          </div>
        </div>
      )}
    </div>
  );
}
