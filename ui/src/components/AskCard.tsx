/**
 * AJ-E2307 (D-28 as amended by D-40) — a session's open ask, answered in
 * AI Juggler: an approval with the tool's own options, in the tool's own
 * words ("Allow once", "Always allow `npm test`", "Deny"), or a question
 * with the tool's choices and a reply box. The user's click is the answer;
 * nothing here answers on a timer or by default. "Always allow" is the
 * tool's own rule, so it shows (and can be taken back) in the tool too.
 *
 * Small approvals come often (the lab of 2026-10-25: even `ls` asks), so
 * the batch view lists every open ask at once and each card answers from
 * the keyboard: `1`–`9` pick an option, `Enter` the first.
 *
 * A terminal session's ask (from its hooks) has no options here: it is
 * answered in its terminal, and the card says so.
 */
import { useState, type JSX, type KeyboardEvent } from "react";
import { Button } from "./primitives";
import { errorText } from "../lib/bridge";
import { runtimeAnswer, useOpenAsks, type AskInfo } from "../lib/ipc/runtime";
import type { AskOptionInfo } from "../types/generated/AskOptionInfo";

/** The option's button style: the first allow is the primary. */
function variantOf(option: AskOptionInfo, first: boolean): "primary" | "secondary" | "danger" {
  if (option.effect === "deny_and_stop") return "danger";
  return first && option.effect === "allow_once" ? "primary" : "secondary";
}

function kindWords(ask: AskInfo): string {
  if (ask.kind === "question") return "Asks you";
  switch (ask.approval_kind) {
    case "command":
      return "Wants to run";
    case "file_change":
      return "Wants to change files";
    case "permissions":
      return "Wants more access";
    case "tool":
      return "A tool asks";
    default:
      return "Asks to";
  }
}

function Approval({ ask, busy, answer }: { ask: AskInfo; busy: boolean; answer: (option: string) => void }): JSX.Element {
  if (ask.options.length === 0) {
    return <p className="text-[11px] text-muted" data-testid="ask-in-terminal">Answer it in its terminal: Go to it.</p>;
  }
  return (
    <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label="The tool's own options">
      {ask.options.map((option, index) => (
        <Button
          key={option.id}
          size="sm"
          variant={variantOf(option, index === 0)}
          disabled={busy}
          title={option.rule ? `The tool remembers: ${option.rule}. Take it back in the tool.` : undefined}
          aria-keyshortcuts={index < 9 ? String(index + 1) : undefined}
          data-testid={`ask-option-${option.id}`}
          onClick={() => answer(option.id)}
        >
          {index < 9 && <span aria-hidden="true" className="opacity-60">{index + 1}</span>}
          {option.label}
        </Button>
      ))}
    </div>
  );
}

function QuestionForm({ ask, busy, reply }: { ask: AskInfo; busy: boolean; reply: (parts: { part: string; answers: string[] }[]) => void }): JSX.Element {
  const [picked, setPicked] = useState<Record<string, string[]>>({});
  const [typed, setTyped] = useState<Record<string, string>>({});
  const answersOf = (id: string) => {
    const free = (typed[id] ?? "").trim();
    return free ? [...(picked[id] ?? []), free] : (picked[id] ?? []);
  };
  const ready = ask.parts.every((part) => answersOf(part.id).length > 0);
  return (
    <form
      className="flex flex-col gap-2"
      onSubmit={(event) => {
        event.preventDefault();
        if (ready) reply(ask.parts.map((part) => ({ part: part.id, answers: answersOf(part.id) })));
      }}
    >
      {ask.parts.map((part) => (
        <fieldset key={part.id} className="flex flex-col gap-1.5">
          <legend className="text-[12px] text-ink">
            {part.header && <span className="mr-1.5 text-[10px] uppercase tracking-wider text-muted">{part.header}</span>}
            {part.question}
          </legend>
          {part.choices.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {part.choices.map((choice) => {
                const on = (picked[part.id] ?? []).includes(choice.label);
                return (
                  <Button
                    key={choice.label}
                    size="sm"
                    variant={on ? "primary" : "secondary"}
                    aria-pressed={on}
                    title={choice.description ?? undefined}
                    disabled={busy}
                    onClick={() =>
                      setPicked((was) => {
                        const list = was[part.id] ?? [];
                        const next = on ? list.filter((label) => label !== choice.label) : part.multi ? [...list, choice.label] : [choice.label];
                        return { ...was, [part.id]: next };
                      })
                    }
                  >
                    {choice.label}
                  </Button>
                );
              })}
            </div>
          )}
          {part.free_text && (
            <input
              type={part.secret ? "password" : "text"}
              aria-label={part.choices.length > 0 ? `Or your own answer to: ${part.question}` : `Your answer to: ${part.question}`}
              placeholder={part.choices.length > 0 ? "Or your own answer…" : "Your answer…"}
              className="h-8 rounded-lg border border-hairline bg-surface-2 px-2.5 text-[12px] text-ink outline-none focus:border-accent/60"
              value={typed[part.id] ?? ""}
              disabled={busy}
              onChange={(event) => setTyped((was) => ({ ...was, [part.id]: event.target.value }))}
            />
          )}
        </fieldset>
      ))}
      <div>
        <Button type="submit" size="sm" variant="primary" disabled={busy || !ready} data-testid="ask-reply">
          Send answer
        </Button>
      </div>
    </form>
  );
}

