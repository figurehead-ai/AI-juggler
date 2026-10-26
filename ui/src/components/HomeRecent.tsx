/**
 * Home's "Pick up where you left off" (AJ-E1005, D-25): what ended this
 * week and isn't marked reviewed, as rows that say what each was, then
 * where and on which profile (the review of 0.1.4: two cards both said
 * "payments-api"). One nothing named takes the first words of the last
 * thing you asked it, else its branch or project and when it started
 * (the review of 0.1.5), and no two rows ever read the same. Each opens
 * its page; Mark reviewed takes it off.
 */
import type { JSX } from "react";
import { ProjectMark } from "./ProjectMark";
import { SectionTitle } from "./SectionTitle";
import { placeOf } from "../screens/Home";
import { minutes } from "../lib/time-words";
import { askedKey, startedWords, untitledName, useAskedWords } from "../lib/session-titles";
import { tellingBranch } from "../lib/project-words";
import type { InstanceInfo, ProjectInfo } from "../lib/bridge";

/** Each row's title and where line, no two the same. */
export function recentRows(recent: InstanceInfo[], projects: ProjectInfo[], asked: ReadonlyMap<string, string | null>, now: number): { session: InstanceInfo; title: string; where: string }[] {
  const rows = recent.map((session) => {
    const place = placeOf(session, projects);
    const words = session.conversation_id ? asked.get(askedKey(session.account_id ?? "", session.conversation_id)) : null;
    const branch = tellingBranch(session.git_branch);
    const named = session.label ?? session.title ?? words ?? null;
    const title = named ?? untitledName(branch, place.title, session.started_at_ms, now);
    // The where line skips what the title already says.
    const where = [named === null && !branch ? null : place.title, named === null && branch ? null : session.git_branch, session.account_label].filter(Boolean).join(" · ");
    return { session, title, where };
  });
  // Never two rows that read alike: the profile, then the time, tells them apart.
  const count = (key: (row: (typeof rows)[number]) => string) => rows.reduce((seen, row) => seen.set(key(row), (seen.get(key(row)) ?? 0) + 1), new Map<string, number>());
  const titles = count((row) => row.title);
  for (const row of rows) if ((titles.get(row.title) ?? 0) > 1) row.title = `${row.title} · ${row.session.account_label}`;
  const again = count((row) => row.title);
  for (const row of rows) if ((again.get(row.title) ?? 0) > 1) row.title = `${row.title} · ${startedWords(row.session.started_at_ms, now)}`;
  return rows;
}

export function HomeRecent({
  recent,
  total,
  shown,
  showAll,
  onShowAll,
  projects,
  now,
  busy,
  onOpen,
  onReviewed,
}: {
  recent: InstanceInfo[];
  total: number;
  shown: number;
  showAll: boolean;
  onShowAll: () => void;
  projects: ProjectInfo[];
  now: number;
  busy: string | null;
  onOpen: (id: string) => void;
  onReviewed: (session: InstanceInfo) => void;
}): JSX.Element | null {
  const asked = useAskedWords(
    recent.flatMap((session) => (!session.label && !session.title && session.conversation_id ? [{ profileId: session.account_id ?? "", conversationId: session.conversation_id }] : [])),
  );
  if (recent.length === 0) return null;
  return (
    <section aria-labelledby="home-recent" className="flex flex-col gap-2.5" data-testid="home-recent">
      <SectionTitle id="home-recent">Pick up where you left off</SectionTitle>
      <ul className="overflow-hidden rounded-card border border-hairline bg-surface shadow-card">
        {recentRows(recent, projects, asked, now).map(({ session, title, where }) => {
          return (
            <li key={session.id} data-home-item className="flex items-center gap-3 border-t border-hairline/60 px-4 py-3 first:border-t-0 hover:bg-surface-2/60" data-testid="home-recent-item">
              <ProjectMark projectId={session.project_id} size={28} />
              <button type="button" data-home-row className="flex min-w-0 flex-1 flex-col items-start gap-0.5 rounded text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/60" onClick={() => onOpen(session.id)}>
                <span className="w-full truncate text-body font-medium text-ink">{title}</span>
                <span className="w-full truncate text-meta text-muted">
                  {where} · {minutes(session.last_activity_ms ?? session.started_at_ms, now)} ago
                </span>
              </button>
              {/* AJ-E1005 Req.4: it leaves this list only when you say so. */}
              <button
                type="button"
                data-home-action
                aria-keyshortcuts="A"
                disabled={busy === session.id}
                onClick={() => onReviewed(session)}
                className="shrink-0 rounded px-1.5 py-1 text-meta text-muted outline-none hover:text-ink hover:underline focus-visible:ring-2 focus-visible:ring-accent/60 disabled:opacity-60"
              >
                Mark reviewed<span className="sr-only"> — {title}</span>
              </button>
            </li>
          );
        })}
      </ul>
      {total > shown && (
        <button
          type="button"
          className="self-start text-meta text-muted hover:text-ink hover:underline"
          aria-expanded={showAll}
          onClick={onShowAll}
          data-testid="home-recent-all"
        >
          {showAll ? "Show fewer" : `Show all ${total}`}
        </button>
      )}
    </section>
  );
}
