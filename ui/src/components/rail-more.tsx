/**
 * AJ-E1554 — what an App's group says of what more there is, quietly, each
 * part one click from exactly what it counts:
 *
 * - its header's counts (R2): working, needs you (amber, its level's
 *   mark), done to review, subagents running (a branch); non-zero only,
 *   tabular, each named in its tooltip; a click opens the next of exactly
 *   those, by id (R6);
 * - its footer (R4): "+12 more · 3 recent": the open ones not drawn (its
 *   App's page, on Open now) and its recent ones (shown here, in place).
 *
 * Muted, no fills or lines at rest; hover is a tone (PAT-SURF, PAT-COLOR).
 */
import type { JSX } from "react";
import { HeadNeeds } from "./rail-groups";
import { IconGitBranch } from "./icons";
import { openSubagents } from "./row/RowParts";
import { useOpenItem } from "./row/RowShell";
import { STATE_TONE, type RailItem } from "../lib/rail-items";
import { countWords, headCounts, nextOf, type HeadCount } from "../lib/rail-counts";
import { presetListState } from "../lib/list-presets";
import { openAppSessions } from "./rail-group-app";
import type { Line } from "../lib/rail-lines";
import type { AttentionLevel } from "../lib/attention";

const COUNT =
  "flex h-6 shrink-0 items-center gap-1 rounded-md px-1 text-label tabular-nums text-muted outline-none transition-colors hover:bg-hover hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60";

/** An App header's counts (R2). */
export function HeadCounts({ items, needItems = [], level, probably, selectedId, onSelect }: { items: readonly RailItem[]; needItems?: readonly RailItem[]; level: AttentionLevel; probably: boolean; selectedId: string | null; onSelect: (id: string) => void }): JSX.Element | null {
  const open = useOpenItem(onSelect);
  const counts = headCounts(items, needItems);
  if (counts.length === 0) return null;
  const go = (count: HeadCount) => {
    const item = nextOf(count.items, selectedId);
    if (!item) return;
    if (count.kind === "subagents") openSubagents(item, () => open(item));
    else open(item);
  };
  return (
    <span className="flex shrink-0 items-center" data-testid="place-counts">
      {counts.map((count) => {
        const words = countWords(count);
        return (
          <button key={count.kind} type="button" onClick={() => go(count)} aria-label={words} title={words} data-testid={`place-count-${count.kind}`} data-count={count.n} className={COUNT}>
            {count.kind === "needs" ? (
              // D-117: its level's mark (amber, red when one broke), as before.
              <HeadNeeds needs={count.n} level={level} probably={probably} className="!px-0" />
            ) : (
              <>
                {count.kind === "working" && <span aria-hidden="true" className={"size-1.5 rounded-full " + STATE_TONE.working.dot} />}
                {count.kind === "review" && <span aria-hidden="true" className="leading-none text-success">✓</span>}
                {count.kind === "subagents" && <IconGitBranch width={11} height={11} aria-hidden="true" />}
                {count.n}
              </>
            )}
          </button>
        );
      })}
    </span>
  );
}

/** A group's footer (R4): what's left out of the rail, each part its way in. */
export function GroupFooter({
  label,
  hidden,
  words,
  total,
  recent,
  onMore,
  onRecent,
}: {
  label: string;
  /** Its open ones not drawn ("more"), or its quiet ones folded ("quiet"). */
  hidden: number;
  words: "more" | "quiet";
  /** All its open ones: what its page's Open now lists. */
  total: number;
  recent: { count: number; open: boolean } | null;
  onMore: () => void;
  onRecent: () => void;
}): JSX.Element | null {
  if (hidden === 0 && !recent) return null;
  const moreTip = words === "quiet" ? `Shows its ${hidden} quiet ${hidden === 1 ? "session" : "sessions"} here` : `Opens ${label}'s page on Open now: all ${total} open`;
  const recentTip = recent ? (recent.open ? `Hides ${label}'s recent sessions` : `Shows ${label}'s ${recent.count} recent ${recent.count === 1 ? "session" : "sessions"} here`) : "";
  return (
    <div className="ml-3 flex h-6 min-w-0 items-center px-1 text-label text-muted" data-testid="rail-group-footer">
      {hidden > 0 && (
        <button type="button" onClick={onMore} title={moreTip} aria-label={`+${hidden} ${words}: ${moreTip}`} data-testid="rail-more-quiet" className={COUNT}>
          +{hidden} {words}
        </button>
      )}
      {hidden > 0 && recent && <span aria-hidden="true">·</span>}
      {recent && (
        <button type="button" onClick={onRecent} title={recentTip} aria-expanded={recent.open} data-rail-nav data-testid="place-recent" className={COUNT}>
          {hidden > 0 ? "" : "+"}
          {recent.count} recent
        </button>
      )}
    </div>
  );
}

/** A group's footer line in the rail: "+12 more" opens its App's page (`app`)
 * on Open now, or shows them in place where it has none (and for Latest
 * activity's quiet ones); its recent ones show here, in place. */
export function RailFooter({ line, app, toggle }: { line: Extract<Line, { kind: "more" }>; app: string | null; toggle: (key: string) => void }): JSX.Element | null {
  const { group } = line;
  const onMore = app
    ? () => {
        presetListState(`app-sessions:${app}`, "open");
        openAppSessions(app);
      }
    : () => toggle(`${group.key}#all`);
  return <GroupFooter label={group.label} hidden={line.quiet} words={line.words} total={group.items.length} recent={line.recent ?? null} onMore={onMore} onRecent={() => toggle(`${group.key}#recent`)} />;
}

/** AJ-E1554 R5: folded Inactive apps say what they hold ("10 apps · 48 sessions"). */
export function idleSummary(apps: number, sessions: number): string {
  return [`${apps} ${apps === 1 ? "app" : "apps"}`, sessions > 0 ? `${sessions} ${sessions === 1 ? "session" : "sessions"}` : null].filter(Boolean).join(" · ");
}
