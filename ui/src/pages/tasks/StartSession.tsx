/**
 * E19 (AJ-E1903 Req.3, D-31) — "Start a session for this task": you pick
 * the launch settings (and so the account), then Start. Nothing is picked
 * by usage: the list is yours in name order, and the only choice made for
 * you is your project's own default account, when you set one. An agent
 * can ask for a session; only this click starts one.
 */
import { useEffect, useId, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { errorText, launchRecipesList, type LaunchRecipeInfo } from "../../lib/bridge";
import { tasksStart, type TaskStartedInfo } from "../../lib/ipc/tasks";
import { Picker } from "../../components/controls";

export function StartSession({
  taskId,
  needId = null,
  defaultAccountId = null,
  onStarted,
  onCancel,
  compact = false,
}: {
  taskId: string;
  /** An agent's request this answers. */
  needId?: string | null;
  /** Your project's default account, when you set one. */
  defaultAccountId?: string | null;
  onStarted?: (started: TaskStartedInfo) => void;
  onCancel?: () => void;
  /** One line (Needs you). */
  compact?: boolean;
}): JSX.Element {
  const id = useId();
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[] | null>(null);
  const [chosen, setChosen] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void launchRecipesList().then(
      (rows) => {
        if (!alive) return;
        const sorted = [...rows].sort((a, b) => `${a.name} ${a.account_label}`.localeCompare(`${b.name} ${b.account_label}`));
        setProfiles(sorted);
        const own = sorted.find((p) => p.account_id === defaultAccountId) ?? (sorted.length === 1 ? sorted[0] : undefined);
        setChosen((was) => was || own?.id || "");
      },
      (cause) => alive && setError(errorText(cause)),
    );
    return () => {
      alive = false;
    };
  }, [defaultAccountId]);
  const start = () => {
    if (!chosen || busy) return;
    setBusy(true);
    setError(null);
    tasksStart(taskId, chosen, needId).then(
      (started) => {
        setBusy(false);
        onStarted?.(started);
      },
      (cause) => {
        setBusy(false);
        setError(errorText(cause));
      },
    );
  };
  return (
    <form
      className={"flex flex-wrap items-center gap-2 " + (compact ? "" : "rounded-xl border border-hairline bg-surface-2/60 p-3")}
      aria-label="Start a session for this task"
      data-testid="task-start"
      onSubmit={(event) => {
        event.preventDefault();
        start();
      }}
    >
      <span aria-hidden="true" className="text-meta text-muted">
        On
      </span>
      <Picker
        id={`${id}-profile`}
        ariaLabel="On"
        heading="Launch settings"
        size="sm"
        value={chosen || null}
        placeholder={profiles === null ? "Loading…" : "Pick launch settings"}
        disabled={profiles === null || busy}
        onChange={(next) => setChosen(next)}
        options={(profiles ?? []).map((profile) => ({ value: profile.id, label: `${profile.name} · ${profile.account_label}` }))}
        className="max-w-[280px] flex-1"
        testId="task-start-profile"
      />
      <Button type="submit" size="sm" variant="primary" disabled={!chosen || busy} data-testid="task-start-go">
        {busy ? "Starting…" : "Start session"}
      </Button>
      {onCancel && (
        <Button size="sm" variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
      )}
      {!compact && <p className="basis-full text-label text-muted">It starts in the pool's project with the task as its first prompt, and the task is its own. You pick the account; nothing starts by itself.</p>}
      {error && (
        <p role="alert" className="basis-full text-meta text-danger">
          {error}
        </p>
      )}
    </form>
  );
}
