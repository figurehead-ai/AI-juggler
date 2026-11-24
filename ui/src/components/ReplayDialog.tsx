/**
 * AJ-E1007 — replay an ended or archived Instance, read-only: its
 * conversation, what happened to it (status changes and lifecycle steps),
 * the switches it took part in and its processes.
 *
 * 0.5.1 F79: the conversation is drawn with the session page's own
 * conversation view (ADR-0024's `Turn`: your words on the right, the
 * agent's on the left under its mark and name), read-only: no composer,
 * nothing to send. Its facts are said in plain words, times as "2 h ago"
 * (the date on hover); no checksum, process roles, ledger states or
 * fidelity codes on screen.
 */
import { useEffect, useMemo, useRef, useState, type JSX, type ReactNode } from "react";
import { errorText } from "../lib/bridge";
import { instanceReplay } from "../lib/ipc/session-history";
import type { ReplayInfo } from "../types/generated/ReplayInfo";
import type { ReplayEntryInfo } from "../types/generated/ReplayEntryInfo";
import type { TurnInfo } from "../lib/ipc/conversations";
import { cliAppName } from "../lib/app-name";
import { byteSize } from "../lib/compact";
import { statusWords } from "../lib/status-words";
import { agoWords } from "../lib/time-words";
import { Turn } from "../pages/session/ConversationTurn";
import { speakerName } from "../pages/session/model";
import { Button } from "./primitives";
import { DialogFrame, DialogOverlay } from "./DialogFrame";

/** The stored messages as the conversation view's turns: yours and the
 * agent's (its consecutive messages one turn); a line of another kind
 * (a system note, a tool's record) isn't part of the conversation. */
export function replayTurns(entries: readonly ReplayEntryInfo[]): TurnInfo[] {
  const turns: TurnInfo[] = [];
  for (const entry of entries) {
    const speaker = entry.role === "user" ? "you" : entry.role === "assistant" ? "assistant" : null;
    if (speaker === null || !entry.text.trim()) continue;
    const block = { kind: "text", text: entry.text, steps: [], hidden: 0, calls: [], thoughts: [], title: null, title_running: null, after_thinking: false, message_id: null, at_ms: null, final_answer: false };
    const last = turns[turns.length - 1];
    if (last && last.speaker === speaker && speaker === "assistant") {
      last.blocks.push(block);
      continue;
    }
    turns.push({ id: `replay-${turns.length}`, revision: 0, offset: turns.length, open: false, speaker, blocks: [block], blocks_hidden: 0, at_ms: null, model: null, redacted: 0 });
  }
  return turns;
}

const LIFECYCLE: Record<string, string> = { reviewed: "Marked reviewed", archived: "Archived", unarchived: "Back from the archive", resumed: "Resumed", relaunched: "Relaunched" };

/** A step of its history in words ("Ended", "Archived"). */
function stepWords(event: { kind: string; what: string }): string {
  if (event.kind === "lifecycle") return LIFECYCLE[event.what] ?? event.what.charAt(0).toUpperCase() + event.what.slice(1).replace(/_/g, " ");
  return statusWords(event.what);
}

const CARRIED: Record<string, string> = {
  F0: "with handover notes",
  F1: "with the full story, fresh start",
  F2: "as the same conversation in another app",
  F3: "as the same conversation in another app",
  F4: "continuing exactly",
};

/** A switch it took part in ("Came from Codex CLI · Personal, with the
 * full story, fresh start"); one that didn't finish says so. */
function switchWords(step: ReplayInfo["lineage"][number]): string {
  const app = cliAppName(step.other_provider, step.other_account);
  const how = step.fidelity ? CARRIED[step.fidelity] : undefined;
  const head = `${step.direction === "in" ? "Came from" : "Went on to"} ${app}`;
  if (step.state === "failed") return `${head}: the switch didn't finish`;
  if (step.state === "rolled_back") return `${head}: the switch was undone`;
  return how ? `${head}, ${how}` : head;
}

/** A process it ran, in words ("The session's own process ended cleanly"). */
function processWords(process: ReplayInfo["processes"][number]): string {
  const who = process.role === "root" ? "The session's own process" : process.role === "helper" ? "A helper process" : "A process";
  if (process.state === "running") return `${who} is running`;
  const how: Record<string, string> = { clean: "ended cleanly", reaped: "was cleaned up by AI Juggler", stopped: "was stopped", crashed: "crashed", killed: "was ended by the system" };
  return `${who} ${(process.exit_kind && how[process.exit_kind]) ?? "ended"}`;
}

/** When, in words; the date and time on hover. */
function When({ ms, now }: { ms: number; now: number }): JSX.Element {
  const date = new Date(ms);
  return (
    <time dateTime={date.toISOString()} title={date.toLocaleString([], { dateStyle: "medium", timeStyle: "short" })} className="shrink-0 tabular-nums text-muted">
      {agoWords(ms, now)}
    </time>
  );
}

