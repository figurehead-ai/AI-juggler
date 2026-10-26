/**
 * AJ-E2303/E2307 — the body of a session AI Juggler drives (in place of
 * its terminal): how it runs, its open asks with the tool's own options,
 * its recent conversation (held in memory to show it, never stored: the
 * tool's transcript is the record), a message box, Interrupt, and Open in
 * terminal (a hand-over at the turn's end: one writer at a time).
 *
 * AJ-E2310: a session whose turn was cut off when AI Juggler stopped says
 * so, with Resume (its conversation resumed, nothing sent) and End.
 *
 * Until the 0.1.5 design system's conversation view lands, this reuses
 * the session page's own pieces (cards, chips, buttons).
 */
import { useState, type JSX } from "react";
import { Button, Card } from "../../components/primitives";
import { AskCard } from "../../components/AskCard";
import { errorText, type InstanceInfo } from "../../lib/bridge";
import { runtimeEnd, runtimeHandover, runtimeInterrupt, runtimeResume, runtimeSend, useRuntimeView } from "../../lib/ipc/runtime";
import { useFolderTrust } from "../../components/FolderTrustAsk";
import { messagesTyping } from "../../lib/ipc/messages";

const ITEM_WORDS: Record<string, string> = {
  user_message: "You",
  agent_message: "Agent",
  command: "Ran",
  file_change: "Changed",
  tool: "Used",
  note: "Note",
  // E18 (D-29): a turn a delivered message started.
  delivered: "Message",
};

const STATE_WORDS: Record<string, string> = {
  working: "Working",
  idle: "Idle",
  asks_you: "Asks you",
  cut_off: "Cut off",
  ended: "Ended",
};

