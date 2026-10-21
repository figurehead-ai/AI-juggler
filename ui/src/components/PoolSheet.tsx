import { useState, type JSX } from "react";

import {
  DEFAULT_POOL_ID,
  errorText,
  poolCreate,
  poolDelete,
  poolMove,
  poolRename,
  poolSetGrant,
  poolSetRole,
  type InstanceInfo,
  type PoolInfo,
  type PoolsInfo,
  type ProjectInfo,
} from "../lib/bridge";
import { CAPABILITY_WORDS, ROLES, poolById, roleOf, scopeWords, sessionsIn, type PoolRole } from "../lib/pools";
import { placeOf } from "../screens/Home";
import { Field, Input, Modal, Select } from "./form";
import { Button } from "./primitives";

/** What the sheet is open on. */
export type PoolSheetTarget =
  | { mode: "create"; sessionId?: string; projectId?: string | null }
  | { mode: "pool"; poolId: string };

/**
 * The pool sheet (AJ-E1702): make a pool, or open one to rename it, set
 * roles, add and take out sessions, say what each role may do, or delete
 * it. The user owns every pool; nothing here starts, stops or types into
 * a session. Details (the grants) wait behind a disclosure.
 */
export function PoolSheet({
  target,
  pools,
  sessions,
  projects,
  onClose,
  onChanged,
}: {
  target: PoolSheetTarget;
  pools: PoolsInfo;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  onClose: () => void;
  /** A change went through: the new picture, and the pool it was about. */
  onChanged: (info: PoolsInfo, poolId: string | null) => void;
}): JSX.Element {
  if (target.mode === "create") {
    return (
      <CreatePool
        pools={pools}
        session={sessions.find((row) => row.id === target.sessionId) ?? null}
        projectId={target.projectId ?? null}
        sessions={sessions}
        projects={projects}
        onClose={onClose}
        onChanged={onChanged}
      />
    );
  }
  const pool = poolById(pools, target.poolId);
  if (!pool) {
    return (
      <Modal title="Pool" onClose={onClose}>
        <p className="text-sm text-muted">That pool isn't there any more.</p>
      </Modal>
    );
  }
  return <OnePool pool={pool} pools={pools} sessions={sessions} projects={projects} onClose={onClose} onChanged={onChanged} />;
}

