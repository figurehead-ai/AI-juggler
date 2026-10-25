/**
 * AJ-E2108 — a session's conversation, on its page only (never in rows,
 * the tray or notifications): its last turns as its own app shows them,
 * your words on the right, the agent's on the left with its mark, its
 * markdown and its tool calls folded to one line each run. Read from the
 * end of its transcript once the page is shown, earlier turns on request,
 * followed along while it runs (useLastMessages); keys and tokens taken
 * out; kept nowhere. When it asks you, the question waits at the bottom
 * with Go to it. A chat its app keeps on its servers has no transcript on
 * this Mac: its title is all there is.
 */
import { useLayoutEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { Button, Card } from "../../components/primitives";
import { ToolMark } from "../../components/ToolMark";
import { IconGoTo } from "../../components/icons-actions";
import type { LastTurnInfo, TurnBlockInfo } from "../../lib/ipc/conversations";
import { agoWords } from "../../lib/time-words";
import { Markdown, plainWords } from "./Markdown";
import { useLastMessages, type Chat } from "./useLastMessages";
import { appWords, isDesktopApp, speakerName, type SessionModel } from "./model";

/** Why there's nothing to show, in words. */
const NONE: Record<string, string> = {
  none: "There's no transcript of it on this Mac to read, so its title is all AI Juggler shows.",
  compressed: "Its transcript is stored compressed now that it has ended, so it isn't read here.",
  unreadable: "Its transcript couldn't be read just now.",
};

/** Meta text: 7:1 or better on every surface, both themes. */
const META = "text-meta text-muted";

function openSettings(): void {
  window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "last-messages" } }));
}

export function LastMessages({
  model,
  visible,
  remoteMac,
  onGoTo,
}: {
  model: SessionModel;
  visible: boolean;
  remoteMac?: string | null;
  /** Go to it, from the question card (a live session). */
  onGoTo?: () => void;
}): JSX.Element {
  const { chat, showEarlier } = useLastMessages({
    profileId: model.profileId,
    conversationId: remoteMac ? null : model.conversationId,
    visible: visible && !remoteMac,
    live: model.live,
    working: model.state.working,
  });
  const speaker = speakerName(model.tool);
  const following = chat.state === "shown" && model.live && visible;
  return (
    // Sized to what it holds, at most the column's height (then its
    // messages scroll): a short conversation has its question card right
    // under its last message, never a void between them.
    <Card className="flex min-h-[22rem] flex-initial flex-col overflow-hidden @4xl:min-h-0" data-testid="last-messages">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-hairline px-5 py-3.5">
        <h3 className="text-section font-semibold text-ink">Conversation</h3>
        {following && (
          <span className="inline-flex items-center gap-1.5 text-label font-medium text-live" data-testid="last-messages-live" title="Following along while it runs">
            <span aria-hidden="true" className="size-1.5 rounded-full bg-current motion-safe:animate-pulse" />
            Live
          </span>
        )}
        {chat.state === "shown" && (
          <span className={"ml-auto " + META} data-testid="last-messages-note">
            From its transcript · nothing kept
            {chat.redacted > 0 && ` · ${chat.redacted === 1 ? "1 key" : `${chat.redacted} keys`} taken out`}
          </span>
        )}
      </div>
      {remoteMac ? (
        <Quiet testId="last-messages-remote">Open on {remoteMac} to read: its messages stay on that Mac.</Quiet>
      ) : model.conversationId === null ? (
        <Quiet>It hasn't written anything yet.</Quiet>
      ) : (
        <Body chat={chat} model={model} speaker={speaker} onShowEarlier={showEarlier} />
      )}
      {!remoteMac && <Bottom chat={chat} model={model} speaker={speaker} onGoTo={onGoTo} />}
    </Card>
  );
}

function Quiet({ children, testId, state }: { children: ReactNode; testId?: string; state?: string }): JSX.Element {
  return (
    <div className="flex flex-1 items-center justify-center px-6 py-10">
      <p className={"max-w-md text-center text-body leading-[1.5] text-muted"} data-testid={testId} data-state={state} role="status">
        {children}
      </p>
    </div>
  );
}

