/**
 * D-62 decision 12 (pools-model §4.2, §4.4): what one session may do on
 * its own in a pool (its own "level"; the word is gone, AJ-E3011). A broad member (App, Account, Tool, Mac, everyone) carries one
 * level for every session it brings in; one session named on its own can
 * override it, and inside one pool the most specific member wins. The
 * rc33 walk (P5 step 3) found no control for it: only the pool-wide level
 * and a broad member's own were settable.
 */
import { useState, type JSX } from "react";

import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { POOL_RULES_CHANGED, poolJoin } from "../../lib/ipc/pool-model";
import type { PoolInfo, PoolRole } from "../../lib/ipc/pools";
import { setPools } from "../../lib/pools";
import { SAME_AS_POOL, ownLabel, wakeWords } from "../../lib/pool-words";
import { MemberOwnLevel } from "./setup/OwnLevelChoice";
import { useSessionPools } from "../session/useSessionPools";
import { Section } from "./Section";

const ROLES: readonly PoolRole[] = ["lead", "member", "reviewer", "observer"];
const roleOf = (word: string): PoolRole => ROLES.find((one) => one === word) ?? "member";

const VIA_WORD: Record<string, string> = { app: "App", account: "Account", tool: "tool", mac: "Mac", everyone: "project" };

/** What the "keep" choice says: whose choice it follows now, and which. */
export function keptLevelWords(via: string, level: string | null, poolLevel: string): string {
  if (via === "session") return `${SAME_AS_POOL} (${ownLabel(poolLevel)})`;
  return `Same as its ${VIA_WORD[via] ?? "member"} (${ownLabel(level ?? poolLevel)})`;
}

/** `look`: a section of its own in the member's inspector, or a row's
 * detail in the List view, whose term says what it is (the rc2 walk P5,
 * AJ-E3304 L4: reachable from both, the walker found it on neither). */
export function MemberLevel({ pool, sessionId, memberRole, name = "it", look = "section" }: { pool: PoolInfo; sessionId: string; memberRole: string; name?: string; look?: "section" | "row" }): JSX.Element | null {
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
      showToast({ message: "Couldn't change what it may do", detail: errorText(cause), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  const body = (
    <div className="flex flex-col gap-1.5" data-testid="inspect-node-level">
      {/* Round 3: the members' one control for this (MemberOwnLevel). */}
      <MemberOwnLevel
        prefix=""
        name={name}
        value={value === "keep" ? null : value}
        poolLevel={pool.access.level}
        kept={keptLevelWords(place.via, place.level, pool.access.level)}
        disabled={busy}
        onChange={(next) => void set(next ?? "keep")}
        testId="inspect-node-level-picker"
      />
      <span className="text-meta text-ink-2">Its own choice wins over its {own ? "pool's" : `${VIA_WORD[place.via] ?? "member"}'s`}, for this session only.</span>
      <span className="text-meta text-muted" data-testid="inspect-node-wake">
        Messages: it {wakeWords(place.level ?? pool.access.level)}.
      </span>
    </div>
  );
  return look === "row" ? body : (
    <Section title="What it may do on its own" testId="inspect-node-own">
      {body}
    </Section>
  );
}
