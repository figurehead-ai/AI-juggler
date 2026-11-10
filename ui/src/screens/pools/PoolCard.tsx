/**
 * AJ-E2807 — a pool's card on the Pools screen, with its menu (⋯,
 * right-click, ⇧F10: Open its canvas, Rename…, Delete…), and the two
 * small sheets its menu opens. Deleting says what happens to its members
 * first, as the pool's own page does.
 */
import { useState, type JSX, type ReactNode } from "react";
import { useItemMenu } from "../../components/ItemMenu";
import { Field, Input, Modal } from "../../components/form";
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
      {menu.button({ label: `Pool actions for ${pool.name}`, className: "absolute right-2 top-2", testId: "pool-card-menu" })}
      {menu.element}
    </div>
  );
}

export function RenamePool({ pool, onClose }: { pool: PoolInfo; onClose: () => void }): JSX.Element {
  const [name, setName] = useState(pool.name);
  const [busy, setBusy] = useState(false);
  const save = async () => {
    const words = name.trim();
    if (!words || words === pool.name) return onClose();
    setBusy(true);
    try {
      setPools(await poolRename(pool.id, words));
      onClose();
    } catch (cause) {
      showToast({ message: "Couldn't rename the pool", detail: errorText(cause), tone: "error" });
      setBusy(false);
    }
  };
  return (
    <Modal title="Rename pool" onClose={onClose} testId="pool-rename">
      <form
        className="flex flex-col gap-3"
        onSubmit={(event) => {
          event.preventDefault();
          void save();
        }}
      >
        <Field label="Name">
          {/* eslint-disable-next-line jsx-a11y/no-autofocus -- the sheet is for this one field */}
          <Input autoFocus value={name} onChange={(event) => setName(event.target.value)} data-testid="pool-rename-input" />
        </Field>
        <span className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" type="submit" disabled={busy || !name.trim()} data-testid="pool-rename-save">
            Rename
          </Button>
        </span>
      </form>
    </Modal>
  );
}

export function DeletePool({ pool, members, onClose }: { pool: PoolInfo; members: number; onClose: () => void }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const remove = async () => {
    setBusy(true);
    try {
      setPools(await poolDelete(pool.id));
      onClose();
    } catch (cause) {
      showToast({ message: "Couldn't delete the pool", detail: errorText(cause), tone: "error" });
      setBusy(false);
    }
  };
  const who = members === 0 ? "It has no members." : members === 1 ? "Its one session goes back to its project's own pool." : `Its ${members} sessions go back to their projects' own pools.`;
  return (
    <Modal title={`Delete «${pool.name}»?`} onClose={onClose} testId="pool-delete-sheet">
      <p className="text-body text-ink-2" data-testid="pool-delete-words">
        {who} Its messages and tasks stay in its history. No session stops.
      </p>
      <span className="mt-3 flex justify-end gap-2">
        <Button variant="ghost" onClick={onClose}>
          Keep it
        </Button>
        <Button variant="danger" onClick={() => void remove()} disabled={busy} data-testid="pool-delete-confirm">
          Delete pool
        </Button>
      </span>
    </Modal>
  );
}
