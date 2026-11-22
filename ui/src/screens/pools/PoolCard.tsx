/**
 * AJ-E2807 — a pool's card on the Pools screen, with its menu (⋯,
 * right-click, ⇧F10: Open its canvas, Rename…, Delete…), and the two
 * small sheets its menu opens. Deleting says what happens to its members
 * first, as the pool's own page does. The pool's page has the same menu
 * in its header (`PoolHeaderMenu`, SK-1).
 */
import { useState, type JSX, type ReactNode } from "react";
import { useItemMenu } from "../../components/ItemMenu";
import { HeaderMenu } from "../../components/InspectorHeaderMenu";
import { RenameSheet } from "../../components/ManageWork";
import { Modal } from "../../components/form";
import { Button } from "../../components/primitives";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { poolDelete, poolRename, type PoolInfo } from "../../lib/ipc/pools";
import type { ActionHost } from "../../lib/action-host";
import { setPools, warmPool } from "../../lib/pools";

export function PoolCard({ pool, local, asks, onOpen, children }: { pool: PoolInfo; local: ActionHost; asks: number; onOpen: () => void; children: ReactNode }): JSX.Element {
  const menu = useItemMenu("pool", pool, pool.name, local);
  return (
    <div className="group relative" {...menu.triggerProps}>
      <button
        type="button"
        onClick={onOpen}
        onPointerEnter={() => void warmPool(pool.id)}
        onFocus={() => void warmPool(pool.id)}
        className={"flex w-full flex-col gap-2.5 rounded-card border bg-surface p-3 text-left outline-none transition-colors hover:border-accent/60 focus-visible:ring-2 focus-visible:ring-accent/60 " + (asks > 0 ? "border-warning/60" : "border-hairline")}
        data-testid="pool-card-tile"
        data-pool={pool.id}
      >
        {children}
      </button>
      {/* SK-1: always in sight, so Rename and Delete have a visible door. */}
      {menu.button({ label: `Pool actions for ${pool.name}`, className: "absolute right-2 top-2", shown: true, testId: "pool-card-menu" })}
      {menu.element}
    </div>
  );
}

/** A pool's Rename: the one rename sheet (sessions, projects, Apps,
 * Accounts), its name selected on open so typing replaces it (the 0.4.6
 * RC walk A8). A pool keeps a name: empty, or the same, changes nothing. */
export function RenamePool({ pool, onClose }: { pool: PoolInfo; onClose: () => void }): JSX.Element {
  return (
    <RenameSheet
      title={`Rename ${pool.name}`}
      initial={pool.name}
      placeholder={pool.name}
      hint="Shown wherever this pool is named."
      testId="pool-rename"
      onSave={async (next) => {
        const words = next.trim();
        if (words && words !== pool.name) setPools(await poolRename(pool.id, words));
        onClose();
      }}
      onClose={onClose}
    />
  );
}

export function DeletePool({ pool, members, onClose, onDeleted }: { pool: PoolInfo; members: number; onClose: () => void; onDeleted?: () => void }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const remove = async () => {
    setBusy(true);
    try {
      setPools(await poolDelete(pool.id));
      onDeleted?.();
      onClose();
    } catch (cause) {
      showToast({ message: "Couldn't delete the pool", detail: errorText(cause), tone: "error" });
      setBusy(false);
    }
  };
  const who = members === 0 ? "It has no members." : members === 1 ? "Its one session goes back to its project's own pool." : `Its ${members} sessions go back to their projects' own pools.`;
  return (
    <Modal title={`Delete “${pool.name}”?`} onClose={onClose} testId="pool-delete-sheet"
      footer={
        <>
            <Button variant="ghost" onClick={onClose}>
              Keep it
            </Button>
            <Button variant="danger" onClick={() => void remove()} disabled={busy} data-testid="pool-delete-confirm">
              Delete pool
            </Button>
        </>
      }
    >
      <p className="text-body text-ink-2" data-testid="pool-delete-words">
        {who} Its messages and tasks stay in its history. No session stops.
      </p>
    </Modal>
  );
}

/** SK-1: a pool's ⋯ on its own page, as on its card: Rename…, Delete….
 * A project's own pool has neither (it goes with its project), so no ⋯. */
export function PoolHeaderMenu({ pool, onDeleted }: { pool: PoolInfo; onDeleted: () => void }): JSX.Element | null {
  const [renaming, setRenaming] = useState(false);
  const [deleting, setDeleting] = useState(false);
  if (pool.kind !== "user") return null;
  return (
    <>
      <HeaderMenu kind="pool" item={pool} heading={pool.name} local={{ openPool: undefined, renamePool: () => setRenaming(true), deletePool: () => setDeleting(true) }} />
      {renaming && <RenamePool pool={pool} onClose={() => setRenaming(false)} />}
      {deleting && <DeletePool pool={pool} members={pool.members} onClose={() => setDeleting(false)} onDeleted={onDeleted} />}
    </>
  );
}
