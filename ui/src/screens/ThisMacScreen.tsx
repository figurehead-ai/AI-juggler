/**
 * AJ-E1528 — the This Mac page: what Home's panel summarises, in full.
 * The Mac's state in words and meters; what's using it, grouped by tool
 * and then by session or window, with the tools AI Juggler didn't start
 * marked; what's working it hard, with links; and recent alerts and
 * advice. From the read model Home already has (`ThisMacContext`):
 * nothing new is measured. Controls only for what AI Juggler owns (D-08);
 * the same rows, meters and type as Home's panel, detail behind each
 * tool's disclosure (D-25).
 *
 * Readability (the maintainer's bar): title 22 px, section titles 15 px
 * semibold in ink, body 14 px, meta 13 px, nothing under 12 px; secondary
 * words in ink at 80 % (7:1 or better); figures in tabular numbers; the
 * Mac's figures as cards sized to their content, in a grid.
 */
import { Page } from "../components/Page";
import { useContext, type JSX, type ReactNode } from "react";
import { Button } from "../components/primitives";
import { ToolMark } from "../components/ToolMark";
import { AwakeLine, MachineState, OwnLine, TreeRow, cpuWords } from "../components/this-mac-parts";
import { FoldChevron, MemoryMeter, OtherAppsGroup } from "../components/this-mac-memory";
import { RowPlaceholders } from "../components/Placeholder";
import { alertSubject, bytesWords, ThisMacContext, type ThisMacInfo, type ToolTreeInfo } from "../lib/machine";
import type { ToolGroupInfo } from "../types/generated/ToolGroupInfo";

/** A group's trees, as the backend grouped them: our sessions of that
 * tool, or the windows of a tool AI Juggler didn't start. */
export function treesOf(group: ToolGroupInfo, info: ThisMacInfo): ToolTreeInfo[] {
  return [...info.top, ...info.others].filter((tree) => (tree.session_id !== null) === group.ours && tree.group === group.tool);
}

/** A section: its title, and its content in one card — or, with `bare`,
 * as given (a grid of cards). */
function Section({ id, title, bare = false, children }: { id: string; title: string; bare?: boolean; children: ReactNode }): JSX.Element {
  return (
    <section aria-labelledby={`${id}-title`} className="flex flex-col gap-3" data-testid={id}>
      <h2 id={`${id}-title`} className="text-section font-semibold text-ink">
        {title}
      </h2>
      {bare ? children : <div className="flex flex-col gap-3 rounded-xl border border-hairline bg-surface px-4 py-3.5">{children}</div>}
    </section>
  );
}

/** One of the Mac's figures as a card: its name, then its content. */
function Card({ title, testId, className = "", children }: { title: string; testId?: string; className?: string; children: ReactNode }): JSX.Element {
  return (
    <div className={`flex flex-col gap-2.5 rounded-xl border border-hairline bg-surface px-4 py-3.5 ${className}`} data-testid={testId}>
      <h3 className="text-body font-semibold text-ink">{title}</h3>
      {children}
    </div>
  );
}

/** A big figure and what it is of: "93%" "of all 30 cores". */
function Figure({ value, of }: { value: string; of: string }): JSX.Element {
  return (
    <p className="flex flex-wrap items-baseline gap-x-1.5 text-body tabular-nums text-muted">
      <span className="text-title font-semibold text-ink">{value}</span> <span>{of}</span>
    </p>
  );
}