function Body({ chat, model, speaker, onShowEarlier }: { chat: Chat; model: SessionModel; speaker: string; onShowEarlier: () => void }): JSX.Element {
  const scroller = useRef<HTMLDivElement | null>(null);
  const turns = chat.state === "shown" ? chat.turns : null;
  const count = turns?.length ?? 0;
  const first = turns?.[0];
  const memory = useRef<{ count: number; first: LastTurnInfo | undefined; height: number; nearBottom: boolean }>({ count: 0, first: undefined, height: 0, nearBottom: true });
  // Newest at the bottom: in view when it opens, and kept in view as new
  // turns come while you're at the bottom; "Show earlier" keeps your
  // place.
  useLayoutEffect(() => {
    const box = scroller.current;
    if (!box || count === 0) return;
    const before = memory.current;
    const prepended = before.count > 0 && first !== before.first;
    if (prepended) box.scrollTop += box.scrollHeight - before.height;
    else if (before.count === 0 || before.nearBottom) box.scrollTop = box.scrollHeight;
    memory.current = { count, first, height: box.scrollHeight, nearBottom: true };
  }, [count, first]);
  const onScroll = () => {
    const box = scroller.current;
    if (!box) return;
    memory.current.nearBottom = box.scrollHeight - box.scrollTop - box.clientHeight < 96;
    memory.current.height = box.scrollHeight;
  };

  if (chat.state === "failed") return <Quiet>Couldn't read them: {chat.why}</Quiet>;
  if (chat.state === "loading") {
    return (
      <div className="flex flex-1 flex-col gap-5 px-5 py-6" aria-label="Loading the conversation" role="status">
        <span className="ml-auto h-10 w-1/2 rounded-2xl bg-surface-3 motion-safe:animate-pulse" />
        <span className="h-16 w-3/4 rounded-2xl bg-surface-3 motion-safe:animate-pulse" />
        <span className="ml-auto h-8 w-1/3 rounded-2xl bg-surface-3 motion-safe:animate-pulse" />
      </div>
    );
  }
  if (chat.state === "off") {
    return (
      <Quiet testId="last-messages-off">
        Last messages are off.{" "}
        <button type="button" onClick={openSettings} className="font-medium text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60">
          Turn them on in Settings › Desktop apps
        </button>
        .
      </Quiet>
    );
  }
  if (chat.state !== "shown" || chat.turns.length === 0) {
    return (
      <Quiet testId="last-messages-none" state={chat.state}>
        {NONE[chat.state] ?? "The end of its transcript holds no messages."}
      </Quiet>
    );
  }
  const asksAt = model.state.asks && chat.turns[chat.turns.length - 1]?.speaker === "assistant" ? chat.turns.length - 1 : -1;
  return (
    <div ref={scroller} onScroll={onScroll} className="min-h-0 flex-1 overflow-y-auto outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent/60" tabIndex={0} role="region" aria-label="Messages" data-testid="last-messages-scroll">
      <div className="mx-auto flex w-full max-w-[760px] flex-col gap-6 px-5 py-5">
        {chat.earlier !== null ? (
          <div className="flex justify-center">
            <Button size="md" variant="secondary" onClick={onShowEarlier} disabled={chat.loadingEarlier} data-testid="last-messages-earlier">
              {chat.loadingEarlier ? "Loading earlier messages…" : "Show earlier messages"}
            </Button>
          </div>
        ) : (
          <p className={"text-center " + META} data-testid="last-messages-start">
            The start of the conversation
          </p>
        )}
        <ol className="flex flex-col gap-6" data-testid="last-messages-turns">
          {chat.turns.map((turn, index) => (
            <Turn key={index} turn={turn} tool={model.tool} speaker={speaker} asks={index === asksAt} />
          ))}
        </ol>
      </div>
    </div>
  );
}

function When({ ms }: { ms: number | null }): JSX.Element | null {
  if (ms === null) return null;
  const date = new Date(ms);
  return (
    <time dateTime={date.toISOString()} title={date.toLocaleString()} className="text-label tabular-nums text-muted">
      {agoWords(ms, Date.now())}
    </time>
  );
}

