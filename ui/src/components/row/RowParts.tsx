/**
 * The parts of a session's row that are ways in of their own, inside the
 * row (RowShell): its project's name (the project's page), its subagent
 * count (its page on the Subagents tab), and a page's place line, where
 * the project and the App both show. Each reaches 24 px down from its
 * line, never up over the row's middle, where the row's own click is.
 */
import type { JSX } from "react";
import { useActionHost } from "../ItemMenu";
import { ProfileBadge } from "../ProfileBadge";
import { ProjectMark } from "../ProjectMark";
import { rememberTab } from "../../pages/registry";
import { showTab } from "../../pages/PageShell";
import { markHasSubagents } from "../../pages/session/useSubagents";
import { placeWords, type RailItem } from "../../lib/rail-items";
import { usePrimary } from "./RowShell";

const LINK =
  "pointer-events-auto relative z-10 -mb-2.5 min-h-6 truncate rounded pb-2.5 text-left text-muted outline-none hover:bg-surface-3 hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60";

/** AJ-E1520: a row's subagent count, its own way in: the session's page
 * on its Subagents tab. */
export function RowSubagents({ item, words, onSelect }: { item: RailItem; words: string; onSelect: (id: string) => void }): JSX.Element {
  const primary = usePrimary(item, onSelect);
  return (
    <span className="flex shrink-0 items-start whitespace-pre">
      <span aria-hidden="true"> · </span>
      <button
        type="button"
        data-testid="row-subagents"
        aria-label={`Open its subagents: ${words}`}
        onClick={(event) => {
          event.stopPropagation();
          // Its count says it has some: the tab is there from the start.
          const account = item.session?.account_id ?? item.conversation?.account_id;
          const conversation = item.session?.conversation_id ?? item.conversation?.id;
          if (account && conversation) markHasSubagents(account, conversation);
          rememberTab(item.session ? "session" : "conversation", "subagents");
          primary();
          showTab("subagents");
        }}
        className="pointer-events-auto relative z-10 -mx-0.5 -mb-2.5 min-h-6 shrink-0 rounded px-0.5 pb-2.5 text-left leading-[14px] text-muted outline-none hover:bg-surface-3 hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        {words}
      </button>
    </span>
  );
}

/** A row's project, its own way in: the project's page here. Text only
 * when it has no page (a folder no project holds). */
export function RowProject({ item, rest, whole = false }: { item: RailItem; rest: string; whole?: boolean }): JSX.Element {
  const host = useActionHost();
  // `whole`: the rest (a time) stays whole and the project gives way —
  // "ai-jug… · 3 h ago", never "ai-juggler · 3 …" (0.3.2). The project
  // keeps 2.5ch of the line, the time the rest. The two numbers must match
  // (and the project no side margins): the time's cap is measured against
  // the line as wide as both need.
  const fit = whole ? "min-w-[2.5ch] shrink" : "-mx-0.5 px-0.5 max-w-[65%] shrink-0";
  const id = item.projectId;
  const linked = item.project !== null && id !== null && !!host.openProjectById;
  return (
    <span className="flex min-w-0 items-start text-label leading-[14px] text-muted">
      {linked ? (
        <button
          type="button"
          data-testid="row-project"
          aria-label={`Open project ${item.project}`}
          onClick={(event) => {
            event.stopPropagation();
            host.openProjectById!(id!);
          }}
          className={`${LINK} ${fit} leading-[14px]`}
        >
          {item.project}
        </button>
      ) : (
        item.project && (
          <span aria-hidden="true" className={`${fit} truncate`}>
            {item.project}
          </span>
        )
      )}
      {rest && (
        // With no project beside it, the time has the line to itself (it
        // read "1 …" under a cap measured against a project).
        <span aria-hidden="true" className={(whole && item.project ? "min-w-0 max-w-[calc(100%-2.5ch)] shrink-0" : "min-w-0") + " truncate whitespace-pre"}>
          {item.project ? ` · ${rest}` : rest}
        </span>
      )}
    </span>
  );
}

/** The App a row is in, in words: "Claude Code · Work Max · in Terminal". */
export function appWords(item: RailItem): string {
  return placeWords(item.place).join(" · ");
}

/**
 * A page's place line (D-36, D-62): the project with its mark (its way
 * in), the App
 * with its one colour's dot (colour is the Apps'), when. The page's own
 * project or App is left out; the time never gives way.
 */
export function PlaceLine({ item, when, app, project }: { item: RailItem; when: string; app: boolean; project: boolean }): JSX.Element {
  const host = useActionHost();
  const name = project ? item.project : null;
  const id = item.projectId;
  const linked = name !== null && id !== null && !!host.openProjectById;
  const words = app ? appWords(item) : "";
  const sep = <span aria-hidden="true" className="shrink-0 whitespace-pre"> · </span>;
  return (
    <span className="flex min-w-0 flex-1 items-start text-meta leading-[18px] text-muted">
      {name !== null &&
        (linked ? (
          <button
            type="button"
            data-testid="page-session-project"
            aria-label={`Open project ${name}`}
            onClick={(event) => {
              event.stopPropagation();
              host.openProjectById!(id!);
            }}
            className={`${LINK} -mx-0.5 flex min-w-[3.5rem] max-w-[45%] shrink items-start gap-1 px-0.5 leading-[18px]`}
          >
            <span className="mt-0.5 flex shrink-0">
              <ProjectMark projectId={id} size={14} />
            </span>
            <span className="truncate">{name}</span>
          </button>
        ) : (
          <span aria-hidden="true" data-testid="page-session-project" className="flex min-w-[3.5rem] max-w-[45%] shrink items-start gap-1">
            <span className="mt-0.5 flex shrink-0">
              <ProjectMark projectId={id} size={14} />
            </span>
            <span className="truncate">{name}</span>
          </span>
        ))}
      {/* A narrow row (Home's column, a card) names the App by its own
          name beside its dot, its icon saying the app (the 0.4 review: it
          was dropped there). */}
      {words && (
        <span aria-hidden="true" className="flex min-w-[3.5rem] items-start" data-testid="page-session-app">
          {name !== null && sep}
          <span className="flex min-w-0 items-center gap-1">
            <ProfileBadge app={item.place.key} name={item.place.profile ?? item.accountLabel} size={7} />
            <span className="truncate @[26rem]:hidden">{item.place.profile ?? item.accountLabel ?? words}</span>
            <span className="hidden truncate @[26rem]:inline">{words}</span>
          </span>
          {when && name === null && sep}
        </span>
      )}
      {when && name !== null && sep}
      {when && (
        <span aria-hidden="true" className="shrink-0 whitespace-nowrap tabular-nums">
          {when}
        </span>
      )}
    </span>
  );
}
