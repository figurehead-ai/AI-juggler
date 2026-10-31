/**
 * AJ-E1520 (0.3) — one subagent's own page: breadcrumbs that stay put
 * (Session › Subagents › its parents › it), a header (its task, state,
 * type, model, whether it ran in the background, who started it), then
 * what it was given and answered, its work (tool calls, lines, files,
 * the subagents it started), its usage and cost, the processes it
 * probably started, and how to stop it. Esc goes back to the list where
 * it was scrolled. Read again every few seconds while it runs.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { Pill } from "../../components/Pill";
import { Disclosure } from "../../components/controls";
import { IconBackground } from "../../components/icons-controls";
import { subagentsDetail, type SubagentDetailInfo, type SubagentInfo } from "../../lib/ipc/subagents";
import { Box, Kv, ProcessesBox, ResultBox, StopBox, TaskBox, UsageBox } from "./DetailBoxes";
import { FOLLOW_MS } from "./useSubagentRows";
import { kindWords, modelWords, statusOf, timeWords, titleOf, tokenWords } from "./words";

/** Files listed before "Show all". */
const FILES_SHOWN = 5;

export type DetailTarget = { profileId: string; conversationId: string; subagentId: string };

export function SubagentDetail({
  target,
  sessionTitle,
  visible,
  onBack,
  onOpen,
  onConversation,
}: {
  target: DetailTarget;
  sessionTitle: string;
  visible: boolean;
  onBack: () => void;
  /** Another subagent's page (a parent, one it started). */
  onOpen: (subagentId: string) => void;
  /** Its conversation, when it can be opened. */
  onConversation: ((agent: SubagentInfo) => void) | null;
}): JSX.Element {
  const [detail, setDetail] = useState<SubagentDetailInfo | null>(null);
  const { profileId, conversationId, subagentId } = target;
  const top = useRef<HTMLDivElement | null>(null);
  // Another subagent's page starts at its top.
  useEffect(() => {
    void top.current?.scrollIntoView?.({ block: "start" });
  }, [subagentId]);
  useEffect(() => {
    let alive = true;
    setDetail(null);
    const read = () =>
      void subagentsDetail(profileId, conversationId, subagentId).then(
        (info) => alive && setDetail(info),
        () => undefined,
      );
    read();
    const timer = window.setInterval(() => {
      if (visible && document.visibilityState !== "hidden") read();
    }, FOLLOW_MS);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [profileId, conversationId, subagentId, visible]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented && !document.querySelector('[role="menu"]')) onBack();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onBack]);
  const agent = detail?.agent ?? null;
  const running = agent?.status === "running";
  return (
    <div ref={top} className="flex w-full min-w-0 scroll-mt-4 flex-col gap-4" data-testid="subagent-detail">
      <nav aria-label="Where you are" className="sticky top-0 z-10 -mx-1 flex min-w-0 items-center gap-1.5 bg-canvas/95 px-1 py-1.5 text-meta text-muted backdrop-blur">
        <span className="max-w-[12rem] truncate">{sessionTitle}</span>
        <span aria-hidden="true">›</span>
        <button type="button" onClick={onBack} className="rounded px-0.5 text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="subagent-crumb-list">
          Subagents
        </button>
        {(detail?.chain ?? []).map((crumb) => (
          <span key={crumb.id} className="contents">
            <span aria-hidden="true">›</span>
            <button type="button" onClick={() => onOpen(crumb.id)} className="max-w-[14rem] truncate rounded px-0.5 text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
              {crumb.title ?? "A subagent"}
            </button>
          </span>
        ))}
        <span aria-hidden="true">›</span>
        <span className="min-w-0 truncate font-medium text-ink" aria-current="page">
          {agent ? titleOf(agent) : "…"}
        </span>
        <span className="ml-auto hidden shrink-0 sm:inline">Esc to go back</span>
      </nav>
      {detail === null ? (
        <div className="flex flex-col gap-3" role="status" aria-label="Loading the subagent">
          <span className="h-8 w-2/3 rounded-lg bg-surface-3 motion-safe:animate-pulse" />
          <span className="h-40 rounded-card bg-surface-3 motion-safe:animate-pulse" />
        </div>
      ) : !agent ? (
        <p className="text-body text-ink-2" data-testid="subagent-detail-missing">
          {detail.reason ?? "This subagent isn't in the session's records any more."}
        </p>
      ) : (
        <>
          <Head detail={detail} agent={agent} onConversation={onConversation} />
          <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)]">
            <div className="flex min-w-0 flex-col gap-4">
              <TaskBox detail={detail} />
              <ResultBox detail={detail} />
              <WorkBox detail={detail} onOpen={onOpen} />
            </div>
            <div className="flex min-w-0 flex-col gap-4">
              <UsageBox detail={detail} />
              <ProcessesBox detail={detail} />
              {running && <StopBox detail={detail} />}
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function Head({ detail, agent, onConversation }: { detail: SubagentDetailInfo; agent: SubagentInfo; onConversation: ((agent: SubagentInfo) => void) | null }): JSX.Element {
  const status = statusOf(agent.status);
  const Icon = status.icon;
  const parent = detail.chain[detail.chain.length - 1];
  const facts = [kindWords(agent.kind), modelWords(agent.model), parent ? `started by ${parent.title ?? "another subagent"}` : "started by the session", detail.branch ? `branch ${detail.branch}` : null].filter(Boolean);
  return (
    <header className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <h2 className="min-w-0 text-title font-semibold text-ink" data-testid="subagent-title">
          {titleOf(agent)}
        </h2>
        <span title={status.hint}>
          <Pill tone={status.tone} testId="subagent-detail-status">
            <Icon aria-hidden="true" width={11} height={11} className="mr-1" />
            {status.word}
            {agent.duration_ms !== null && ` · ${timeWords(agent)}`}
          </Pill>
        </span>
        {agent.has_conversation && onConversation && (
          <Button size="sm" variant="secondary" className="ml-auto" onClick={() => onConversation(agent)} data-testid="subagent-open-conversation">
            Open its conversation
          </Button>
        )}
      </div>
      <p className="flex flex-wrap items-center gap-x-1.5 text-meta text-ink-2">
        {facts.join(" · ")}
        {agent.background === true && (
          <span className="inline-flex items-center gap-1">
            · <IconBackground aria-hidden="true" width={12} height={12} /> in the background
          </span>
        )}
      </p>
    </header>
  );
}

function WorkBox({ detail, onOpen }: { detail: SubagentDetailInfo; onOpen: (id: string) => void }): JSX.Element {
  const tools = detail.tools;
  const parts = tools
    ? [
        [tools.reads, "read", "reads"],
        [tools.searches, "search", "searches"],
        [tools.commands, "command", "commands"],
        [tools.edits, "edit", "edits"],
        [tools.other, "other", "other"],
      ]
        .filter(([n]) => (n as number) > 0)
        .map(([n, one, many]) => `${n} ${n === 1 ? one : many}`)
    : [];
  const name = (path: string) => path.split("/").pop() || path;
  const files = detail.files;
  return (
    <Box title="Work" testId="subagent-work-box">
      <Kv label="Tool uses">{tools ? [`${tools.total}`, ...parts].join(" · ") : <span className="font-normal text-muted">Not recorded by {detail.tool}</span>}</Kv>
      {detail.lines && (
        <Kv label="Lines">
          <span className="text-success">+{detail.lines.added.toLocaleString()}</span> <span className="text-danger">−{detail.lines.removed.toLocaleString()}</span>
        </Kv>
      )}
      {files.length > 0 ? (
        <div className="flex flex-col gap-1">
          <Kv label="Files">
            <span title={files.slice(0, FILES_SHOWN).join("\n")}>
              {files.slice(0, FILES_SHOWN).map(name).join(", ")}
              {detail.files_total > FILES_SHOWN && <span className="font-normal text-muted">, +{detail.files_total - FILES_SHOWN} more</span>}
            </span>
          </Kv>
          {detail.files_total > FILES_SHOWN && (
            <Disclosure summary={`Show all ${detail.files_total} files`} testId="subagent-files-more" className="self-end">
              <ul className="mt-2 max-h-60 overflow-y-auto rounded-lg border border-hairline bg-surface-2 p-2 font-mono text-label text-ink">
                {files.map((file) => (
                  <li key={file} className="truncate" title={file}>
                    {file}
                  </li>
                ))}
              </ul>
            </Disclosure>
          )}
        </div>
      ) : (
        <Kv label="Files">
          <span className="font-normal text-muted">None edited</span>
        </Kv>
      )}
      {detail.children_total > 0 && (
        <div className="flex flex-col gap-1 border-t border-hairline pt-2">
          <span className="text-meta text-muted">The subagents it started ({detail.children_total})</span>
          <ul className="flex flex-col">
            {detail.children.map((child) => {
              const status = statusOf(child.status);
              const Icon = status.icon;
              return (
                <li key={child.id}>
                  <button type="button" onClick={() => onOpen(child.id)} className="flex w-full min-w-0 items-center gap-2 rounded-lg px-2 py-1.5 text-left outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="subagent-child">
                    <Icon aria-label={status.word} width={13} height={13} className="shrink-0 text-muted" />
                    <span className="min-w-0 flex-1 truncate text-body text-ink">{titleOf(child)}</span>
                    <span className="shrink-0 text-meta tabular-nums text-muted">{timeWords(child)} · {tokenWords(child.tokens)}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </Box>
  );
}