function Turn({ turn, tool, speaker, asks }: { turn: LastTurnInfo; tool: string; speaker: string; asks: boolean }): JSX.Element {
  if (turn.speaker === "you") {
    const words = turn.blocks.filter((block) => block.kind === "text").map((block) => block.text).join("\n\n");
    return (
      <li className="flex flex-col items-end gap-1.5" data-testid="last-turn" data-speaker="you">
        <span className="sr-only">You</span>
        <p className="max-w-[85%] whitespace-pre-wrap break-words rounded-2xl rounded-br-md border border-accent/20 bg-accent-soft px-4 py-2.5 text-body leading-[1.5] text-ink">{words}</p>
        <When ms={turn.at_ms} />
      </li>
    );
  }
  return (
    <li className="flex flex-col gap-2" data-testid="last-turn" data-speaker="assistant" data-asks={asks ? "true" : undefined}>
      <div className="flex items-center gap-2">
        <span aria-hidden="true" className="inline-flex size-7 items-center justify-center rounded-full border border-hairline bg-surface-2 text-ink">
          <ToolMark tool={tool} size={15} />
        </span>
        <span className="text-body font-semibold text-ink">{speaker}</span>
        {asks && (
          <span className="rounded-full bg-warning-soft px-2 py-0.5 text-label font-semibold text-warning" data-testid="last-turn-asks">
            asks you
          </span>
        )}
        <When ms={turn.at_ms} />
      </div>
      <div className="flex min-w-0 flex-col gap-3 pl-9">
        {turn.blocks.map((block, index) => (block.kind === "steps" ? <Steps key={index} block={block} /> : <Markdown key={index} text={block.text} />))}
      </div>
    </li>
  );
}

type StepKind = "command" | "read" | "edit" | "search" | "agent" | "plan" | "web" | "other";

/** What kind of call a line is, from its words (the adapters' summary). */
export function stepKind(line: string): StepKind {
  if (line.startsWith("ran a subagent")) return "agent";
  if (line.startsWith('ran "')) return "command";
  if (/^(Read|Glob)\b/.test(line) || line.startsWith("listed ")) return "read";
  if (/^(Edit|MultiEdit|Write|NotebookEdit)\b/.test(line) || line.startsWith("edited ")) return "edit";
  if (line.startsWith("searched the web") || line.startsWith("read ")) return "web";
  if (line.startsWith("searched")) return "search";
  if (line === "updated the task list") return "plan";
  return "other";
}

