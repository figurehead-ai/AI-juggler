/**
 * AJ-E1520 (0.3) — a subagent's page, its boxes: the task it was given,
 * its result, its work, its usage, the processes it probably started,
 * and how to stop it. Each says plainly when its tool doesn't record a
 * part ("Not recorded by Kimi Code"), never a guess.
 */
import type { JSX, ReactNode } from "react";
import { Card } from "../../components/primitives";
import { Disclosure } from "../../components/controls";
import { IconStop } from "../../components/icons-actions";
import { IconTerminal } from "../../components/icons";
import { Markdown } from "../session/Markdown";
import { bytesWords } from "../../lib/machine";
import { showProcess } from "../../lib/subagent-links";
import type { SubagentDetailInfo } from "../../lib/ipc/subagents";
import { shareWords, tokenWords } from "./words";

/** The first lines of a long text shown before "Show the whole …". */
const HEAD_LINES = 4;

export function Box({ title, children, testId }: { title: string; children: ReactNode; testId?: string }): JSX.Element {
  return (
    <Card className="flex min-w-0 flex-col gap-3 p-4" data-testid={testId}>
      <h3 className="text-meta font-semibold uppercase tracking-wide text-muted">{title}</h3>
      {children}
    </Card>
  );
}

export function Kv({ label, children }: { label: string; children: ReactNode }): JSX.Element {
  return (
    <div className="flex min-w-0 items-baseline justify-between gap-4 text-body">
      <span className="shrink-0 text-muted">{label}</span>
      <span className="min-w-0 text-right font-medium tabular-nums text-ink">{children}</span>
    </div>
  );
}

function Missing({ children }: { children: ReactNode }): JSX.Element {
  return <p className="text-meta leading-[1.5] text-muted">{children}</p>;
}

/** A long text: its first lines, the rest one click away. */
function Long({ text, more, markdown = false, testId }: { text: string; more: string; markdown?: boolean; testId: string }): JSX.Element {
  const lines = text.split("\n");
  const head = lines.slice(0, HEAD_LINES).join("\n");
  const body = (words: string) => (markdown ? <Markdown text={words} /> : <p className="whitespace-pre-wrap break-words text-body leading-[1.55] text-ink">{words}</p>);
  if (lines.length <= HEAD_LINES && text.length <= 480) return <div data-testid={testId}>{body(text)}</div>;
  return (
    <div className="flex flex-col gap-2" data-testid={testId}>
      <div className="relative max-h-[7.5rem] overflow-hidden">{body(head.slice(0, 480))}</div>
      <Disclosure summary={more} testId={`${testId}-more`}>
        <div className="mt-2 max-h-[28rem] overflow-y-auto rounded-lg border border-hairline bg-surface-2 p-3">{body(text)}</div>
      </Disclosure>
    </div>
  );
}

export function TaskBox({ detail }: { detail: SubagentDetailInfo }): JSX.Element {
  return (
    <Box title="Task it was given" testId="subagent-task-box">
      {detail.prompt ? (
        <Long text={detail.prompt} more="Show the whole prompt" testId="subagent-prompt" />
      ) : (
        <Missing>{detail.messages_off ? "Last messages are off in Settings, so its prompt isn't read." : `Its prompt isn't recorded by ${detail.tool}.`}</Missing>
      )}
      {detail.asked && <p className="border-t border-hairline pt-2 text-meta text-muted">From your message: “{detail.asked}”</p>}
    </Box>
  );
}

export function ResultBox({ detail }: { detail: SubagentDetailInfo }): JSX.Element {
  const status = detail.agent?.status;
  return (
    <Box title="Result" testId="subagent-result-box">
      {detail.failure && (
        <p className="rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-body text-danger" data-testid="subagent-failure">
          {detail.failure}
        </p>
      )}
      {detail.result ? (
        <Long text={detail.result} more="Show all" markdown testId="subagent-result" />
      ) : status === "running" ? (
        <Missing>It's still running. Its answer shows here when it ends; a failure shows its reason in one line.</Missing>
      ) : detail.messages_off ? (
        <Missing>Last messages are off in Settings, so its answer isn't read.</Missing>
      ) : !detail.failure ? (
        <Missing>{status === "unknown" ? "Its records don't say how it ended." : `Its answer isn't recorded by ${detail.tool}.`}</Missing>
      ) : null}
    </Box>
  );
}