function CreatePool({
  pools,
  session,
  projectId,
  sessions,
  projects,
  onClose,
  onChanged,
}: {
  pools: PoolsInfo;
  session: InstanceInfo | null;
  projectId: string | null;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  onClose: () => void;
  onChanged: (info: PoolsInfo, poolId: string | null) => void;
}): JSX.Element {
  const [name, setName] = useState("");
  const [project, setProject] = useState<string>(projectId ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const where = session ? placeOf(session, projects) : null;
  return (
    <Modal title="New pool" subtitle="Sessions that work together." onClose={onClose}>
      <form
        className="flex flex-col gap-4"
        data-testid="pool-create"
        onSubmit={async (event) => {
          event.preventDefault();
          setBusy(true);
          setError(null);
          try {
            let info = await poolCreate(name, project === "" ? null : project);
            const made = info.pools.find((pool) => !pools.pools.some((old) => old.id === pool.id)) ?? null;
            if (made && session) info = await poolMove(session.id, made.id);
            onChanged(info, made?.id ?? null);
          } catch (cause) {
            setError(errorText(cause));
            setBusy(false);
          }
        }}
      >
        <Field label="Name">
          <Input
            autoFocus
            value={name}
            placeholder="Checkout redesign"
            maxLength={48}
            onChange={(event) => setName(event.target.value)}
            data-testid="pool-create-name"
          />
        </Field>
        {projects.length > 0 && (
          <Field label="For" hint="A pool for one project takes only that project's sessions.">
            <Select value={project} onChange={(event) => setProject(event.target.value)} data-testid="pool-create-project">
              <option value="">Any project</option>
              {projects.map((row) => (
                <option key={row.id} value={row.id}>
                  {row.name}
                </option>
              ))}
            </Select>
          </Field>
        )}
        {where && (
          <p className="text-xs text-muted" data-testid="pool-create-session">
            <span className="text-ink">{where.title}</span>
            {where.detail ? ` · ${where.detail}` : ""} goes in it.
          </p>
        )}
        {!session && sessions.length === 0 && <p className="text-xs text-muted">Sessions can join it once you start some.</p>}
        {error && (
          <p className="text-xs text-danger" role="alert" data-testid="pool-error">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" type="submit" disabled={busy || name.trim() === ""} data-testid="pool-create-save">
            Make pool
          </Button>
        </div>
      </form>
    </Modal>
  );
}

function OnePool({
  pool,
  pools,
  sessions,
  projects,
  onClose,
  onChanged,
}: {
  pool: PoolInfo;
  pools: PoolsInfo;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  onClose: () => void;
  onChanged: (info: PoolsInfo, poolId: string | null) => void;
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

  const run = async (work: () => Promise<PoolsInfo>, poolId: string | null = pool.id) => {
    setBusy(true);
    setError(null);
    try {
      onChanged(await work(), poolId);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  const subtitle = isDefault
    ? "Every session that isn't in another pool."
    : [`${members.length} ${members.length === 1 ? "session" : "sessions"}`, forProject ? `for ${forProject.name}` : null].filter(Boolean).join(" · ");

  return (
    <Modal title={pool.name} subtitle={subtitle} onClose={onClose}>
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
              {isDefault ? "Every session is in another pool." : "No sessions yet. Add one below, or from a session's menu in the list."}
            </p>
          ) : (
            <ul className="flex flex-col gap-1" data-testid="pool-members">
              {members.map((session) => {
                const where = placeOf(session, projects);
                const named = [where.title, where.detail].filter(Boolean).join(" · ");
                const role = roleOf(pools, session.id);
                return (
                  <li key={session.id} className="flex min-w-0 items-center gap-2 rounded-lg border border-hairline/60 px-2.5 py-1.5" data-testid="pool-member">
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate text-[12.5px] text-ink">{where.title}</span>
                      {where.detail && <span className="truncate text-[11px] text-muted">{where.detail}</span>}
                    </span>
                    <select
                      aria-label={`Role for ${named}`}
                      value={role}
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
                    {where.title}
                    {where.detail ? ` · ${where.detail}` : ""}
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

        <details className="group rounded-lg border border-hairline/60 px-2.5 py-1.5" data-testid="pool-grants">
          <summary className="cursor-pointer select-none text-[12px] text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
            What each role may do
          </summary>
          <p className="mt-1.5 text-[11px] leading-relaxed text-muted">
            Agents can't reach pools yet; these apply once they can. You can always do everything here, and only you can read everyone's messages.
          </p>
          <table className="mt-2 w-full text-[11.5px]">
            <thead>
              <tr className="text-faint">
                <th className="py-1 text-left font-normal">
                  <span className="sr-only">Capability</span>
                </th>
                {ROLES.map((entry) => (
                  <th key={entry.id} scope="col" className="w-16 py-1 text-center font-normal">
                    {entry.label}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {pool.grants.map((grant) => {
                const words = CAPABILITY_WORDS[grant.capability] ?? grant.capability;
                return (
                  <tr key={grant.capability} className="border-t border-hairline/40" data-testid="pool-grant">
                    <th scope="row" className="py-1 pr-2 text-left font-normal text-ink">
                      {words}
                    </th>
                    {ROLES.map((entry) => (
                      <td key={entry.id} className="py-1 text-center">
                        <input
                          type="checkbox"
                          checked={grant[entry.id]}
                          disabled={busy || grant.fixed}
                          aria-label={`${entry.label}: ${words}`}
                          onChange={(event) => void run(() => poolSetGrant(pool.id, grant.capability, entry.id, event.target.checked))}
                          className="size-3.5 accent-accent"
                        />
                      </td>
                    ))}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </details>

        {error && (
          <p className="text-xs text-danger" role="alert" data-testid="pool-error">
            {error}
          </p>
        )}

        <div className="flex items-center justify-between gap-2">
          {!isDefault ? (
            confirmDelete ? (
              <span className="flex min-w-0 flex-wrap items-center gap-2 text-xs text-muted" data-testid="pool-delete-confirm">
                <span>
                  {members.length > 0 ? `Its ${members.length === 1 ? "session goes" : `${members.length} sessions go`} back to All sessions; nothing else changes.` : "Nothing else changes."}
                </span>
                <Button size="sm" variant="danger" disabled={busy} onClick={() => void run(() => poolDelete(pool.id), null)} data-testid="pool-delete-yes">
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
            )
          ) : (
            <span className="text-[11px] text-faint">All sessions is always here.</span>
          )}
          <Button size="sm" variant="secondary" onClick={onClose} data-testid="pool-done">
            Done
          </Button>
        </div>
      </div>
    </Modal>
  );
}