/** A run of calls in one line: "Ran 2 commands, read 3 files". */
export function stepsSummary(lines: string[]): string {
  const count: Record<StepKind, number> = { command: 0, read: 0, edit: 0, search: 0, agent: 0, plan: 0, web: 0, other: 0 };
  for (const line of lines) {
    const kind = stepKind(line);
    // "edited a.ts, b.ts" is two files.
    count[kind] += kind === "edit" && line.startsWith("edited ") ? line.slice(7).split(", ").length : 1;
  }
  const n = (k: number, one: string, many: string) => `${k} ${k === 1 ? one : many}`;
  const parts = [
    count.command && `ran ${n(count.command, "command", "commands")}`,
    count.edit && `edited ${n(count.edit, "file", "files")}`,
    count.read && `read ${n(count.read, "file", "files")}`,
    count.search && `searched ${count.search === 1 ? "once" : `${count.search} times`}`,
    count.web && `looked on the web ${count.web === 1 ? "once" : `${count.web} times`}`,
    count.agent && `ran ${n(count.agent, "subagent", "subagents")}`,
    count.plan && "updated the task list",
    count.other && `used ${n(count.other, "tool", "tools")}`,
  ].filter(Boolean) as string[];
  const words = parts.join(", ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function Steps({ block }: { block: TurnBlockInfo }): JSX.Element {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex flex-col" data-testid="last-turn-steps">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((now) => !now)}
        className="inline-flex w-fit max-w-full items-center gap-2 rounded-lg border border-hairline bg-surface-2/60 px-2.5 py-1.5 text-left text-meta font-medium text-ink/85 outline-none transition-colors hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className={"shrink-0 transition-transform " + (open ? "rotate-90" : "")}>
          <path d="M9 6l6 6-6 6" />
        </svg>
        <span className="truncate">{stepsSummary(block.steps)}</span>
      </button>
      {open && (
        <ul className="mt-2 flex flex-col gap-1 border-l border-hairline pl-4" data-testid="last-turn-step-lines">
          {block.hidden > 0 && <li className="text-label text-muted">{block.hidden === 1 ? "1 earlier step" : `${block.hidden} earlier steps`} not shown</li>}
          {block.steps.map((line, index) => (
            <li key={index} className="break-words font-mono text-meta leading-[1.5] text-ink/90">
              {line}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** The question an asking turn ends on: its last paragraph with a "?",
 * else its last paragraph, in plain words. */
export function questionOf(turn: LastTurnInfo | undefined): string | null {
  if (!turn || turn.speaker !== "assistant") return null;
  const text = turn.blocks.filter((block) => block.kind === "text").map((block) => block.text).join("\n\n");
  const paragraphs = text
    .replace(/```[\s\S]*?```/g, "")
    .split(/\n\s*\n/)
    .map((paragraph) => plainWords(paragraph))
    .filter(Boolean);
  const asked = [...paragraphs].reverse().find((paragraph) => paragraph.includes("?"));
  return asked ?? paragraphs[paragraphs.length - 1] ?? null;
}

function Bottom({ chat, model, speaker, onGoTo }: { chat: Chat; model: SessionModel; speaker: string; onGoTo?: () => void }): JSX.Element | null {
  if (!model.live) return null;
  const place = model.places[0];
  const app = appWords(place, model.tool);
  // "Answer it" only while it asks (the backlog: a working session's
  // page said "Answer it in ChatGPT" with nothing to answer).
  const answer = app ? (
    <p className={META} data-testid="last-messages-answer">
      {model.state.asks ? `Answer it in ${app}. ` : ""}Go to it brings {place?.label || app} to the front
      {isDesktopApp(place, model.tool) ? "; if another session is showing there, pick this one in its sidebar." : "."}
    </p>
  ) : null;
  const last = chat.state === "shown" ? chat.turns[chat.turns.length - 1] : undefined;
  const question = model.state.asks ? questionOf(last) : null;
  if (model.state.asks) {
    return (
      <div className="border-t border-warning/30 bg-warning-soft px-5 py-4" data-testid="last-messages-asks">
        <div className="mx-auto flex w-full max-w-[760px] flex-wrap items-start gap-x-4 gap-y-3">
          <div className="flex min-w-0 flex-1 basis-72 flex-col gap-1">
            <p className="text-meta font-semibold text-warning">{speaker} asks you</p>
            {question && (
              <p className="line-clamp-3 text-body leading-[1.5] text-ink" data-testid="last-messages-question">
                {question}
              </p>
            )}
            {answer}
          </div>
          {onGoTo && (
            <Button size="md" variant="primary" onClick={onGoTo} data-testid="last-messages-go-to" className="shrink-0">
              Go to it
              <IconGoTo width={14} height={14} aria-hidden="true" />
            </Button>
          )}
        </div>
      </div>
    );
  }
  return (
    <div className="border-t border-hairline px-5 py-3" data-testid={model.state.working ? "last-messages-working" : "last-messages-open"}>
      <div className="mx-auto flex w-full max-w-[760px] flex-col gap-1">
        {model.state.working && (
          <p className="inline-flex items-center gap-2 text-body font-medium text-live" role="status">
            <span aria-hidden="true" className="inline-flex gap-1">
              {[0, 1, 2].map((dot) => (
                <span key={dot} className="size-1.5 rounded-full bg-current motion-safe:animate-pulse" style={{ animationDelay: `${dot * 200}ms` }} />
              ))}
            </span>
            {speaker} is working
          </p>
        )}
        {answer}
      </div>
    </div>
  );
}