/** A plain gauge: how full, out of 100. */
function Gauge({ pct, label }: { pct: number; label: string }): JSX.Element {
  const value = Math.max(0, Math.min(100, pct));
  return (
    <div className="h-3 overflow-hidden rounded-full bg-surface-3" role="meter" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(value)} aria-label={label}>
      <div className="h-full rounded-full bg-faint" style={{ width: `${value}%` }} />
    </div>
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
  // Ours and another app's (AJ-E2407) are sessions; the Claude tools' are windows.
  const sessions = group.ours || trees.some((tree) => !!tree.title);
  return (
    <details className="group border-t border-hairline/60 first:border-t-0" data-testid="this-mac-group" data-ours={group.ours ? "true" : "false"} open={group.ours}>
      <summary className="flex cursor-pointer list-none flex-wrap items-center gap-x-3 gap-y-1 rounded-lg py-3 outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
        <FoldChevron />
        <span className="flex min-w-0 flex-1 basis-40 items-center gap-2 text-body font-medium text-ink">
          {mark && <ToolMark tool={mark} size={14} className="text-muted" />}
          <span className="shrink-0">{group.label}</span>
          <span className="min-w-0 truncate text-meta font-normal text-muted">
            {trees.length === 1 ? "1" : trees.length} {sessions ? (trees.length === 1 ? "session" : "sessions") : trees.length === 1 ? "window" : "windows"}
            {group.ours ? "" : " · not started by AI Juggler"}
          </span>
        </span>
        <span className="ml-auto flex shrink-0 flex-col items-end gap-0.5 tabular-nums">
          <span className="text-body text-ink">{memory}</span>
          <span className="text-meta text-muted" title="As Activity Monitor counts it: 100% is one core busy">
            {cpuWords(group.cpu_one_core_pct)}
          </span>
        </span>
      </summary>
      {trees.length > 0 && (
        <ul className="mb-2 ml-[22px] flex flex-col" aria-label={group.label}>
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
    <Page title="This Mac" subtitle="What your AI tools use on this Mac, measured where they run. Nothing here is sent anywhere." testId="this-mac-page" headingLevel={1}>
      {!info || !machine ? (
        <RowPlaceholders shape="home" rows={4} label="Measuring this Mac" />
      ) : (
        // Two columns on a wide window: the Mac's figures, and what's
        // using it (the shared page frame, 0.1.5).
        <div className="grid items-start gap-x-8 gap-y-8 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
          <div className="flex min-w-0 flex-col gap-8">
          <Section id="this-mac-state" title="Now" bare>
            <MachineState info={info} reasonsShown={machine.summary.reasons.length} />
            {/* The Mac's figures as cards sized to their content: memory,
                the tallest, beside the processor, swap and AI Juggler. */}
            <div className="grid items-start gap-3 md:grid-cols-2" data-testid="this-mac-figures">
              <Card title="Memory" testId="this-mac-memory-card" className="md:col-span-2">
                <MemoryMeter info={info} more />
              </Card>
              <Card title="Processor" testId="this-mac-cpu">
                {machine.cpu_pct === null ? (
                  <p className="text-body text-muted">Not measured yet</p>
                ) : (
                  <>
                    <Figure value={`${Math.round(machine.cpu_pct)}%`} of={`of all ${machine.cores} cores`} />
                    <Gauge pct={machine.cpu_pct} label={`Processor: ${Math.round(machine.cpu_pct)}% of all ${machine.cores} cores`} />
                  </>
                )}
                {info.tools.count > 0 && (
                  <p className="text-meta tabular-nums text-muted">
                    AI tools: {info.tools.cpu_pct === null ? "not measured yet" : `${Math.round(info.tools.cpu_pct)}% of all cores`}
                  </p>
                )}
              </Card>
              {machine.swap_total > 0 && (
                <Card title="Swap" testId="this-mac-swap">
                  <Figure value={bytesWords(machine.swap_used)} of={`of ${bytesWords(machine.swap_total)} moved to disk`} />
                  <Gauge pct={(machine.swap_used / machine.swap_total) * 100} label={`Swap: ${bytesWords(machine.swap_used)} of ${bytesWords(machine.swap_total)} moved to disk`} />
                  <p className="text-meta text-muted">Memory macOS moved to disk to make room. Reading it back is slow.</p>
                </Card>
              )}
              <Card title="AI Juggler" testId="this-mac-own-card">
                <OwnLine info={info} />
                <AwakeLine info={info} />
              </Card>
            </div>
          </Section>
          </div>

          <div className="flex min-w-0 flex-col gap-8">
          <Section id="this-mac-using" title="What's using it">
            {info.by_tool.length === 0 && <p className="text-body text-muted">No AI tools are running.</p>}
            <div className="flex flex-col">
              {info.by_tool.map((group) => (
                <ToolGroup key={`${group.ours ? "ours" : "theirs"}-${group.tool}`} group={group} info={info} nameOf={nameOf} onOpenSession={onOpenSession} />
              ))}
              {/* The 0.1.4 review: what else is using the memory, folded. */}
              <OtherAppsGroup info={info} />
            </div>
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
                      <span className="text-body font-medium text-ink">{alert.title}</span>
                      <span className="text-meta text-muted">{[alert.detail, alertSubject(alert, nameOf)].filter(Boolean).join(". ")}</span>
                    </li>
                  ))}
                </ul>
              )}
              {info.recommendations.length > 0 && (
                <ul className="flex flex-col gap-1 text-meta text-muted" aria-label="From your history" data-testid="this-mac-recommendations">
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
                <span className="min-w-0 flex-1 basis-40 text-body text-muted">
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
          </div>
        </div>
      )}
    </Page>
  );
}
