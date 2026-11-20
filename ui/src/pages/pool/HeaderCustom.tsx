/**
 * The diagram header's Custom (Sam's round-4 walk: choosing it changed
 * only the label, and the sentence read "they may Custom"): choosing it
 * opens the pool's own Custom list in place, under the header's control —
 * the inspector's and Settings' same list (CustomActions) — with what
 * they may do in a sentence ("On their own they may do the 11 things you
 * picked"). Escape, Done or a press outside closes it; the page under it
 * is inert while it's open.
 */
import { useEffect, useRef, useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { errorText } from "../../lib/bridge";
import { useInertOutside } from "../../lib/inert";
import type { PoolInfo, PoolsInfo } from "../../lib/ipc/pools";
import { setPools, usePools } from "../../lib/pools";
import { levelSentence, pickedCount } from "../../lib/pool-words";
import { CustomActions } from "./PoolAccessFolds";

const isPoolsInfo = (answer: unknown): answer is PoolsInfo => typeof answer === "object" && answer !== null && "pools" in answer;

export function HeaderCustom({ pool: given, onClose }: { pool: PoolInfo; onClose: () => void }): JSX.Element {
  // The pool as the last change left it (each tick reads back at once).
  const pool = usePools()?.pools.find((one) => one.id === given.id) ?? given;
  const ref = useRef<HTMLDialogElement | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useInertOutside(ref);
  useEffect(() => {
    ref.current?.querySelector<HTMLInputElement>("input")?.focus();
    const outside = (event: MouseEvent) => {
      if (!(event.target instanceof Node && ref.current?.contains(event.target))) onClose();
    };
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", outside);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("mousedown", outside);
      window.removeEventListener("keydown", key);
    };
  }, [onClose]);
  const run = (work: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    work().then(
      (answer) => {
        if (isPoolsInfo(answer)) setPools(answer);
        setBusy(false);
      },
      (cause) => {
        setError(errorText(cause));
        setBusy(false);
      },
    );
  };
  return (
    <dialog
      ref={ref}
      open
      aria-labelledby="pool-header-custom-title"
      className="absolute left-auto right-0 top-full z-40 m-0 mt-2 flex max-h-[min(32rem,70vh)] w-80 border-0 text-ink flex-col gap-2 overflow-y-auto rounded-xl bg-surface p-3 shadow-card"
      data-testid="pool-header-custom"
    >
      <h3 id="pool-header-custom-title" className="text-body font-semibold text-ink">
        Custom: what they may do on their own
      </h3>
      <p className="text-meta text-ink-2" data-testid="pool-header-custom-sentence">
        {levelSentence("custom", pickedCount(pool))}
      </p>
      <CustomActions pool={pool} busy={busy} run={run} testId="pool-header-custom-list" />
      {error && (
        <p className="text-meta text-danger" role="alert">
          {error}
        </p>
      )}
      <Button size="sm" variant="secondary" className="self-end" onClick={onClose} data-testid="pool-header-custom-done">
        Done
      </Button>
    </dialog>
  );
}