export function UsageBox({ detail }: { detail: SubagentDetailInfo }): JSX.Element {
  const tokens = detail.tokens;
  const total = tokens ? tokens.input + tokens.output + tokens.cache_read + tokens.cache_write : 0;
  const kinds = tokens
    ? [
        { label: "Input", value: tokens.input, swatch: "bg-accent" },
        { label: "Output", value: tokens.output, swatch: "bg-live" },
        { label: "Read from cache", value: tokens.cache_read, swatch: "bg-ink/35" },
        { label: "Written to cache", value: tokens.cache_write, swatch: "bg-warning" },
      ]
    : [];
  return (
    <Box title="Usage" testId="subagent-usage-box">
      {tokens && total > 0 ? (
        <>
          <div className="flex h-2.5 w-full overflow-hidden rounded-full bg-surface-3" role="img" aria-label={kinds.map((k) => `${k.label} ${Math.round((k.value / total) * 100)}%`).join(", ")}>
            {kinds.map((k) => (k.value > 0 ? <span key={k.label} className={`h-full ${k.swatch}`} style={{ width: `${(k.value / total) * 100}%` }} /> : null))}
          </div>
          <ul className="flex flex-wrap gap-x-3 gap-y-1 text-label text-muted">
            {kinds.map((k) => (
              <li key={k.label} className="inline-flex items-center gap-1.5">
                <span aria-hidden="true" className={`size-2 rounded-sm ${k.swatch}`} />
                {k.label} {tokenWords(k.value)}
              </li>
            ))}
          </ul>
          <Kv label="Tokens">{total.toLocaleString()}</Kv>
          <Kv label="Estimated cost">{detail.cost_usd === null ? <span className="font-normal text-muted">No price for its model</span> : detail.cost_usd < 0.01 ? "<$0.01" : `~$${detail.cost_usd.toFixed(2)}`}</Kv>
          {detail.share_pct !== null && <Kv label={detail.counted === "separate" ? "Share of all tokens" : "Share of the session"}>{shareWords(detail.share_pct)}</Kv>}
          <p className="text-label leading-[1.5] text-muted">
            {detail.counted === "separate" ? "Counted apart from the session's own: its own thread." : "Part of the session's total, not added to it. With the ones it started for the share."}
          </p>
        </>
      ) : (
        <Missing>Its tokens aren't recorded by {detail.tool}.</Missing>
      )}
    </Box>
  );
}

export function ProcessesBox({ detail }: { detail: SubagentDetailInfo }): JSX.Element {
  return (
    <Box title="Processes" testId="subagent-processes-box">
      {detail.processes.length > 0 ? (
        <ul className="flex flex-col gap-1">
          {detail.processes.map((process) => (
            <li key={process.key}>
              <button
                type="button"
                onClick={() => showProcess(process.key)}
                className="flex w-full min-w-0 items-center gap-2 rounded-lg px-2 py-1.5 text-left outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60"
                data-testid="subagent-process"
              >
                <IconTerminal aria-hidden="true" width={14} height={14} className="shrink-0 text-muted" />
                <span className="min-w-0 flex-1 truncate text-body text-ink">{process.name}</span>
                <span className="shrink-0 rounded-full border border-hairline px-1.5 text-label text-muted">probably</span>
                <span className="shrink-0 text-meta tabular-nums text-muted">{process.alive ? bytesWords(process.rss_bytes) : "ended"}</span>
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <Missing>{detail.commands > 0 ? `None running that match the ${detail.commands === 1 ? "command" : `${detail.commands} commands`} it ran.` : "It ran no commands, so it started no processes."}</Missing>
      )}
      <p className="text-label leading-[1.5] text-muted">Matched from the commands it ran: a subagent has no process of its own. Stop one from the session's Processes tab.</p>
    </Box>
  );
}

export function StopBox({ detail }: { detail: SubagentDetailInfo }): JSX.Element {
  const running = detail.agent?.status === "running";
  return (
    <Box title="Stop" testId="subagent-stop-box">
      <div className="flex items-start gap-2.5">
        <IconStop aria-hidden="true" width={16} height={16} className="mt-0.5 shrink-0 text-muted" />
        <p className="text-meta leading-[1.5] text-muted">
          {running ? `AI Juggler can't stop one subagent inside a ${detail.tool} session it doesn't drive.` : "It isn't running."}
          {running && (detail.stop_hint ? ` ${detail.stop_hint}` : " Stopping the session stops all of them.")}
        </p>
      </div>
    </Box>
  );
}
