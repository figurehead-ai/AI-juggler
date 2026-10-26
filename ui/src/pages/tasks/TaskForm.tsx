/**
 * E19 (AJ-E1903) — you add a task, a story or an epic to the pool. The
 * title and what it asks first; when it's done, its parent, what it
 * waits on and the issue it comes from behind "More". It starts ready for
 * anyone (or to do), and no one's.
 */
import { useId, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { errorText } from "../../lib/bridge";
import { tasksCreate, type TaskDetail, type TaskInfo } from "../../lib/ipc/tasks";
import { KIND_WORDS, PRIORITIES, PRIORITY_WORDS } from "./words";

const FIELD = "h-8 rounded-lg border border-hairline bg-surface px-2.5 text-[13px] text-ink outline-none focus:border-accent/60";

export function TaskForm({
  poolId,
  plan,
  parentId = null,
  onMade,
  onCancel,
}: {
  poolId: string;
  /** The pool's epics and stories (a parent to pick). */
  plan: TaskInfo[];
  parentId?: string | null;
  onMade: (detail: TaskDetail) => void;
  onCancel?: () => void;
}): JSX.Element {
  const ids = useId();
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [acceptance, setAcceptance] = useState("");
  const [kind, setKind] = useState("task");
  const [parent, setParent] = useState(parentId ?? "");
  const [priority, setPriority] = useState("normal");
  const [status, setStatus] = useState("ready");
  const [waits, setWaits] = useState("");
  const [url, setUrl] = useState("");
  const [more, setMore] = useState(parentId !== null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const parents = plan.filter((item) => (kind === "story" ? item.kind === "epic" : kind === "task"));
  const save = () => {
    if (!title.trim() || busy) return;
    setBusy(true);
    setError(null);
    tasksCreate({
      pool_id: poolId,
      title: title.trim(),
      body,
      acceptance,
      kind,
      parent_id: kind === "epic" ? null : parent || null,
      priority,
      status,
      url: url.trim() || null,
      blocked_by: waits
        .split(/[\s,]+/)
        .map((word) => word.trim())
        .filter(Boolean),
    }).then(
      (detail) => {
        setBusy(false);
        onMade(detail);
      },
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  return (
    <form
      className="flex flex-col gap-3"
      aria-label="New task"
      data-testid="task-form"
      onSubmit={(event) => {
        event.preventDefault();
        save();
      }}
    >
      <div className="flex flex-wrap items-center gap-2">
        <input aria-label="Title" placeholder="What needs doing" value={title} maxLength={120} autoFocus onChange={(event) => setTitle(event.target.value)} className={FIELD + " h-9 min-w-[240px] flex-1 text-[14px]"} data-testid="task-form-title" />
        <select aria-label="Kind" value={kind} onChange={(event) => setKind(event.target.value)} className={FIELD}>
          {(["task", "story", "epic"] as const).map((word) => (
            <option key={word} value={word}>
              {KIND_WORDS[word]}
            </option>
          ))}
        </select>
        <select aria-label="Priority" value={priority} onChange={(event) => setPriority(event.target.value)} className={FIELD}>
          {PRIORITIES.map((word) => (
            <option key={word} value={word}>
              {PRIORITY_WORDS[word]} priority
            </option>
          ))}
        </select>
      </div>
      <textarea aria-label="What it asks" placeholder="What it asks, and anything the one who takes it should know" rows={3} maxLength={4000} value={body} onChange={(event) => setBody(event.target.value)} className="min-h-[64px] resize-y rounded-lg border border-hairline bg-surface px-3 py-2 text-[14px] leading-relaxed text-ink outline-none focus:border-accent/60" />
      <button type="button" aria-expanded={more} onClick={() => setMore((was) => !was)} className="self-start text-[13px] text-muted underline decoration-hairline underline-offset-2 hover:text-ink" data-testid="task-form-more">
        {more ? "Fewer details" : "More: when it's done, its epic, what it waits on, an issue link"}
      </button>
      {more && (
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="flex flex-col gap-1 text-[13px] text-muted sm:col-span-2">
            Done when
            <textarea rows={2} maxLength={2000} value={acceptance} placeholder="- tests pass&#10;- the docs say how" onChange={(event) => setAcceptance(event.target.value)} className="resize-y rounded-lg border border-hairline bg-surface px-3 py-2 text-[14px] text-ink outline-none focus:border-accent/60" />
          </label>
          {kind !== "epic" && (
            <label className="flex flex-col gap-1 text-[13px] text-muted">
              Part of
              <select value={parent} onChange={(event) => setParent(event.target.value)} className={FIELD}>
                <option value="">Nothing above it</option>
                {parents.map((item) => (
                  <option key={item.id} value={item.id}>
                    {KIND_WORDS[item.kind]} · {item.title}
                  </option>
                ))}
              </select>
            </label>
          )}
          <label htmlFor={`${ids}-status`} className="flex flex-col gap-1 text-[13px] text-muted">
            Starts
            <select id={`${ids}-status`} value={status} onChange={(event) => setStatus(event.target.value)} className={FIELD}>
              <option value="ready">Ready for anyone</option>
              <option value="todo">To do (not ready yet)</option>
            </select>
          </label>
          <label className="flex flex-col gap-1 text-[13px] text-muted">
            Waits on
            <input value={waits} placeholder="aj-7f3c, aj-1a2b" onChange={(event) => setWaits(event.target.value)} className={FIELD + " font-mono"} />
          </label>
          <label className="flex flex-col gap-1 text-[13px] text-muted">
            Issue link
            <input value={url} type="url" placeholder="https://github.com/…/issues/42" onChange={(event) => setUrl(event.target.value)} className={FIELD} />
          </label>
        </div>
      )}
      <div className="flex items-center justify-end gap-2">
        {onCancel && (
          <Button size="sm" variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
        )}
        <Button type="submit" size="sm" variant="primary" disabled={busy || !title.trim()} data-testid="task-form-save">
          {busy ? "Adding…" : `Add ${KIND_WORDS[kind]!.toLowerCase()}`}
        </Button>
      </div>
      {error && (
        <p role="alert" className="text-[13px] text-danger">
          {error}
        </p>
      )}
    </form>
  );
}