function Facts({ id, title, children }: { id: string; title: string; children: ReactNode }): JSX.Element {
  return (
    <section aria-labelledby={id} className="flex flex-col gap-1.5">
      <h4 id={id} className="text-meta font-semibold text-ink">
        {title}
      </h4>
      {children}
    </section>
  );
}

export function ReplayDialog({ sessionId, tool = "", onClose }: { sessionId: string; /** Its tool: the agent's name and mark in the conversation. */ tool?: string; onClose: () => void }): JSX.Element {
  const [replay, setReplay] = useState<ReplayInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const closeRef = useRef<HTMLButtonElement | null>(null);
  // Its times are read against when it opened.
  const [now] = useState(() => Date.now());
  useEffect(() => {
    instanceReplay(sessionId)
      .then(setReplay)
      .catch((cause) => setError(errorText(cause)));
  }, [sessionId]);
  useEffect(() => {
    closeRef.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  const turns = useMemo(() => (replay ? replayTurns(replay.entries) : []), [replay]);
  const speaker = tool ? speakerName(tool) : "Agent";
  return (
    <DialogOverlay className="fixed inset-0 z-50 flex items-stretch justify-center bg-black/60 p-4 backdrop-blur-sm">
      <DialogFrame
        role="dialog"
        aria-modal="true"
        aria-labelledby="replay-title"
        data-testid="replay-dialog"
        sheet
        className="h-full max-w-6xl rounded-xl bg-surface shadow-card"
        headerClassName="flex flex-wrap items-center gap-x-3 gap-y-1 px-5 pb-3 pt-4"
        header={
          <>
            <span className="flex min-w-0 flex-1 flex-col gap-0.5">
              <h3 id="replay-title" className="text-section font-semibold text-ink">
                Replay
              </h3>
              {replay && (
                <span className="text-meta text-muted" data-testid="replay-proof">
                  {replay.transcript_path ? `Read-only: the conversation as it was stored · ${byteSize(replay.bytes)}` : "Read-only: no conversation was stored for it"}
                  {replay.truncated ? " · its last 16 MB" : ""}
                </span>
              )}
            </span>
            {/* Its one way out, the kit's secondary; Escape closes it too. */}
            <Button ref={closeRef} size="sm" variant="secondary" onClick={onClose} aria-keyshortcuts="Escape" data-testid="replay-close">
              Close
            </Button>
          </>
        }
        bodyClassName="flex min-h-0 flex-col gap-3 px-5 pb-5"
      >
        {error && (
          <p role="alert" className="text-meta text-ink">
            {error}
          </p>
        )}
        {replay && (
          <div className="grid min-h-0 flex-1 grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_17rem]">
            <section className="min-h-[240px] overflow-y-auto rounded-card bg-surface-2" aria-label="The conversation, read-only" data-testid="replay-transcript">
              {turns.length === 0 ? (
                <p className="p-5 text-body text-muted" data-testid="replay-empty">
                  Nothing to replay: the conversation is empty or wasn't stored.
                </p>
              ) : (
                <div className="mx-auto flex w-full max-w-[760px] flex-col gap-6 px-5 py-5">
                  <p className="text-center text-meta text-muted">{replay.truncated ? "Its last 16 MB" : "The start of the conversation"}</p>
                  <ol className="flex flex-col gap-6" data-testid="replay-turns">
                    {turns.map((turn, index) => (
                      <Turn key={turn.id} turn={turn} tool={tool} speaker={speaker} layout="stretches" asks={false} startedMs={turns[index - 1]?.at_ms ?? null} working={false} />
                    ))}
                  </ol>
                </div>
              )}
            </section>
            <div className="flex min-h-0 flex-col gap-5 overflow-y-auto text-meta">
              <Facts id="replay-history" title="What happened">
                <ol className="flex flex-col gap-1" data-testid="replay-history-list">
                  {replay.history.map((event, index) => (
                    <li key={index} className="flex justify-between gap-3">
                      <span className="text-ink">{stepWords(event)}</span>
                      <When ms={event.at_ms} now={now} />
                    </li>
                  ))}
                </ol>
              </Facts>
              <Facts id="replay-lineage" title="Switches">
                {replay.lineage.length === 0 ? (
                  <p className="text-ink-2">Never switched.</p>
                ) : (
                  <ul className="flex flex-col gap-1" data-testid="replay-lineage-list">
                    {replay.lineage.map((step, index) => (
                      <li key={index} className="text-ink">
                        {switchWords(step)}
                      </li>
                    ))}
                  </ul>
                )}
              </Facts>
              <Facts id="replay-processes" title="Processes">
                {replay.processes.length === 0 ? (
                  <p className="text-ink-2">No record: AI Juggler keeps one for 7 days after a process ends.</p>
                ) : (
                  <ul className="flex flex-col gap-1" data-testid="replay-process-list">
                    {replay.processes.map((process, index) => (
                      <li key={index} className="flex justify-between gap-3">
                        <span className="text-ink">{processWords(process)}</span>
                        <When ms={process.last_seen_ms} now={now} />
                      </li>
                    ))}
                  </ul>
                )}
              </Facts>
            </div>
          </div>
        )}
      </DialogFrame>
    </DialogOverlay>
  );
}
