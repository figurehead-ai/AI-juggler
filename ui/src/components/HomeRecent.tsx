/**
 * Home's "Pick up where you left off" (AJ-E1005, D-25): what ended this
 * week and isn't marked reviewed, as rows that say what each was, then
 * where and on which profile (the review of 0.1.4: two cards both said
 * "payments-api"). One nothing named takes its conversation's title (the
 * tool's name for it, else its first prompt, 0.4.5), else its telling
 * branch, else its facts — its project and when it started (the review of
 * 0.1.5; D-96, D-103.8) — and no two rows ever read the same. Each opens
 * its page; Mark reviewed takes it off. E31 (AJ-E3103): each is the one
 * session row (`row/SessionRow`), as in the rail and on every page. The
 * 0.4.6 RC walk K2 (F-K2-7a): its count is every one to review, not the
 * ones the app holds, and Show all pages every one (`lib/home-review`).
 */
import { useKeys } from "../lib/use-keys";
import type { JSX } from "react";
import type { PagedList as Paged } from "../lib/paged";
import { PagedList } from "./PagedList";
import { SectionTitle } from "./SectionTitle";
import { IconCheck } from "./icons-controls";
import { SessionRow } from "./row/SessionRow";
import { sessionItem } from "../lib/rail-items";
import { placeOf } from "../screens/Home";
import { startedWords, tellApart, untitledName } from "../lib/session-titles";
import { tellingBranch } from "../lib/project-words";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

/** Each row's title and where line, no two the same. */
export function recentRows(recent: InstanceInfo[], projects: ProjectInfo[], now: number): { session: InstanceInfo; title: string; where: string }[] {
  const rows = recent.map((session) => {
    const place = placeOf(session, projects);
    const branch = tellingBranch(session.git_branch);
    const named = session.label ?? session.title ?? branch;
    const untitled = named === null;
    const title = named ?? untitledName({ folder: place.title, startedMs: session.started_at_ms }, now);
    // The where line skips what the title already says (its branch; an
    // untitled one's facts name its place), never its Account (D-103.8).
    const byBranch = named !== null && named === branch;
    const where = [untitled ? null : place.title, byBranch ? null : session.git_branch, session.account_label].filter(Boolean).join(" · ");
    return { session, title, where, untitled };
  });
  // Never two rows that read alike: a named one adds its profile, then its
  // time; one told by its facts has its time already, and its Account is
  // its second line's (D-103.8).
  const count = (key: (row: (typeof rows)[number]) => string) => rows.reduce((seen, row) => seen.set(key(row), (seen.get(key(row)) ?? 0) + 1), new Map<string, number>());
  const titles = count((row) => row.title);
  for (const row of rows) if (!row.untitled && (titles.get(row.title) ?? 0) > 1) row.title = `${row.title} · ${row.session.account_label}`;
  const again = count((row) => row.title);
  for (const row of rows) if (!row.untitled && (again.get(row.title) ?? 0) > 1) row.title = `${row.title} · ${startedWords(row.session.started_at_ms, now)}`;
  // The same minute, folder and App: numbered, as a pool's cards are.
  const apart = tellApart(rows, (row) => row.title, (row) => [row.session.started_at_ms, row.session.id]);
  return rows.map((row) => ({ session: row.session, title: apart.get(row) ?? row.title, where: row.where }));
}

