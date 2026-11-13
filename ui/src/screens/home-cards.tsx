/**
 * AJ-E2018 — Home's cards as a registry: each card is one entry, with the
 * area it sits in (the top, the two columns, the foot) and how it draws
 * from what Home has read. Home (screens/Home.tsx) keeps the reads, the
 * heading and the keys; a new card is one entry here.
 */
import { Fragment, type ReactNode } from "react";
import { CapNeedItems, type useCapNeeds } from "../components/BudgetNeeds";
import { HomeApps } from "../components/HomeApps";
import { HomeNeeds } from "../components/HomeNeeds";
import { HomeNext } from "../components/HomeNext";
import { HomeRunway } from "../components/HomeRunway";
import { MorningSummary } from "../components/MorningSummary";
import { ProjectMark } from "../components/ProjectMark";
import { HomeNeedsPlace } from "../components/HomeNeedsPlace";
import { fewNeedsNow } from "../lib/needs-place";
import { SectionTitle } from "../components/SectionTitle";
import { SmootherCard } from "../components/SmootherCard";
import { showToast } from "../components/Toast";
import { Button } from "../components/primitives";
import { errorText } from "../lib/bridge";
import type { HomeNeed } from "../lib/home-needs";
import type { ConversationInfo } from "../lib/ipc/conversations";
import type { MessageNeedInfo } from "../lib/ipc/messages";
import { instancePark } from "../lib/ipc/park";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import type { TaskNeedInfo } from "../lib/ipc/tasks";
import { parkedWords } from "../lib/parked";
import { placeOf } from "../lib/project-words";
import type { RailItem } from "../lib/rail-items";
import { keysOf, SHORTCUTS } from "../lib/shortcuts";
import { accountNameOf, type SubscriptionInfo } from "../lib/subscriptions";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

/** What Home has read, as its cards need it. */
export interface HomeCardContext {
  sessions: InstanceInfo[];
  conversations: ConversationInfo[];
  accounts: AccountInfo[];
  projects: ProjectInfo[];
  subscriptions: SubscriptionInfo[] | null;
  now: number;
  items: RailItem[];
  allNeeds: HomeNeed[];
  capNeeds: ReturnType<typeof useCapNeeds>;
  messageNeeds: MessageNeedInfo[];
  taskNeeds: TaskNeedInfo[];
  parked: InstanceInfo[];
  recent: InstanceInfo[];
  quota: QuotaDisplayData[];
  quotaKnown: boolean;
  ledgerDegraded: boolean;
  /** D-103.6: the needs have loaded (until then Needs you holds its place). */
  needsKnown: boolean;
  /** "Pick up where you left off", drawn by Home (it holds its state). */
  recentArea: ReactNode;
  busy: string | null;
  actionKeysAria: string | undefined;
  signingIn: string | null;
  signInError: string | null;
  runwayFor: (accountId: string | null) => { left: number | null; words: string } | null;
  act: (session: InstanceInfo, what: "resume" | "reviewed") => Promise<void>;
  startSignIn: (account: AccountInfo) => void;
  onOpenSession: (id: string) => void;
  onNewSession: () => void;
  onLimit?: (id: string) => void;
  onChanged?: () => void;
  onOpenThisMac?: () => void;
  onOpenCleanup?: () => void;
}

/** Where a card sits: above the columns, in one of them, or at the foot. */
export type HomeArea = "top" | "left" | "right" | "foot";

interface HomeCard {
  id: string;
  area: HomeArea | ((ctx: HomeCardContext) => HomeArea);
  render: (ctx: HomeCardContext) => ReactNode;
}

const needsCount = (ctx: HomeCardContext) => ctx.allNeeds.length + ctx.capNeeds.caps.length + ctx.messageNeeds.length + ctx.taskNeeds.length;

