/**
 * D-62 decision 12 (pools-model §4.2, §4.4): one session's own level in a
 * pool. A broad member (App, Account, Tool, Mac, everyone) carries one
 * level for every session it brings in; one session named on its own can
 * override it, and inside one pool the most specific member wins. The
 * rc33 walk (P5 step 3) found no control for it: only the pool-wide level
 * and a broad member's own were settable.
 */
import { useState, type JSX } from "react";

import { Picker } from "../../components/controls";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { POOL_RULES_CHANGED, poolJoin } from "../../lib/ipc/pool-model";
import type { PoolInfo, PoolRole } from "../../lib/ipc/pools";
import { levelLabel, setPools } from "../../lib/pools";
import { useSessionPools } from "../session/useSessionPools";

const ROLES: readonly PoolRole[] = ["lead", "member", "reviewer", "observer"];
const roleOf = (word: string): PoolRole => ROLES.find((one) => one === word) ?? "member";

const VIA_WORD: Record<string, string> = { app: "App", account: "Account", tool: "tool", mac: "Mac", everyone: "project" };

/** What the "keep" choice says: whose level it is now, and which. */
export function keptLevelWords(via: string, level: string | null, poolLevel: string): string {
  if (via === "session") return `The pool's (${levelLabel(poolLevel)})`;
  return `As its ${VIA_WORD[via] ?? "member"} (${levelLabel(level ?? poolLevel)})`;
}

export function MemberLevel({ pool, sessionId, memberRole }: { pool: PoolInfo; sessionId: string; memberRole: string }): JSX.Element | null {
  const info = useSessionPools(sessionId);
  const [busy, setBusy] = useState(false);
  const place = info?.pools.find((one) => one.pool_id === pool.id);
  if (!place) return null;
  const own = place.via === "session";
  const value = own ? (place.level ?? "keep") : "keep";
  const set = async (next: string) => {
    if (next === value) return;
    setBusy(true);
    try {
      // Named on its own: a session member, so it wins over the broad one.
      const member = own ? place.member : `session:${sessionId}`;
      setPools(await poolJoin(pool.id, member, roleOf(own ? place.role : memberRole), next === "keep" ? null : next));
      window.dispatchEvent(new CustomEvent(POOL_RULES_CHANGED, { detail: pool.id }));
    } catch (cause) {
      showToast({ message: "Couldn't change its level", detail: errorText(cause), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="flex flex-col gap-1.5" data-testid="inspect-node-level">
      <span className="text-meta font-semibold text-ink-2">Its level here</span>
      <Picker
        choice
        size="sm"
        ariaLabel="Its level here"
        value={value}
        disabled={busy}
        options={[
          { value: "keep", label: keptLevelWords(place.via, place.level, pool.access.level) },
          { value: "read", label: "Read" },
          { value: "work_together", label: "Work together" },
          { value: "full", label: "Full" },
        ]}
        onChange={(next) => void set(next)}
        testId="inspect-node-level-picker"
      />
      <span className="text-meta text-ink-2">Its own level wins over its {own ? "pool's" : `${VIA_WORD[place.via] ?? "member"}'s`} for this session only.</span>
    </div>
  );
}
