import { useState, type JSX } from "react";

import { errorText, type InstanceInfo, type ProjectInfo } from "../lib/bridge";
import { poolCreate, poolMove, type PoolsInfo } from "../lib/ipc/pools";
import { defaultPoolIdOf, hasUserPools, openPoolSheet, poolById, poolIdOf, setPools, type PoolSheetTarget } from "../lib/pools";
import { placeOf } from "../lib/project-words";
import { Field, Input, Modal, Select } from "./form";
import { PoolEditor } from "./PoolEditor";
import { Button } from "./primitives";

/**
 * The pool sheet (AJ-E1702), loaded on first use: make a pool, move a
 * session to another, or open one pool ({@link PoolEditor}). The user
 * owns every pool; nothing here starts, stops or types into a session.
 */
export function PoolSheet({
  target,
  pools,
  sessions,
  projects,
}: {
  target: PoolSheetTarget;
  pools: PoolsInfo;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
}): JSX.Element {
  const close = () => openPoolSheet(null);
  if (target.mode === "pool") {
    const pool = poolById(pools, target.poolId);
    if (!pool) {
      return (
        <Modal title="Pool" onClose={close}>
          <p className="text-sm text-muted">That pool isn't there any more.</p>
        </Modal>
      );
    }
    return <PoolEditor pool={pool} pools={pools} sessions={sessions} projects={projects} onClose={close} />;
  }
  const session = sessions.find((row) => row.id === target.sessionId) ?? null;
  // With no pool of yours yet, moving a session means making one.
  if (target.mode === "move" && session && hasUserPools(pools)) {
    return <MovePool session={session} pools={pools} projects={projects} onClose={close} />;
  }
  return (
    <CreatePool
      pools={pools}
      session={session}
      projectId={target.mode === "create" ? (target.projectId ?? null) : null}
      sessions={sessions}
      projects={projects}
      onClose={close}
    />
  );
}

function CreatePool({
  pools,
  session,
  projectId,
  sessions,
  projects,
  onClose,
}: {
  pools: PoolsInfo;
  session: InstanceInfo | null;
  projectId: string | null;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  onClose: () => void;
}): JSX.Element {
  const [name, setName] = useState("");
  const [project, setProject] = useState<string>(projectId ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const where = session ? placeOf(session, projects) : null;
  return (
    <Modal title="New pool" subtitle="Sessions that work together." onClose={onClose} testId="pool-create-sheet">
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
            setPools(info);
            openPoolSheet(made ? { mode: "pool", poolId: made.id } : null);
          } catch (cause) {
            setError(errorText(cause));
            setBusy(false);
          }
        }}
      >
        <Field label="Name">
          <Input
            autoFocus
            data-autofocus
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

/** A session's "Move to pool…": every pool it could go in, one click
 * each, and a new one. */
function MovePool({
  session,
  pools,
  projects,
  onClose,
}: {
  session: InstanceInfo;
  pools: PoolsInfo;
  projects: ProjectInfo[];
  onClose: () => void;
}): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const where = placeOf(session, projects);
  const here = poolIdOf(pools, session);
  // Your pools it can join, and its own project's default pool (D-33).
  const choices = pools.pools.filter((pool) =>
    pool.kind === "user" ? pool.project_id === null || pool.project_id === session.project_id : pool.id === defaultPoolIdOf(session.project_id),
  );
  const move = async (poolId: string) => {
    setBusy(true);
    setError(null);
    try {
      setPools(await poolMove(session.id, poolId));
      onClose();
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };
  return (
    <Modal title="Move to pool" subtitle={[where.title, where.detail].filter(Boolean).join(" · ")} onClose={onClose} testId="pool-move-sheet">
      <div className="flex flex-col gap-3">
        <ul className="overflow-hidden rounded-xl border border-hairline bg-surface" data-testid="pool-move-list">
          {choices.map((pool) => (
            <li key={pool.id} className="border-t border-hairline/60 first:border-t-0">
              <button
                type="button"
                disabled={busy || pool.id === here}
                aria-current={pool.id === here ? "true" : undefined}
                onClick={() => void move(pool.id)}
                className="flex w-full items-center justify-between gap-3 px-4 py-2.5 text-left text-sm text-ink outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-accent/60 disabled:cursor-default disabled:hover:bg-transparent"
                data-testid="pool-move-choice"
              >
                <span className="min-w-0 truncate">{pool.name}</span>
                {pool.id === here && <span className="shrink-0 text-xs text-faint">It's here</span>}
              </button>
            </li>
          ))}
        </ul>
        {error && (
          <p className="text-xs text-danger" role="alert" data-testid="pool-error">
            {error}
          </p>
        )}
        <div className="flex justify-between gap-2">
          <Button size="sm" variant="secondary" onClick={() => openPoolSheet({ mode: "create", sessionId: session.id })} data-testid="pool-move-new">
            New pool…
          </Button>
          <Button size="sm" variant="secondary" onClick={onClose}>
            Cancel
          </Button>
        </div>
      </div>
    </Modal>
  );
}
