/**
 * AJ-E1528 — the This Mac page: what Home's panel summarises, in full.
 * The Mac's state in words and meters; what's using it, grouped by tool
 * and then by session or window, with the tools AI Juggler didn't start
 * marked; what's working it hard, with links; and recent alerts and
 * advice. From the read model Home already has (`ThisMacContext`):
 * nothing new is measured. Controls only for what AI Juggler owns (D-08);
 * the same rows, meters and type as Home's panel, detail behind each
 * tool's disclosure (D-25).
 */
import { useContext, type JSX, type ReactNode } from "react";
import { Button } from "../components/primitives";
import { SectionTitle } from "../components/SectionTitle";
import { ToolMark } from "../components/ToolMark";
import { MachineState, MemoryMeter, TreeRow, cpuWords } from "../components/this-mac-parts";
import { RowPlaceholders } from "../components/Placeholder";
import { alertSubject, bytesWords, ThisMacContext, type ThisMacInfo, type ToolTreeInfo } from "../lib/machine";
import type { ToolGroupInfo } from "../types/generated/ToolGroupInfo";

/** A group's trees, as the backend grouped them: our sessions of that
 * tool, or the windows of a tool AI Juggler didn't start. */
export function treesOf(group: ToolGroupInfo, info: ThisMacInfo): ToolTreeInfo[] {
  return [...info.top, ...info.others].filter((tree) => (tree.session_id !== null) === group.ours && tree.group === group.tool);
}

function Section({ id, title, children }: { id: string; title: string; children: ReactNode }): JSX.Element {
  return (
    <section aria-labelledby={`${id}-title`} className="flex flex-col gap-2.5" data-testid={id}>
      <SectionTitle id={`${id}-title`}>{title}</SectionTitle>
      <div className="flex flex-col gap-3 rounded-xl border border-hairline bg-surface px-4 py-3.5">{children}</div>
    </section>
  );
}

function ToolGroup({
  group,
  info,
  nameOf,
  onOpenSession,
}: {
  group: ToolGroupInfo;
  info: ThisMacInfo;
  nameOf: (sessionId: string) => string;
  onOpenSession: (id: string) => void;
}): JSX.Element {
  const trees = treesOf(group, info);
  // Another tool's windows: its trees say which tool they are.
  const mark = group.ours ? group.tool : trees.find((tree) => tree.tool)?.tool;
  const memory = `${group.totals.memory_estimate ? "about " : ""}${bytesWords(group.totals.rss_bytes)}`;
  return (
    <details className="group border-t border-hairline/60 first:border-t-0" data-testid="this-mac-group" data-ours={group.ours ? "true" : "false"} open={group.ours}>
      <summary className="flex cursor-pointer list-none flex-wrap items-center gap-x-3 gap-y-1 rounded-lg py-2.5 outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
        {/* The rail's fold chevron: one look for every disclosure. */}
        <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" className="shrink-0 text-faint transition-transform group-open:rotate-90 motion-reduce:transition-none">
          <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        <span className="flex min-w-0 flex-1 basis-40 items-center gap-2 text-sm font-medium text-ink">
          {mark && <ToolMark tool={mark} size={14} className="text-muted" />}
          <span className="truncate">{group.label}</span>
          <span className="shrink-0 text-[11px] font-normal text-muted">
            {trees.length === 1 ? "1" : trees.length} {group.ours ? (trees.length === 1 ? "session" : "sessions") : trees.length === 1 ? "window" : "windows"}
            {group.ours ? "" : " · not started by AI Juggler"}
          </span>
        </span>
        <span className="ml-auto flex shrink-0 flex-col items-end gap-0.5 font-mono text-xs">
          <span className="text-ink">{memory}</span>
          <span className="text-faint" title="As Activity Monitor counts it: 100% is one core busy">
            {cpuWords(group.cpu_one_core_pct)}
          </span>
        </span>
      </summary>
      {trees.length > 0 && (
        <ul className="mb-2 ml-5 flex flex-col" aria-label={group.label}>
          {trees.map((tree) => (
            <TreeRow key={tree.root_pid !== null ? `pid-${tree.root_pid}` : (tree.session_id ?? `${tree.place}-${tree.rss_bytes}`)} tree={tree} nameOf={nameOf} onOpenSession={onOpenSession} />
          ))}
        </ul>
      )}
    </details>
  );
}

