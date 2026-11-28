import { useState, type JSX } from "react";

import { errorText } from "../lib/bridge";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { poolCreate, poolMove, type PoolsInfo } from "../lib/ipc/pools";
import { defaultPoolIdOf, hasUserPools, openPoolSheet, poolIdOf, setPools, usePoolPlace, type PoolSheetTarget } from "../lib/pools";
import { placeOf } from "../lib/project-words";
import { Field, Input, Modal } from "./form";
import { useActionHost } from "./ItemMenu";
import { Button } from "./primitives";
import { Picker, RowChip } from "./controls";
import { SELECTED } from "./controls/selected";
import { IconProjects } from "./icons";
import { IconMove, IconPool } from "./icons-actions";
import { recentProjects } from "../lib/pickers";

/**
 * The pool sheet (AJ-E1702), loaded on first use: make a pool, or move a
 * session to another. A pool itself is changed on its canvas (0.3). The
 * user owns every pool; nothing here starts, stops or types into a
 * session.
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
  const host = useActionHost();
  const [name, setName] = useState("");
  const [project, setProject] = useState<string>(projectId ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const where = session ? placeOf(session, projects) : null;
  return (
    <Modal title="New pool" subtitle="Sessions that work together." icon={IconPool} onClose={onClose} testId="pool-create-sheet"
      footer={
        <>
            <Button size="sm" variant="ghost" onClick={onClose}>
              Cancel
            </Button>
            <Button size="sm" variant="primary" type="submit" form="poolsheet-form" disabled={busy || name.trim() === ""} data-testid="pool-create-save">
              Make pool
            </Button>
        </>
      }
    >
      <form id="poolsheet-form"
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
            // The new pool's page opens (the rail stays as you set it; its
            // View offers Group by Pool from now on).
            openPoolSheet(null);
            if (made) host.openPool?.(made.id);
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
            <Picker
              ariaLabel="For"
              value={project}
              onChange={setProject}
              testId="pool-create-project"
              className="self-start"
              options={[{ value: "", label: "Any project", pinned: true, icon: IconProjects }, ...projects.map((row) => ({ value: row.id, label: row.name, icon: IconProjects }))]}
              groups={[{ label: "Recent", values: recentProjects(projects) }]}
              searchLabel="Search projects"
              allHeading="All projects"
            />
          </Field>
        )}
        {where && (
          <p className="text-meta text-ink-2" data-testid="pool-create-session">
            <span className="text-ink">{where.title}</span>
            {where.detail ? ` · ${where.detail}` : ""} goes in it.
          </p>
        )}
        {!session && sessions.length === 0 && <p className="text-meta text-ink-2">Sessions can join it once you start some.</p>}
        {error && (
          <p className="text-label text-danger" role="alert" data-testid="pool-error">
            {error}
          </p>
        )}
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
  // Read alone too: a session past the working set (opened from History).
  const here = usePoolPlace(session.id)?.pool_id ?? poolIdOf(pools, session);
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
    <Modal title="Move to pool" subtitle={[where.title, where.detail].filter(Boolean).join(" · ")} icon={IconMove} onClose={onClose} testId="pool-move-sheet">
      <div className="flex flex-col gap-3">
        {/* AJ-E1558 (D-126, PAT-SURF): the pools on their inset tone, no
            outline or rules; each with its chip; the one it is in a fill. */}
        <ul className="flex flex-col gap-0.5 rounded-xl bg-surface-2 p-1.5" data-testid="pool-move-list">
          {choices.map((pool) => (
            <li key={pool.id}>
              <button
                type="button"
                disabled={busy || pool.id === here}
                aria-current={pool.id === here ? "true" : undefined}
                onClick={() => void move(pool.id)}
                className={
                  "flex w-full items-center gap-3 rounded-[10px] px-2.5 py-2 text-left text-body text-ink outline-none focus-visible:ring-2 focus-visible:ring-accent/60 disabled:cursor-default " +
                  (pool.id === here ? SELECTED : "hover:bg-hover")
                }
                data-testid="pool-move-choice"
              >
                <RowChip icon={IconPool} on={pool.id === here} />
                <span className="min-w-0 flex-1 truncate">{pool.name}</span>
                {pool.id === here && <span className="shrink-0 text-label font-normal text-muted">It's here</span>}
              </button>
            </li>
          ))}
        </ul>
        {error && (
          <p className="text-label text-danger" role="alert" data-testid="pool-error">
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
