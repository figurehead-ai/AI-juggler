/**
 * D-62 (AJ-E3008): a member rule in the inspector: what it brings in
 * (its sessions now, and new ones), its role and its own level (the
 * pool's by default), and taking it out. A Watched member says what it
 * can't do and offers Continue in AI Juggler.
 */
import { useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { Picker, Segmented } from "../../components/controls";
import { useActionHost } from "../../components/ItemMenu";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { useConversations } from "../../lib/conversations";
import { poolJoin, poolLeave, type PoolRuleInfo, type WatchedMemberInfo } from "../../lib/ipc/pool-model";
import type { PoolInfo, PoolRole } from "../../lib/ipc/pools";
import { leadHint, levelLabel, roleLabel, rolesFor, setPools } from "../../lib/pools";

export function InspectRule({
  pool,
  rule,
  watched,
  label,
  onChanged,
  onGone,
}: {
  pool: PoolInfo;
  rule: PoolRuleInfo | undefined;
  watched: WatchedMemberInfo | undefined;
  label: string;
  onChanged: () => void;
  onGone: () => void;
}): JSX.Element {
  const host = useActionHost();
  const { all } = useConversations();
  const [busy, setBusy] = useState(false);
  const member = rule?.member ?? watched?.member ?? "";
  const role = (rule?.role ?? watched?.role ?? "member") as PoolRole;
  const level = rule?.level ?? null;
  const run = async (what: string, call: () => Promise<unknown>) => {
    setBusy(true);
    try {
      const info = await call();
      if (info && typeof info === "object" && "pools" in (info as object)) setPools(info as Parameters<typeof setPools>[0]);
      onChanged();
    } catch (cause) {
      showToast({ message: `Couldn't ${what}`, detail: errorText(cause), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  const conversation = watched ? all.find((row) => row.account_id === watched.folder && row.id === watched.conversation_id) : undefined;
  const fixed = pool.kind === "default" && member === "all";
  return (
    <div className="flex flex-col gap-4 px-4" data-testid="inspect-rule">
      <div className="flex flex-col gap-0.5">
        <span className="text-section text-ink">{label}</span>
        <span className="text-meta text-ink-2">
          {watched
            ? "Runs in another app: shown here and counted, with no tools and no rules, until it's continued in AI Juggler."
            : `${rule?.sessions ?? 0} ${rule?.sessions === 1 ? "session" : "sessions"} here now; its new sessions in this pool's projects join by themselves.`}
        </span>
      </div>
      {conversation && (
        <Button variant="primary" size="sm" className="self-start" onClick={() => void host.continueConversation?.(conversation)} data-testid="inspect-rule-continue">
          Continue in AI Juggler
        </Button>
      )}
      <div className="flex flex-col gap-1.5">
        <span className="text-meta font-semibold text-ink-2">Role</span>
        <Segmented
          label="Role"
          value={role}
          segments={rolesFor(member).map((one) => ({ value: one, label: roleLabel(one) }))}
          onChange={(next) => void run("change its role", () => poolJoin(pool.id, member, next, level))}
          className="self-start"
          testId="inspect-rule-role"
        />
        {leadHint(member) && <span className="text-meta text-ink-2" data-testid="inspect-rule-lead-hint">{leadHint(member)}</span>}
      </div>
      {!watched && (
        <div className="flex flex-col gap-1.5">
          <span className="text-meta font-semibold text-ink-2">What its sessions may do</span>
          <Picker
            choice
            size="sm"
            ariaLabel="Its level"
            value={level ?? "pool"}
            options={[
              { value: "pool", label: `The pool's (${levelLabel(pool.access.level)})` },
              { value: "read", label: "Read" },
              { value: "work_together", label: "Work together" },
              { value: "full", label: "Full" },
            ]}
            onChange={(next) => void run("change its level", () => poolJoin(pool.id, member, role, next === "pool" ? null : next))}
            testId="inspect-rule-level"
          />
          <span className="text-meta text-ink-2">Its own level replaces the pool's for the sessions it brings in.</span>
        </div>
      )}
      {!fixed && (
        <Button
          variant="secondary"
          size="sm"
          className="self-start"
          disabled={busy}
          onClick={() =>
            void run("take it out", async () => {
              const info = await poolLeave(pool.id, member);
              onGone();
              return info;
            })
          }
          data-testid="inspect-rule-leave"
        >
          Take out of the pool
        </Button>
      )}
    </div>
  );
}