export function ThisMacScreen({
  nameOf,
  onOpenSession,
  onOpenCleanup,
}: {
  /** A session's name on screen (its place: branch, else folder). */
  nameOf: (sessionId: string) => string;
  onOpenSession: (id: string) => void;
  onOpenCleanup?: () => void;
}): JSX.Element {
  const info = useContext(ThisMacContext);
  const machine = info?.machine;
  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-7 py-2" data-testid="this-mac-page">
      <header className="flex flex-col gap-1.5">
        <h1 className="text-2xl font-semibold tracking-tight">This Mac</h1>
        <p className="text-sm text-muted">What your AI tools use on this Mac, measured where they run. Nothing here is sent anywhere.</p>
      </header>
      {!info || !machine ? (
        <RowPlaceholders shape="home" rows={4} label="Measuring this Mac" />
      ) : (
        <>
          <Section id="this-mac-state" title="Now">
            <MachineState info={info} reasonsShown={machine.summary.reasons.length} />
            <MemoryMeter info={info} />
            <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-xs" data-testid="this-mac-figures">
              <dt className="text-muted">CPU</dt>
              <dd className="font-mono text-ink">{machine.cpu_pct === null ? "—" : `${Math.round(machine.cpu_pct)}% of all ${machine.cores} cores`}</dd>
              {machine.swap_total > 0 && (
                <>
                  <dt className="text-muted">Swap</dt>
                  <dd className="font-mono text-ink">
                    {bytesWords(machine.swap_used)} of {bytesWords(machine.swap_total)}
                  </dd>
                </>
              )}
            </dl>
          </Section>

          <Section id="this-mac-using" title="What's using it">
            {info.by_tool.length === 0 ? (
              <p className="text-sm text-muted">No AI tools are running.</p>
            ) : (
              <div className="flex flex-col">
                {info.by_tool.map((group) => (
                  <ToolGroup key={`${group.ours ? "ours" : "theirs"}-${group.tool}`} group={group} info={info} nameOf={nameOf} onOpenSession={onOpenSession} />
                ))}
              </div>
            )}
          </Section>

          {info.heavy.length > 0 && (
            <Section id="this-mac-heavy" title="Working your Mac hard">
              <ul className="flex flex-col">
                {info.heavy.map((tree) => (
                  <TreeRow key={tree.root_pid !== null ? `pid-${tree.root_pid}` : (tree.session_id ?? `${tree.place}-${tree.rss_bytes}`)} tree={tree} nameOf={nameOf} onOpenSession={onOpenSession} />
                ))}
              </ul>
            </Section>
          )}

          {(info.alerts.length > 0 || info.recommendations.length > 0) && (
            <Section id="this-mac-alerts" title="Alerts and advice">
              {info.alerts.length > 0 && (
                <ul className="flex flex-col gap-2" aria-label="Recent alerts">
                  {info.alerts.map((alert) => (
                    <li key={alert.key} className="flex flex-col gap-0.5" data-testid="this-mac-alert">
                      <span className="text-sm font-medium text-ink">{alert.title}</span>
                      <span className="text-xs text-muted">{[alert.detail, alertSubject(alert, nameOf)].filter(Boolean).join(". ")}</span>
                    </li>
                  ))}
                </ul>
              )}
              {info.recommendations.length > 0 && (
                <ul className="flex flex-col gap-1 text-xs text-muted" aria-label="From your history" data-testid="this-mac-recommendations">
                  {info.recommendations.map((line) => (
                    <li key={line}>{line}.</li>
                  ))}
                </ul>
              )}
            </Section>
          )}

          {info.leftover_processes > 0 && (
            <Section id="this-mac-leftovers-page" title="Left behind">
              <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
                <span className="min-w-0 flex-1 basis-40 text-sm text-muted">
                  {info.leftover_processes === 1 ? "1 process" : `${info.leftover_processes} processes`} left running after
                  {info.leftover_processes === 1 ? " its" : " their"} session closed
                  {info.leftover_ports > 0 ? ` · ${info.leftover_ports === 1 ? "holding a port" : `holding ${info.leftover_ports} ports`}` : ""}
                </span>
                {onOpenCleanup && (
                  <Button size="sm" variant="secondary" onClick={onOpenCleanup}>
                    Clean up
                  </Button>
                )}
              </div>
            </Section>
          )}
        </>
      )}
    </div>
  );
}