export function DrivenPanel({ session, onOpenSession }: { session: InstanceInfo; onOpenSession?: (id: string) => void }): JSX.Element {
  const view = useRuntimeView(session.id);
  const runtime = view?.runtime ?? session.runtime ?? null;
  const [text, setText] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Lab 2: the tool's folder-trust question before its terminal opens.
  const trust = useFolderTrust();
  const act = (what: string, run: () => Promise<unknown>) => {
    setBusy(what);
    setError(null);
    void run().then(
      () => setBusy(null),
      (cause) => {
        setBusy(null);
        setError(errorText(cause));
      },
    );
  };
  const cutOff = runtime?.cut_off ?? null;
  const state = cutOff ? "cut_off" : runtime?.state ?? null;
  // Ended, or cut off: nothing to send to until Resume.
  const ended = state === "ended" || cutOff !== null;
  return (
    <Card className="flex min-h-[280px] flex-1 flex-col gap-3 p-4" data-testid="driven-panel">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex min-w-0 flex-col">
          <span className="text-[12px] text-ink" data-testid="runtime-line">
            Runs through {runtime?.label ?? "its tool's protocol"}
            {runtime?.beta ? " (Beta)" : ""}
          </span>
          <span className="text-[11px] text-muted">
            {state ? STATE_WORDS[state] ?? state : "Starting"}
            {view?.model ? ` · ${view.model}` : ""}
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          {state === "working" && (
            <Button size="sm" variant="secondary" disabled={busy !== null} onClick={() => act("interrupt", () => runtimeInterrupt(session.id))}>
              Interrupt
            </Button>
          )}
          {!ended && (
            <Button
              size="sm"
              variant="secondary"
              disabled={busy !== null}
              title="Waits for the turn to end, then continues this conversation in its own terminal."
              data-testid="open-in-terminal"
              onClick={() =>
                act("handover", async () => {
                  if (!(await trust.confirm(session.launch_recipe_id, session.cwd))) return;
                  const next = await runtimeHandover(session.id, "terminal");
                  onOpenSession?.(next.session_id);
                })
              }
            >
              {busy === "handover" ? "Waiting for the turn to end…" : "Open in terminal"}
            </Button>
          )}
        </div>
      </div>
      {cutOff && (
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-hairline bg-surface-2 p-3" data-testid="runtime-cut-off">
          <p className="min-w-0 flex-1 text-[12px] text-ink">{cutOff}. Resume continues the conversation and sends nothing.</p>
          <Button size="sm" variant="primary" disabled={busy !== null} data-testid="runtime-resume" onClick={() => act("resume", () => runtimeResume(session.id).then((next) => onOpenSession?.(next.session_id)))}>
            {busy === "resume" ? "Resuming…" : "Resume"}
          </Button>
          <Button size="sm" variant="ghost" disabled={busy !== null} data-testid="runtime-end" onClick={() => act("end", () => runtimeEnd(session.id))}>
            End it
          </Button>
        </div>
      )}
      {runtime?.limits && runtime.limits.length > 0 && (
        <p className="text-[11px] text-muted" data-testid="runtime-limits">
          {runtime.limits
            .map((w) => `${w.window_mins === 300 ? "5 hours" : w.window_mins === 10080 ? "Weekly" : w.window_mins ? `${w.window_mins} min` : w.id}: ${Math.round(w.used_pct)}% used${w.resets_at_ms ? `, resets ${new Date(w.resets_at_ms).toLocaleString()}` : ""}`)
            .join(" · ")}
          {!runtime.limits.some((w) => w.window_mins === 300) && " · no 5-hour limit reported"}
        </p>
      )}
      {(runtime?.asks ?? []).map((ask) => (
        <AskCard key={ask.id} ask={ask} />
      ))}
      <ol className="flex max-h-[420px] flex-col gap-1.5 overflow-y-auto" aria-label="Recent conversation" data-testid="driven-items">
        {(view?.items ?? []).map((item, index) => (
          <li key={`${item.at_ms}-${index}`} className="flex gap-2 text-[12px]">
            <span className="w-14 shrink-0 text-[10px] uppercase tracking-wider text-muted">{ITEM_WORDS[item.kind] ?? item.kind}</span>
            <span className={"min-w-0 whitespace-pre-wrap break-words " + (item.kind === "command" ? "font-mono text-[11.5px] text-ink" : "text-ink")}>{item.text}</span>
          </li>
        ))}
        {view?.streaming && (
          <li className="flex gap-2 text-[12px]" aria-live="polite">
            <span className="w-14 shrink-0 text-[10px] uppercase tracking-wider text-muted">Agent</span>
            <span className="min-w-0 whitespace-pre-wrap break-words text-muted">{view.streaming}</span>
          </li>
        )}
      </ol>
      {!ended && (
        <form
          className="flex items-center gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            const message = text.trim();
            if (!message) return;
            act("send", () => runtimeSend(session.id, message).then(() => (setText(""), void messagesTyping(session.id, false))));
          }}
        >
          <input
            aria-label={runtime?.can_steer ? "Add to the running turn" : "Message"}
            placeholder={runtime?.can_steer ? "Add to what it's doing…" : "Message…"}
            className="h-9 min-w-0 flex-1 rounded-lg border border-hairline bg-surface-2 px-3 text-[13px] text-ink outline-none focus:border-accent/60"
            value={text}
            disabled={busy !== null || state === "asks_you"}
            onChange={(event) => {
              // E18: a delivered message waits while you type here.
              const next = event.target.value;
              if ((next.trim() !== "") !== (text.trim() !== "")) void messagesTyping(session.id, next.trim() !== "");
              setText(next);
            }}
          />
          <Button type="submit" size="md" variant="primary" disabled={busy !== null || !text.trim() || state === "asks_you"}>
            Send
          </Button>
        </form>
      )}
      {runtime?.error && <p className="text-[11px] text-warning">{runtime.error}</p>}
      {trust.element}
      {error && (
        <p role="alert" className="text-[11px] text-danger">
          {error}
        </p>
      )}
    </Card>
  );
}
