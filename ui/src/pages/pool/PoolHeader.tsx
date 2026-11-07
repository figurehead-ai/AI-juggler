/**
 * A pool page's header: its name (yours rename in place; a project's own
 * pool keeps its project's), what it is in one line, and its controls —
 * the canvas or the list, Tasks and Messages over the canvas, and what
 * agents may do, changed right here.
 */
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";

import { IconPool } from "../../components/icons-actions";
import { IconMessage, IconTasks } from "../../components/icons-canvas";
import { errorText } from "../../lib/bridge";
import { poolRename, poolSetAccess, type PoolInfo } from "../../lib/ipc/pools";
import { LEVELS, setPools } from "../../lib/pools";
import { Picker, Segmented } from "../../components/controls";
import { Chip } from "../../components/primitives";

export type Panel = "tasks" | "messages" | null;
export type Mode = "canvas" | "list";

/** The pool whose name is edited as soon as its page opens (New pool). */
let nameNext: string | null = null;
export function editNameOf(poolId: string): void {
  nameNext = poolId;
}

function Name({ pool }: { pool: PoolInfo }): JSX.Element {
  const [editing, setEditing] = useState(() => nameNext === pool.id);
  const [words, setWords] = useState(pool.name);
  const [error, setError] = useState<string | null>(null);
  const input = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    if (nameNext === pool.id) nameNext = null;
  }, [pool.id]);
  useEffect(() => {
    if (editing) input.current?.select();
  }, [editing]);
  useEffect(() => setWords(pool.name), [pool.name]);
  const save = async () => {
    if (words.trim() === pool.name) return setEditing(false);
    try {
      setPools(await poolRename(pool.id, words));
      setEditing(false);
      setError(null);
    } catch (cause) {
      setError(errorText(cause));
    }
  };
  if (pool.kind === "default") {
    return (
      <h2 tabIndex={-1} className="truncate text-lg font-semibold text-ink outline-none" title={pool.name} data-testid="page-name">
        {pool.name}
      </h2>
    );
  }
  return editing ? (
    <span className="flex min-w-0 flex-col">
      <input autoCorrect="off" autoCapitalize="off" spellCheck={false}
        ref={input}
        value={words}
        maxLength={48}
        aria-label="The pool's name"
        aria-invalid={error !== null}
        aria-describedby={error ? "pool-name-error" : undefined}
        onChange={(event) => setWords(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") void save();
          if (event.key === "Escape") (setWords(pool.name), setEditing(false), setError(null));
        }}
        onBlur={() => void save()}
        className="h-8 w-72 max-w-full rounded-lg border border-accent bg-surface-2 px-2 text-lg font-semibold text-ink outline-none"
        data-testid="pool-name-input"
      />
      {error && (
        <span id="pool-name-error" className="text-label text-danger" role="alert">
          {error}
        </span>
      )}
    </span>
  ) : (
    <button type="button" onClick={() => setEditing(true)} className="group flex min-w-0 items-center gap-1.5 rounded-lg px-1 text-left outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60" title="Rename" data-testid="pool-name">
      <h2 className="truncate text-lg font-semibold text-ink" data-testid="page-name">
        {pool.name}
      </h2>
      <span className="text-label text-faint opacity-0 group-hover:opacity-100 group-focus-visible:opacity-100">Rename</span>
    </button>
  );
}

function Toggle({ on, onClick, icon, children, testId }: { on: boolean; onClick: () => void; icon: ReactNode; children: string; testId: string }): JSX.Element {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={onClick}
      className={"flex h-8 items-center gap-1.5 rounded-lg px-2.5 text-meta outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " + (on ? "bg-accent-soft text-accent-hi" : "text-muted hover:bg-surface-2 hover:text-ink")}
      data-testid={testId}
    >
      {icon}
      {children}
    </button>
  );
}

/** AJ-E3104 R1: where a pool you made works, as a chip that opens its
 * scope; a project's own pool says its project in its title instead. */