const HOME_CARDS: readonly HomeCard[] = [
  {
    id: "ledger-degraded",
    area: "top",
    render: (ctx) =>
      ctx.ledgerDegraded && (
        <div
          role="alert"
          data-testid="ledger-degraded-banner"
          className="flex items-center gap-2.5 rounded-lg border border-warning/25 bg-warning-soft px-4 py-3 text-meta text-ink"
        >
          <span aria-hidden>⚠</span>
          Process tracking degraded — cleanup is read-only until restart.
        </div>
      ),
  },
  // AJ-E1006: what changed while you were away — only when something did.
  { id: "morning", area: "top", render: (ctx) => <MorningSummary onOpenSession={ctx.onOpenSession} now={ctx.now} /> },
  {
    id: "first-session",
    area: "top",
    render: (ctx) =>
      ctx.sessions.length === 0 && ctx.conversations.length === 0 && (
        <section className="flex flex-col items-start gap-3 rounded-2xl border border-hairline bg-surface p-6" data-testid="home-first-session">
          {/* Walk A9: "first" only when it is; older sessions, out of the
              working set, are in History. */}
          <h2 className="text-section font-semibold text-ink">{ctx.projects.length > 0 ? "Start a session" : "Start your first session"}</h2>
          <p className="max-w-xl text-body text-ink-2">
            Pick a project folder and an app. AI Juggler starts Claude Code there and keeps an eye on it — when it needs you, it shows up here.
            {ctx.projects.length > 0 && " Your earlier sessions are in History."}
          </p>
          <Button variant="primary" onClick={ctx.onNewSession}>New session</Button>
        </section>
      ),
  },
  // D-97 (1): what setup left to do, until a session has run.
  { id: "next", area: "top", render: (ctx) => <HomeNext accounts={ctx.accounts} subscriptions={ctx.subscriptions} quota={ctx.quota} quotaKnown={ctx.quotaKnown} sessions={ctx.sessions} /> },
  { id: "apps", area: "top", render: (ctx) => <HomeApps items={ctx.items} runwayOf={ctx.runwayFor} /> },
  {
    id: "needs",
    area: "left",
    // D-103.6: its skeleton, sized from the last look, until the needs
    // load; gone when there are none: nothing under it moves when they arrive.
    render: (ctx) => (
      <HomeNeedsPlace known={ctx.needsKnown} rows={needsCount(ctx)}>
        <HomeNeeds
          needs={ctx.allNeeds}
          now={ctx.now}
          messages={ctx.messageNeeds}
          tasks={{ needs: ctx.taskNeeds, projects: ctx.projects }}
          handlers={{
            onOpenSession: ctx.onOpenSession,
            onLimit: ctx.onLimit,
            onWait: (id) => void instancePark(id).then(() => { showToast({ message: "Parked until its reset." }); ctx.onChanged?.(); }, (cause) => showToast({ tone: "warning", message: errorText(cause) })),
            onSignIn: (need) => need.account && ctx.startSignIn(need.account),
            onOpenThisMac: ctx.onOpenThisMac,
            onOpenCleanup: ctx.onOpenCleanup,
          }}
        >
          {ctx.signingIn && <p className="text-meta text-ink-2" role="status">Finish signing in in the terminal that opened; this clears by itself when you're done.{ctx.signInError ? ` ${ctx.signInError}` : ""}</p>}
          <CapNeedItems caps={ctx.capNeeds.caps} accounts={ctx.accounts} projects={ctx.projects} onSeen={ctx.capNeeds.reload} />
        </HomeNeeds>
      </HomeNeedsPlace>
    ),
  },
  { id: "parked", area: "left", render: (ctx) => ctx.parked.length > 0 && <ParkedCard ctx={ctx} /> },
  { id: "runway", area: "right", render: (ctx) => <HomeRunway accounts={ctx.accounts} quota={ctx.quota} quotaKnown={ctx.quotaKnown} sessions={ctx.sessions} /> },
  // Few needs: Pick up where you left off moves up into the left column
  // (the 0.4 design review, 39); before they load, as many as last time.
  { id: "recent", area: (ctx) => (fewNeedsNow(ctx.needsKnown, needsCount(ctx)) ? "left" : "right"), render: (ctx) => ctx.recentArea },
  // The optional access, once and quietly at the foot (the review of
  // 0.1.4: it took the top of Home until answered).
  { id: "smoother", area: "foot", render: () => <SmootherCard variant="reminder" /> },
  {
    id: "keys",
    area: "foot",
    render: (ctx) =>
      ctx.allNeeds.length + ctx.parked.length + ctx.recent.length > 0 && (
        <p className="text-label text-muted" data-testid="home-keys">
          {/* AJ-E1504: the same words and keys as the ? sheet. */}
          Keyboard:{" "}
          {(["home-next", "home-prev", "home-open", "home-action", "home-reply", "home-handover", "home-snooze", "home-archive", "help"] as const).map((id, index) => {
            const entry = SHORTCUTS.find((row) => row.id === id)!;
            return (
              <span key={id}>
                {index > 0 && " · "}
                <kbd className="font-mono">{keysOf(id)[0]}</kbd> {entry.label.toLowerCase()}
              </span>
            );
          })}
        </p>
      ),
  },
];

/** The cards of one area, in the registry's order. */
export function HomeCards({ area, ctx }: { area: HomeArea; ctx: HomeCardContext }): ReactNode {
  return HOME_CARDS.filter((card) => (typeof card.area === "function" ? card.area(ctx) : card.area) === area).map((card) => (
    <Fragment key={card.id}>{card.render(ctx)}</Fragment>
  ));
}

/** AJ-E1021: sessions parked until a reset, Resume once it's reached. */
function ParkedCard({ ctx }: { ctx: HomeCardContext }) {
  return (
    <section aria-labelledby="home-parked" className="flex flex-col gap-2.5" data-testid="home-parked">
      <SectionTitle id="home-parked">Parked until a reset</SectionTitle>
      <ul className="overflow-hidden rounded-xl border border-hairline bg-surface">
        {ctx.parked.map((session) => {
          const place = placeOf(session, ctx.projects);
          const words = parkedWords(session, ctx.now)!;
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
                  {accountNameOf(ctx.subscriptions, session.account_id, session.account_label)} · {words.short.replace(/^Parked · /, "")}
                </span>
              </span>
              <Button
                size="sm"
                variant={words.due ? "primary" : "secondary"}
                className="ml-auto"
                data-home-row
                aria-keyshortcuts={ctx.actionKeysAria}
                disabled={ctx.busy === session.id}
                onClick={() => (words.due ? void ctx.act(session, "resume") : ctx.onOpenSession(session.id))}
              >
                {words.due ? "Resume" : "Open"}
                <span className="sr-only"> — {place.title}</span>
              </Button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