export function HomeRecent({
  recent,
  total,
  counted = true,
  paged = null,
  shown,
  showAll,
  onShowAll,
  projects,
  now,
  busy,
  onOpen,
  onReviewed,
  onReviewAll,
}: {
  recent: InstanceInfo[];
  /** How many to review: every one once counted, else the ones held. */
  total: number;
  /** `total` is every one (the backend's count): only then is it said. */
  counted?: boolean;
  /** Every one, a page at a time, when shown (the app holds only some). */
  paged?: Paged<InstanceInfo> | null;
  shown: number;
  showAll: boolean;
  onShowAll: () => void;
  projects: ProjectInfo[];
  now: number;
  busy: string | null;
  onOpen: (id: string) => void;
  onReviewed: (session: InstanceInfo) => void;
  /** AJ-E2910: all of them reviewed at once (not only those shown). */
  onReviewAll?: () => void;
}): JSX.Element | null {
  const actionKeys = useKeys("home-action");
  // The working set's rows until every one's first page is read.
  const pagedShown = paged !== null && (paged.items.length > 0 || (paged.loading && recent.length === 0));
  if (recent.length === 0 && !pagedShown) return null;
  const row = (session: InstanceInfo, title: string) => {
    const project = projects.find((one) => one.id === session.project_id)?.name ?? null;
    return (
      <SessionRow
        item={sessionItem(session, () => project, false)}
        density="list"
        now={now}
        onSelect={onOpen}
        title={title}
        slim
        data={{ "data-home-item": "", "data-testid": "home-recent-item", "data-session-id": session.id, "data-ended": "yes", "data-has-conversation": session.conversation_id ? "yes" : undefined }}
        buttonData={{ "data-home-row": "" }}
        stateSlot={
          // AJ-E1005 Req.4: it leaves this list only when you say so.
          // Every row here has ended: it stands where "Ended" would.
          <button
            type="button"
            data-home-action
            aria-keyshortcuts={actionKeys.aria}
            disabled={busy === session.id}
            onClick={() => onReviewed(session)}
            title={`Mark reviewed (${actionKeys.words})`}
            className="pointer-events-auto relative z-10 -my-1 -mr-1.5 flex min-h-6 shrink-0 items-center gap-1 rounded px-1.5 text-meta leading-[18px] text-muted outline-none hover:text-ink hover:underline focus-visible:ring-2 focus-visible:ring-accent/60 disabled:opacity-60"
          >
            {/* A narrow column (Home's beside Runway) keeps its mark;
                a wide one says it too (the 0.4 review). */}
            <IconCheck width={13} height={13} aria-hidden="true" className="@[36rem]:hidden" />
            <span className="sr-only @[36rem]:not-sr-only">Mark reviewed</span>
            <span className="sr-only"> — {title}</span>
          </button>
        }
      />
    );
  };
  const titled = (sessions: InstanceInfo[]) => new Map(recentRows(sessions, projects, now).map(({ session, title }) => [session.id, title]));
  const pagedTitles = pagedShown ? titled(paged.items) : null;
  return (
    <section aria-labelledby="home-recent" className="flex flex-col gap-2.5" data-testid="home-recent">
      <div className="flex items-center justify-between gap-2">
        <SectionTitle id="home-recent">Pick up where you left off</SectionTitle>
        {onReviewAll && total > 1 && (
          <button type="button" onClick={onReviewAll} disabled={busy !== null} className="text-meta text-muted hover:text-ink hover:underline disabled:opacity-60" data-testid="home-review-all">
            {counted ? `Mark all ${total} reviewed` : "Mark all reviewed"}
          </button>
        )}
      </div>
      {pagedShown && pagedTitles ? (
        // Every one, a page at a time, in a box of its own (never all at once).
        <PagedList list={paged} keyOf={(session) => session.id} renderRow={(session) => row(session, pagedTitles.get(session.id) ?? "")} rowHeight={52} label="Sessions to pick up" empty={null} maxHeight="min(32rem, 60vh)" plain className="overflow-hidden rounded-card border border-hairline bg-surface shadow-card" testId="home-recent-paged" />
      ) : (
        <ul className="overflow-hidden rounded-card border border-hairline bg-surface shadow-card">
          {recentRows(recent, projects, now).map(({ session, title }) => (
            <li key={session.id} className="border-t border-hairline/60 first:border-t-0">
              {row(session, title)}
            </li>
          ))}
        </ul>
      )}
      {total > shown && (
        <button
          type="button"
          className="self-start text-meta text-muted hover:text-ink hover:underline"
          aria-expanded={showAll}
          onClick={onShowAll}
          data-testid="home-recent-all"
        >
          {showAll ? "Show fewer" : counted ? `Show all ${total}` : "Show all"}
        </button>
      )}
    </section>
  );
}
