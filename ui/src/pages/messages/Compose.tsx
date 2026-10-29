/**
 * E18 (AJ-E1802) — you write to a member of the pool or to the whole pool.
 * A reply stays in its thread. "Deliver to their sessions" (on by default:
 * you're the one writing) also puts it into the sessions AI Juggler runs,
 * as their next prompt, between turns; a session in a terminal reads it
 * itself. Your word resets the thread's count of agent messages in a row.
 */
import { useId, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { errorText } from "../../lib/bridge";
import { messagesSend, type ThreadPage } from "../../lib/ipc/messages";
import { KINDS, KIND_WORDS } from "./words";
import { Checkbox, Picker } from "../../components/controls";

export interface Member {
  session_id: string;
  name: string;
}

export function Compose({
  poolId,
  members,
  threadId,
  to: fixedTo,
  onSent,
  onCancel,
}: {
  poolId: string;
  members: Member[];
  /** A reply in this thread. */
  threadId?: string;
  /** Always to this member (a session's own page). */
  to?: string;
  onSent: (page: ThreadPage) => void;
  onCancel?: () => void;
}): JSX.Element {
  const ids = useId();
  const [to, setTo] = useState(fixedTo ?? (members.length === 1 ? members[0]!.session_id : "pool"));
  const [subject, setSubject] = useState("");
  const [kind, setKind] = useState<string>("note");
  const [body, setBody] = useState("");
  const [deliver, setDeliver] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const reply = threadId !== undefined;
  const send = () => {
    const text = body.trim();
    if (!text || busy) return;
    setBusy(true);
    setError(null);
    messagesSend({ pool_id: poolId, to, thread_id: threadId ?? null, kind, subject: reply ? null : subject.trim() || null, body: text, deliver }).then(
      (page) => {
        setBusy(false);
        setBody("");
        setSubject("");
        onSent(page);
      },
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  return (
    <form
      className="flex flex-col gap-2.5 rounded-xl border border-hairline bg-surface-2/60 p-3"
      aria-label={reply ? "Reply in this thread" : "New message"}
      data-testid="message-compose"
      onSubmit={(event) => {
        event.preventDefault();
        send();
      }}
    >
      {!reply && (
        <div className="flex flex-wrap items-center gap-2">
          {!fixedTo && (
            <div className="flex items-center gap-2 text-meta text-muted">
              <span aria-hidden="true">To</span>
              <Picker
                ariaLabel="To"
                size="sm"
                value={to}
                onChange={(next) => setTo(next)}
                options={[{ value: "pool", label: "Everyone in the pool" }, ...members.map((member) => ({ value: member.session_id, label: member.name }))]}
                testId="compose-to"
              />
            </div>
          )}
          <input
            aria-label="Subject"
            placeholder="Subject"
            value={subject}
            maxLength={120}
            onChange={(event) => setSubject(event.target.value)}
            className="h-8 min-w-0 flex-1 rounded-lg border border-hairline bg-surface px-2.5 text-meta text-ink outline-none focus:border-accent/60"
          />
          <Picker
            ariaLabel="Kind"
            size="sm"
            value={kind}
            onChange={(next) => setKind(next)}
            options={KINDS.map((word) => ({ value: word, label: KIND_WORDS[word] }))}
          />
        </div>
      )}
      <textarea
        aria-label={reply ? "Your reply" : "Your message"}
        placeholder={reply ? "Reply…" : "Write to the pool…"}
        rows={reply ? 2 : 3}
        maxLength={4000}
        value={body}
        onChange={(event) => setBody(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
            event.preventDefault();
            send();
          }
        }}
        className="min-h-[56px] resize-y rounded-lg border border-hairline bg-surface px-3 py-2 text-body leading-relaxed text-ink outline-none focus:border-accent/60"
        data-testid="compose-body"
      />
      <div className="flex flex-wrap items-center justify-between gap-2">
        <label htmlFor={`${ids}-deliver`} className="flex items-center gap-2 text-meta text-muted" title="Into the sessions AI Juggler runs, as their next prompt, between turns. A session in a terminal reads its messages itself.">
          <Checkbox id={`${ids}-deliver`} checked={deliver} onChange={(event) => setDeliver(event.target.checked)} />
          Deliver to their sessions
        </label>
        <div className="flex items-center gap-2">
          {onCancel && (
            <Button size="sm" variant="ghost" onClick={onCancel}>
              Cancel
            </Button>
          )}
          <Button type="submit" size="sm" variant="primary" disabled={busy || !body.trim()} aria-keyshortcuts="Meta+Enter" data-testid="compose-send">
            {busy ? "Sending…" : "Send"}
            <span aria-hidden="true" className="opacity-75">⌘↵</span>
          </Button>
        </div>
      </div>
      {error && (
        <p role="alert" className="text-meta text-danger">
          {error}
        </p>
      )}
    </form>
  );
}
