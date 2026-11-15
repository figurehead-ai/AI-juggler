import { useState, type JSX } from "react";

import { errorText } from "../lib/bridge";
import { poolSetAccess, type PoolInfo, type PoolsInfo } from "../lib/ipc/pools";
import { setPools } from "../lib/pools";
import { asOwnLevel } from "../lib/pool-words";
import { OwnLevelChoice } from "../pages/pool/setup/OwnLevelChoice";
import { PoolAccessFolds } from "../pages/pool/PoolAccessFolds";

/** A call's answer that is the pools list (the access calls'). */
const isPoolsInfo = (answer: unknown): answer is PoolsInfo => typeof answer === "object" && answer !== null && "pools" in answer;

/**
 * D-28: what agents may do in one pool — its level, picked by you here
 * only; a Custom level's actions; how far a delivered message may go
 * (D-29); and the rules you made, each one taken back with a click (it
 * stops applying on the very next request). Above the level, an agent's
 * request becomes a question in Needs you, never a block. AJ-E3011 round
 * 3: the same controls and words as the pool's own inspector
 * (OwnLevelChoice, PoolAccessFolds), so Settings and the pool can't drift.
 */
export function PoolAccessPanel({ pool, now = Date.now() }: { pool: PoolInfo; now?: number }): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = async (work: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    try {
      const answer = await work();
      if (isPoolsInfo(answer)) setPools(answer);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  const level = pool.access.level;
  return (
    <div className="flex flex-col gap-2.5" data-testid="pool-access" data-pool={pool.id}>
      {/* AJ-E3011 round 3: the one control for what a pool's sessions may do on their own. */}
      <OwnLevelChoice
        value={level === "custom" ? "custom" : asOwnLevel(level)}
        label="What sessions in this pool may do on their own"
        disabled={busy}
        onChange={(next) => void run(() => poolSetAccess(pool.id, next, []))}
        onCustom={() => void run(() => poolSetAccess(pool.id, "custom", pool.access.actions))}
        testId="pool-access-level"
      />

      {/* The pool inspector's own folds: Custom actions, Allowed without asking, Messages into sessions. */}
      <PoolAccessFolds pool={pool} now={now} busy={busy} run={(work) => void run(work)} />

      {error && (
        <p className="text-xs text-danger" role="alert" data-testid="pool-access-error">
          {error}
        </p>
      )}
    </div>
  );
}