/** One open ask. `title` names its session when shown among others.
 * `answer` carries the click elsewhere (E18: an agent's held message
 * goes to the pool's ask, not a tool's); the card is the same. */
export function AskCard({ ask, title, kindLabel, answer, onAnswered }: { ask: AskInfo; title?: string; kindLabel?: string; answer?: (option: string) => Promise<void>; onAnswered?: () => void }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const send = (reply: Parameters<typeof runtimeAnswer>[2]) => {
    setBusy(true);
    setError(null);
    const sent = answer && reply.option ? answer(reply.option) : runtimeAnswer(ask.session_id, ask.id, reply);
    void sent.then(
      () => {
        setBusy(false);
        onAnswered?.();
      },
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  const onKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (ask.kind !== "approval" || busy || event.target instanceof HTMLInputElement) return;
    const index = event.key === "Enter" ? 0 : Number.parseInt(event.key, 10) - 1;
    const option = Number.isInteger(index) && index >= 0 ? ask.options[index] : undefined;
    if (option) {
      event.preventDefault();
      send({ option: option.id });
    }
  };
  return (
    <div
      tabIndex={0}
      onKeyDown={onKey}
      className="flex flex-col gap-2 rounded-lg border border-warning/30 bg-warning-soft/40 p-3 outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="ask-card"
      data-ask={ask.id}
      aria-label={`${kindLabel ?? kindWords(ask)}: ${ask.summary}${title ? ` — ${title}` : ""}`}
    >
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="text-[12px] font-semibold uppercase tracking-wider text-warning">
          {kindLabel ?? kindWords(ask)}
          {title ? ` · ${title}` : ""}
        </span>
        {kindLabel ? (
          <span className="text-[14px] text-ink">{ask.summary}</span>
        ) : ask.kind === "approval" ? (
          <code className="truncate font-mono text-[12px] text-ink" title={ask.summary}>
            {ask.summary}
          </code>
        ) : (
          ask.parts.length === 0 && <span className="text-[12px] text-ink">{ask.summary}</span>
        )}
        {ask.reason && <span className="whitespace-pre-wrap text-[13px] text-muted">{ask.reason}</span>}
      </div>
      {ask.kind === "approval" ? (
        <Approval ask={ask} busy={busy} answer={(option) => send({ option })} />
      ) : ask.parts.length > 0 ? (
        <QuestionForm ask={ask} busy={busy} reply={(replies) => send({ replies })} />
      ) : (
        <p className="text-[11px] text-muted" data-testid="ask-in-terminal">Answer it in its terminal: Go to it.</p>
      )}
      {error && (
        <p role="alert" className="text-[12px] text-danger">
          {error}
        </p>
      )}
    </div>
  );
}

/** Every open ask of the driven sessions, together (Needs you's batch
 * view): small approvals answered one after another without leaving
 * Home. `titleOf` names each one's session. */
export function AskBatch({ titleOf }: { titleOf: (sessionId: string) => string | undefined }): JSX.Element | null {
  const asks = useOpenAsks().filter((ask) => ask.options.length > 0 || ask.parts.length > 0);
  if (asks.length === 0) return null;
  return (
    <div className="flex flex-col gap-2" data-testid="ask-batch">
      <span className="text-[11px] text-muted">
        {asks.length === 1 ? "1 question from a session" : `${asks.length} questions from your sessions`}
        {" · answer here, or go to the session"}
      </span>
      {asks.map((ask) => (
        <AskCard key={`${ask.session_id}:${ask.id}`} ask={ask} title={titleOf(ask.session_id)} />
      ))}
    </div>
  );
}
