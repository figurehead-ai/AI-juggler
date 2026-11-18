/**
 * AJ-E2018 — Home's cards as a registry: each card is one entry, with the
 * area it sits in and how it draws from what Home has read. Home
 * (screens/Home.tsx) keeps the reads, the heading and the keys; a new card
 * is one entry here.
 *
 * D-118 (AJ-E3701): the areas are the two columns. `top` sits over both
 * (a banner); `left` is the feed's column: the pinned zone ("Needs you
 * now", L2 and L3 needs, each loud once there), sessions parked until a
 * reset, then the feed; `right` is Your apps, Runway by account, then
 * Next steps (the tools setup left out); `foot` closes the feed's column.
 * The morning summary became the feed's away digest, and "Pick up where
 * you left off" its Finished filter (D-118 point 3).
 */
import { Fragment, type ReactNode } from "react";
import { CapNeedItems, type useCapNeeds } from "../components/BudgetNeeds";
import { HomeApps } from "../components/HomeApps";
import { HomeNeeds } from "../components/HomeNeeds";
import { HomeNext } from "../components/HomeNext";
import { HomeRunway } from "../components/HomeRunway";
import { ProjectMark } from "../components/ProjectMark";
import { HomeNeedsPlace } from "../components/HomeNeedsPlace";
import { HomeSection } from "../components/HomeSection";
import { Button } from "../components/primitives";
import { SmootherCard } from "../components/SmootherCard";
import { showToast } from "../components/Toast";
import { errorText } from "../lib/bridge";
import { homeNeedLevel, type HomeNeed } from "../lib/home-needs";
import { keptShape, NEEDS_PLACE, useKeepShape } from "../lib/needs-place";
import { brokenOf, counted, highest } from "../lib/attention";
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
  quota: QuotaDisplayData[];
  quotaKnown: boolean;
  ledgerDegraded: boolean;
  /** D-103.6: the needs have loaded (until then Needs you holds its place). */
  needsKnown: boolean;
  /** The activity feed, drawn by Home (it holds its handlers). */
  feed: ReactNode;
  /** Under 640 px: Your apps as a strip of icons (AJ-E3701 R4). */
  strip: boolean;
  busy: string | null;
  actionKeysAria: string | undefined;
  signingIn: string | null;
  signInError: string | null;
  act: (session: InstanceInfo, what: "resume" | "reviewed") => Promise<void>;
  startSignIn: (account: AccountInfo) => void;
  onOpenSession: (id: string) => void;
  onNewSession: () => void;
  onLimit?: (id: string) => void;
  onChanged?: () => void;
  onOpenThisMac?: () => void;
  onOpenCleanup?: () => void;
}

/** Where a card sits: above the columns, in one of them, or at the feed's foot. */
export type HomeArea = "top" | "left" | "right" | "foot";

interface HomeCard {
  id: string;
  area: HomeArea | ((ctx: HomeCardContext) => HomeArea);
  render: (ctx: HomeCardContext) => ReactNode;
}

/** The pinned needs: L2 and L3 (D-118 R2; "To review" is the feed's
 * Finished filter, leftovers the top bar's Clean up). */
function pinnedNeeds(ctx: Pick<HomeCardContext, "allNeeds">): HomeNeed[] {
  return ctx.allNeeds.filter((need) => counted(homeNeedLevel(need)));
}

/** The zone's count: the rail's and the bell's (AJ-E3602): every pinned
 * need, cap, agent's message and task that waits on you. */
const needsCount = (ctx: HomeCardContext) => pinnedNeeds(ctx).length + ctx.capNeeds.caps.length + ctx.messageNeeds.length + ctx.taskNeeds.length;

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
  {
    id: "needs",
    area: "left",
    // D-103.6: its skeleton, sized from the last look, until the needs
    // load: nothing under it moves when they arrive.
    render: (ctx) => <PinnedZone ctx={ctx} />,
  },
  { id: "parked", area: "left", render: (ctx) => ctx.parked.length > 0 && <ParkedCard ctx={ctx} /> },
  { id: "feed", area: "left", render: (ctx) => ctx.feed },
  { id: "apps", area: "right", render: (ctx) => <HomeApps items={ctx.items} accounts={ctx.accounts} strip={ctx.strip} /> },
  { id: "runway", area: "right", render: (ctx) => <HomeRunway accounts={ctx.accounts} quota={ctx.quota} quotaKnown={ctx.quotaKnown} sessions={ctx.sessions} /> },
  // D-97 (1): the tools setup found and left out, until a session has run;
  // last, under the runway (D-117: setup never above what needs you).
  { id: "next", area: "right", render: (ctx) => <HomeNext accounts={ctx.accounts} sessions={ctx.sessions} /> },
  // The optional access, once and quietly at the foot (the review of
  // 0.1.4: it took the top of Home until answered).
  { id: "smoother", area: "foot", render: () => <SmootherCard variant="reminder" /> },
  {
    id: "keys",
    area: "foot",
    render: (ctx) =>
      ctx.allNeeds.length + ctx.parked.length > 0 && (
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

/** D-118 R2: "Needs you now", pinned over the feed: every open L2 and L3
 * need, loud once here (the count carries its level; the first need has
 * the feed's one filled primary), never filtered. Nothing waiting, no
 * zone at all: the title already says "All clear" (2026-11-16). Before
 * the needs load, the zone holds its place only if it showed last time
 * (D-103.6), so an empty one never flashes in and out. */
function PinnedZone({ ctx }: { ctx: HomeCardContext }) {
  const pinned = pinnedNeeds(ctx);
  const count = needsCount(ctx);
  const empty = count === 0 && !ctx.signingIn;
  // Kept here: the place itself isn't drawn when there's nothing.
  useKeepShape(NEEDS_PLACE, ctx.needsKnown && empty, 0);
  if (ctx.needsKnown ? empty : keptShape(NEEDS_PLACE)?.rows === 0) return null;
  // Agents' messages, tasks and caps wait on you too: level 2.
  const level = highest([...pinned.map(homeNeedLevel), ...(count > pinned.length ? [2 as const] : [])]);
  return (
    <HomeSection id="home-needs-now" label="Needs you now" count={count} level={level} broken={brokenOf(pinned, homeNeedLevel, (need) => (need.kind === "session" ? need.item?.need : need.kind))} region>
      <HomeNeedsPlace known={ctx.needsKnown} rows={count + (ctx.signingIn ? 1 : 0)}>
        <HomeNeeds
          needs={pinned}
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
    </HomeSection>
  );
}

/** AJ-E1021: sessions parked until a reset, Resume once it's reached. */
function ParkedCard({ ctx }: { ctx: HomeCardContext }) {
  return (
    <section aria-labelledby="home-parked" className="flex flex-col gap-2.5" data-testid="home-parked">
      <h2 id="home-parked" className="section-label">Parked until a reset</h2>
      <ul className="overflow-hidden rounded-xl bg-surface">
        {ctx.parked.map((session) => {
          const place = placeOf(session, ctx.projects);
          const words = parkedWords(session, ctx.now)!;
          return (
            <li key={session.id} data-home-item className="flex flex-wrap items-center gap-x-3.5 gap-y-2 px-4 py-2.5">
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
