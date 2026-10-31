/**
 * AJ-E1520 (0.3) — a Subagents view: what's running, what failed, what
 * it cost, first. A summary line ("437 subagents · 3 running · 2 failed
 * · 41% of this session's tokens", its counts one click to their view),
 * the views (Running, Needs attention, All — it opens on Running when
 * anything runs), a search over task, prompt, result and files (`type:`
 * and `file:` narrow), a type and an order, then the grouped tree. A
 * session's tab and a project's page show the same view.
 */
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { Picker, Segmented, type PickerOption } from "../../components/controls";
import type { SubagentRowInfo, SubagentsQuery } from "../../lib/ipc/subagents";
import { SubagentTree } from "./SubagentTree";
import { useSubagentRows, type SubagentsMeta, type SubagentsSource } from "./useSubagentRows";
import { kindWords, shareWords } from "./words";

export type ViewId = "running" | "attention" | "all";
export type SortId = "newest" | "tokens" | "longest";

const SORTS: PickerOption<SortId>[] = [
  { value: "newest", label: "Newest", hint: "The latest started first" },
  { value: "tokens", label: "Most tokens", hint: "With the ones each started" },
  { value: "longest", label: "Longest", hint: "The longest running first" },
];

/** What the view was set to, kept across a subagent's page (the list
 * comes back as it was left). */
export type ViewMemory = { view?: ViewId; sort?: SortId; kind?: string; typed?: string; groups?: { open: string[]; closed: string[] } };

/** The summary line's words before its links. */
export function summaryWords(meta: Pick<SubagentsMeta, "summary">, whose: string): { all: string; share: string | null } {
  const { total, share_pct, counted } = meta.summary;
  const all = `${total.toLocaleString()} ${total === 1 ? "subagent" : "subagents"}`;
  const share = share_pct === null ? null : counted === "separate" ? `${shareWords(share_pct)} of all tokens, with ${whose}` : `${shareWords(share_pct)} of ${whose} tokens`;
  return { all, share };
}

