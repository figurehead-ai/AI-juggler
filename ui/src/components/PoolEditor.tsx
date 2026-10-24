import { useState, type JSX } from "react";

import { errorText, type InstanceInfo, type ProjectInfo } from "../lib/bridge";
import { poolDelete, poolMove, poolRename, poolSetRole, type PoolInfo, type PoolsInfo } from "../lib/ipc/pools";
import { DEFAULT_POOL_ID, ROLES, levelLabel, roleOf, scopeWords, sessionsIn, setPools, type PoolRole } from "../lib/pools";
import { PoolAccessPanel } from "./PoolAccess";
import { placeOf } from "../lib/project-words";
import { Field, Input, Modal } from "./form";
import { Button } from "./primitives";

/**
 * One pool (AJ-E1702): rename it, set roles, add and take out sessions,
 * say what agents may do (D-28), or delete it. The minimum shows; what
 * agents may do waits behind a disclosure. "All sessions" can't be renamed or
 * deleted.
 */
export function PoolEditor({
  pool,
  pools,
  sessions,
  projects,
  onClose,
}: {
  pool: PoolInfo;
  pools: PoolsInfo;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  onClose: () => void;
}): JSX.Element {
  const isDefault = pool.kind === "default";
  const [name, setName] = useState(pool.name);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const members = sessionsIn(pools, pool.id, sessions);
  const forProject = pool.project_id ? projects.find((row) => row.id === pool.project_id) : undefined;
  const addable = sessions.filter(
    (session) => !members.includes(session) && (pool.project_id === null || session.project_id === pool.project_id),
  );
  const scope = scopeWords(pool);

  const run = async (work: () => Promise<PoolsInfo>, closeAfter = false) => {
    setBusy(true);
    setError(null);
    try {
      setPools(await work());
      if (closeAfter) onClose();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  const count = `${members.length} ${members.length === 1 ? "session" : "sessions"}`;
  const subtitle = isDefault ? "Every session that isn't in another pool." : [count, forProject ? `for ${forProject.name}` : null].filter(Boolean).join(" · ");

  return (
    <Modal title={pool.name} subtitle={subtitle} onClose={onClose} testId="pool-editor">
      <div className="flex max-h-[70vh] flex-col gap-4 overflow-y-auto" data-testid="pool-sheet" data-pool={pool.id}>
        {!isDefault && (
          <form
            className="flex items-end gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              void run(() => poolRename(pool.id, name));
            }}
          >
            <div className="min-w-0 flex-1">
              <Field label="Name">
                <Input value={name} maxLength={48} onChange={(event) => setName(event.target.value)} data-testid="pool-name" />
              </Field>
            </div>
            {name.trim() !== pool.name && (
              <Button size="sm" variant="secondary" type="submit" disabled={busy || name.trim() === ""} data-testid="pool-rename">
                Rename
              </Button>
            )}
          </form>
        )}

        <section aria-label="Sessions" className="flex flex-col gap-1.5">
          <p className="text-[11px] uppercase tracking-[0.14em] text-faint">Sessions</p>
          {members.length === 0 ? (
            <p className="text-xs text-muted" data-testid="pool-empty">
              {isDefault ? "Every session is in another pool." : "No sessions yet. Add one below, or from a session's menu."}
            </p>
          ) : (
            <ul className="flex flex-col gap-1" data-testid="pool-members">
              {members.map((session) => {
                const where = placeOf(session, projects);
                const named = [where.title, where.detail].filter(Boolean).join(" · ");
                return (
                  <li key={session.id} className="flex min-w-0 items-center gap-2 rounded-lg border border-hairline/60 px-2.5 py-1.5" data-testid="pool-member">
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate text-[12.5px] text-ink">{where.title}</span>
                      {where.detail && <span className="truncate text-[11px] text-muted">{where.detail}</span>}
                    </span>
                    <select
                      aria-label={`Role for ${named}`}
                      value={roleOf(pools, session.id)}
                      disabled={busy}
                      onChange={(event) => void run(() => poolSetRole(session.id, event.target.value as PoolRole))}
                      className="h-7 shrink-0 rounded-lg border border-hairline bg-surface-2 px-1.5 text-[11.5px] text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
                      data-testid="pool-member-role"
                    >
                      {ROLES.map((entry) => (
                        <option key={entry.id} value={entry.id}>
                          {entry.label}
                        </option>
                      ))}
                    </select>
                    {!isDefault && (
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void run(() => poolMove(session.id, DEFAULT_POOL_ID))}
                        aria-label={`Take out ${named}`}
                        title="Back to All sessions"
                        className="shrink-0 rounded-md px-1.5 py-0.5 text-[11px] text-muted outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
                        data-testid="pool-member-out"
                      >
                        Take out
                      </button>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
          {!isDefault && addable.length > 0 && (
            <select
              aria-label={`Add a session to ${pool.name}`}
              value=""
              disabled={busy}
              onChange={(event) => {
                const id = event.target.value;
                if (id) void run(() => poolMove(id, pool.id));
              }}
              className="h-8 rounded-lg border border-dashed border-hairline bg-transparent px-2 text-[11.5px] text-muted outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
              data-testid="pool-add"
            >
              <option value="">Add a session…</option>
              {addable.map((session) => {
                const where = placeOf(session, projects);
                return (
                  <option key={session.id} value={session.id}>
                    {[where.title, where.detail].filter(Boolean).join(" · ")}
                  </option>
                );
              })}
            </select>
          )}
        </section>

        {scope && (
          <p className={"text-xs " + (isDefault ? "text-muted" : "text-warning")} data-testid="pool-scope">
            {scope}
          </p>
        )}

        <details className="rounded-lg border border-hairline/60 px-2.5 py-1.5" data-testid="pool-access-fold">
          <summary className="cursor-pointer select-none text-[12px] text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
            What agents may do: <span className="text-muted">{levelLabel(pool.access.level)}</span>
          </summary>
          <div className="mt-2">
            <PoolAccessPanel pool={pool} />
          </div>
        </details>

        {error && (
          <p className="text-xs text-danger" role="alert" data-testid="pool-error">
            {error}
          </p>
        )}

        <div className="flex items-center justify-between gap-2">
          {isDefault ? (
            <span className="text-[11px] text-faint">All sessions is always here.</span>
          ) : confirmDelete ? (
            <span className="flex min-w-0 flex-wrap items-center gap-2 text-xs text-muted" data-testid="pool-delete-confirm">
              <span>
                {members.length > 0
                  ? `Its ${members.length === 1 ? "session goes" : `${members.length} sessions go`} back to All sessions; nothing else changes.`
                  : "Nothing else changes."}
              </span>
              <Button size="sm" variant="danger" disabled={busy} onClick={() => void run(() => poolDelete(pool.id), true)} data-testid="pool-delete-yes">
                Delete
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setConfirmDelete(false)}>
                Keep
              </Button>
            </span>
          ) : (
            <Button size="sm" variant="ghost" className="text-danger" onClick={() => setConfirmDelete(true)} data-testid="pool-delete">
              Delete pool
            </Button>
          )}
          <Button size="sm" variant="secondary" onClick={onClose} data-testid="pool-done">
            Done
          </Button>
        </div>
      </div>
    </Modal>
  );
}