function ScopeChip({ words, onOpen }: { words: string; onOpen: () => void }): JSX.Element {
  return (
    <button
      type="button"
      onClick={onOpen}
      title="Change where it works"
      className="flex h-6 shrink-0 items-center gap-1 rounded-md border border-hairline px-1.5 text-label text-ink-2 outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="pool-header-scope"
    >
      <span className="text-muted">Scope</span>
      <span className="max-w-48 truncate">{words}</span>
    </button>
  );
}

export function PoolHeader({
  pool,
  facts,
  scope,
  mode,
  panel,
  onMode,
  onPanel,
}: {
  pool: PoolInfo;
  facts: ReactNode;
  /** A pool you made: where it works, in words, and its scope sheet. */
  scope?: { words: string; onOpen: () => void };
  mode: Mode;
  panel: Panel;
  onMode: (mode: Mode) => void;
  onPanel: (panel: Panel) => void;
}): JSX.Element {
  const [error, setError] = useState<string | null>(null);
  return (
    <header className="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-hairline px-4 py-2.5" data-testid="pool-header">
      <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-accent-soft text-accent-hi" aria-hidden="true">
        <IconPool width={18} height={18} />
      </span>
      {/* The name never gives way to the controls: they wrap to a row of
          their own first (at 1024 px the name read "payments-api · all
          se…" beside them). */}
      <div className="flex min-w-0 max-w-full flex-[1_0_auto] flex-col">
        {/* AJ-E2803 R3: a page below a screen says where it sits. */}
        <nav aria-label="Breadcrumb" className="flex min-w-0 items-center gap-1 text-meta text-muted" data-testid="page-crumbs">
          <button
            type="button"
            onClick={() => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "pools" } }))}
            data-testid="crumb-pools"
            className="rounded outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
          >
            Pools
          </button>
          <span aria-hidden="true">›</span>
          <span className="truncate font-medium" aria-current="page">
            {pool.name}
          </span>
        </nav>
        <div className="flex min-w-0 items-center gap-2">
          <Name pool={pool} />
          {pool.kind === "default" && <span className="shrink-0 rounded-md border border-hairline px-1.5 text-label text-muted">Project pool</span>}
          {scope && <ScopeChip words={scope.words} onOpen={scope.onOpen} />}
          {pool.sealed && (
            <span title="Kept apart: its sessions are in no other pool" data-testid="pool-header-sealed" className="shrink-0">
              <Chip tone="danger">sealed</Chip>
            </span>
          )}
        </div>
        <span className="truncate text-meta text-muted" data-testid="pool-page-count">
          {facts}
        </span>
      </div>
      <div className="flex flex-wrap items-center gap-1.5">
        <Segmented
          label="Show the pool as"
          value={mode}
          segments={[
            { value: "canvas", label: "Canvas", title: "Its members as nodes, who talks to whom as edges" },
            { value: "list", label: "List", title: "The same as rows, for the keyboard and screen readers" },
          ]}
          onChange={onMode}
          className="w-40"
          testId="pool-mode"
        />
        <Toggle on={panel === "tasks"} onClick={() => onPanel(panel === "tasks" ? null : "tasks")} icon={<IconTasks width={15} height={15} aria-hidden="true" />} testId="pool-panel-tasks">
          Tasks
        </Toggle>
        <Toggle on={panel === "messages"} onClick={() => onPanel(panel === "messages" ? null : "messages")} icon={<IconMessage width={15} height={15} aria-hidden="true" />} testId="pool-panel-messages">
          Messages
        </Toggle>
        <span className="mx-1 h-5 w-px bg-hairline" aria-hidden="true" />
        <Picker choice
          ariaLabel="Agents"
          heading="Agents in this pool may"
          size="sm"
          prefix={<span className="text-muted">Agents:</span>}
          value={pool.access.level}
          options={LEVELS.map((level) => ({ value: level.id, label: level.label, hint: level.words }))}
          onChange={(level) => void poolSetAccess(pool.id, level, level === "custom" ? pool.access.actions : []).then(setPools, (cause) => setError(errorText(cause)))}
          testId="pool-header-level"
        />
      </div>
      {error && (
        <p className="w-full text-label text-danger" role="alert">
          {error}
        </p>
      )}
    </header>
  );
}