export function SubagentsView({
  source,
  visible,
  now,
  onOpen,
  firstView,
  firstSort = "newest",
  whose = "this session's",
  height = "calc(100vh - 21rem)",
  keep,
  memory,
}: {
  source: SubagentsSource | null;
  visible: boolean;
  now: number;
  onOpen: (row: SubagentRowInfo) => void;
  /** The view it opens on (Running when anything runs). */
  firstView: ViewId;
  firstSort?: SortId;
  /** Whose tokens the share is of ("this session's"). */
  whose?: string;
  height?: string;
  /** Where the list was scrolled. */
  keep: { current: number };
  /** What it was set to, when it was shown before. */
  memory?: { current: ViewMemory };
}): JSX.Element {
  const kept = memory?.current ?? {};
  const [view, setView] = useState<ViewId>(kept.view ?? firstView);
  const [sort, setSort] = useState<SortId>(kept.sort ?? firstSort);
  const [kind, setKind] = useState<string>(kept.kind ?? "");
  const [typed, setTyped] = useState(kept.typed ?? "");
  const [search, setSearch] = useState((kept.typed ?? "").trim());
  const [groups, setGroups] = useState<{ open: string[]; closed: string[] }>(kept.groups ?? { open: [], closed: [] });
  useEffect(() => {
    if (memory) memory.current = { view, sort, kind, typed, groups };
  }, [memory, view, sort, kind, typed, groups]);
  useEffect(() => {
    const timer = window.setTimeout(() => setSearch(typed.trim()), 200);
    return () => window.clearTimeout(timer);
  }, [typed]);
  const query = useMemo<SubagentsQuery>(
    () => ({ view, search: search || null, kind: kind || null, sort, open: groups.open, closed: groups.closed }),
    [view, search, kind, sort, groups],
  );
  const { list, rows, meta } = useSubagentRows(source, query, visible);
  const onGroup = (key: string, open: boolean) =>
    setGroups((now) => ({
      open: open ? [...now.open.filter((k) => k !== key), key] : now.open.filter((k) => k !== key),
      closed: open ? now.closed.filter((k) => k !== key) : [...now.closed.filter((k) => k !== key), key],
    }));
  const summary = meta ? summaryWords(meta, whose) : null;
  const running = meta?.summary.running ?? 0;
  const attention = meta?.summary.attention ?? 0;
  const types: PickerOption<string>[] = useMemo(
    () => [{ value: "", label: "All types" }, ...(meta?.kinds ?? []).map((k) => ({ value: k.kind, label: kindWords(k.kind) ?? k.kind, hint: `${k.count} ${k.count === 1 ? "subagent" : "subagents"}` }))],
    [meta?.kinds],
  );
  const searchBox = useRef<HTMLInputElement | null>(null);
  const filtered = view !== "all" || !!search || !!kind;
  return (
    <div className="flex w-full min-w-0 flex-col gap-3" data-testid="subagents-view">
      {summary && (
        <p className="flex flex-wrap items-baseline gap-x-1.5 gap-y-1 text-body" data-testid="subagents-summary">
          <span className="font-semibold text-ink">{summary.all}</span>
          {running > 0 && (
            <>
              <span aria-hidden="true" className="text-muted">·</span>
              <button type="button" onClick={() => setView("running")} className="rounded text-live underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="subagents-summary-running">
                {running} running
              </button>
            </>
          )}
          {(meta?.summary.failed ?? 0) > 0 && (
            <>
              <span aria-hidden="true" className="text-muted">·</span>
              <button type="button" onClick={() => setView("attention")} className="rounded text-danger underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="subagents-summary-failed">
                {meta!.summary.failed} failed
              </button>
            </>
          )}
          {summary.share && (
            <>
              <span aria-hidden="true" className="text-muted">·</span>
              <span className="text-muted" title={meta?.summary.tokens ? `${meta.summary.tokens.toLocaleString()} tokens: part of the session's total, not added to it` : undefined}>
                {summary.share}
              </span>
            </>
          )}
          {meta?.reading && (
            <span className="ml-1 text-meta text-muted" role="status" data-testid="subagents-reading">
              · still reading a long session, more may appear
            </span>
          )}
        </p>
      )}
      {/* Nothing to filter, search or sort: no bar of dead controls. */}
      {!(meta && meta.state === "listed" && meta.summary.total === 0) && (
      <div className="flex flex-wrap items-center gap-2" data-testid="subagents-toolbar">
        <Segmented<ViewId>
          className="h-9 w-[23rem] max-w-full shrink-0"
          label="Which subagents"
          value={view}
          onChange={setView}
          testId="subagents-views"
          segments={[
            { value: "running", label: running > 0 ? `Running ${running}` : "Running" },
            { value: "attention", label: attention > 0 ? `Needs attention ${attention}` : "Needs attention", title: "Failed, stopped, or how it ended isn't on record" },
            { value: "all", label: "All" },
          ]}
        />
        <input
          ref={searchBox}
          type="search"
          value={typed}
          onChange={(event) => setTyped(event.target.value)}
          placeholder="Search task, prompt, result, file:"
          aria-label="Search subagents (type: and file: narrow to one)"
          className="h-9 min-w-[12rem] max-w-80 flex-1 rounded-xl border border-hairline bg-surface-2 px-3 text-body text-ink placeholder:text-muted focus:border-accent/60 focus:outline-none"
          data-testid="subagents-search"
        />
        <div className="ml-auto flex items-center gap-2">
          <Picker<string> value={kind} options={types} onChange={setKind} ariaLabel="Type" prefix="Type:" heading="Type" testId="subagents-type" disabled={types.length < 3 && !kind} reason="They are all one type." />
          <Picker<SortId> value={sort} options={SORTS} onChange={setSort} ariaLabel="Sort" prefix="Sort:" heading="Sort by" testId="subagents-sort" />
        </div>
      </div>
      )}
      {list.loading && rows.length === 0 ? (
        <div className="flex flex-col gap-1.5" role="status" aria-label="Loading its subagents" data-testid="subagents-loading">
          {[0, 1, 2, 3].map((row) => (
            <span key={row} className="h-11 rounded-lg bg-skeleton motion-safe:animate-pulse" />
          ))}
        </div>
      ) : meta && meta.state !== "listed" ? (
        <p className="text-body text-ink-2" data-testid="subagents-unlisted" data-state={meta.state}>
          {meta.reason ?? "There's no record of these subagents on this computer."}
        </p>
      ) : meta && meta.summary.total === 0 ? (
        <p className="text-body text-ink-2" data-testid="subagents-empty">
          {whose === "this session's" ? "This session hasn't started any subagents." : "None of these sessions has started a subagent."}
        </p>
      ) : rows.length === 0 && filtered ? (
        <div className="flex flex-col items-start gap-2 rounded-card border border-dashed border-hairline px-4 py-6" data-testid="subagents-none-match">
          <p className="text-body text-ink">{view === "running" && !search && !kind ? "Nothing is running now." : view === "attention" && !search && !kind ? "Nothing needs your attention." : "No subagent matches."}</p>
          <button type="button" className="rounded text-meta text-accent-hi underline-offset-2 hover:underline" onClick={() => (setView("all"), setTyped(""), setKind(""))}>
            Show all {meta?.summary.total.toLocaleString()}
          </button>
        </div>
      ) : (
        <SubagentTree rows={rows} label="Subagents" now={now} onOpen={onOpen} onGroup={onGroup} hasMore={list.hasMore} onNearEnd={list.loadMore} height={height} keep={keep} />
      )}
      {meta?.conversations_note && (
        <p className="text-meta text-ink-2" data-testid="subagents-note">
          {meta.conversations_note}
        </p>
      )}
      {meta && meta.summary.tokens === null && meta.summary.total > 0 && (
        <p className="text-meta text-ink-2">Tokens per subagent: not recorded by {meta.tool || "its tool"}.</p>
      )}
      <span className="sr-only" aria-live="polite">
        {meta && filtered ? `${meta.matched} ${meta.matched === 1 ? "subagent matches" : "subagents match"}` : ""}
      </span>
    </div>
  );
}
