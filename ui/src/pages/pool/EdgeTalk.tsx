/**
 * D-62 decision 13 (AJ-E3008 req. 4): a line between two members says
 * its talk rule, and it can be changed for that pair only (wireframe 6):
 * "The role pair Member → Member says Ask first. This line follows it."
 * Setting it back to the pair's clears the line's own rule. Claude's own
 * channel isn't held by it (D-95), and the line says so.
 */
import { useEffect, useState, type JSX } from "react";

import { Segmented } from "../../components/controls";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import { poolEdgeTalk, poolSetTalk, type EdgeTalkInfo, type TalkRule } from "../../lib/ipc/pool-model";
import { CLAUDE_CHANNEL_NOTE, roleLabel } from "../../lib/pools";

const WORDS: Record<TalkRule, string> = { allowed: "Allowed", ask: "Ask first", blocked: "Blocked" };

export function EdgeTalk({ poolId, from, to, fromName, toName }: { poolId: string; from: string; to: string; fromName: string; toName: string }): JSX.Element | null {
  const [talk, setTalk] = useState<EdgeTalkInfo | null>(null);
  const [nonce, setNonce] = useState(0);
  useEffect(() => {
    let alive = true;
    void poolEdgeTalk(poolId, from, to).then(
      (info) => alive && setTalk(info),
      () => alive && setTalk(null),
    );
    return () => {
      alive = false;
    };
  }, [poolId, from, to, nonce]);
  if (!talk) return null;
  const set = async (rule: TalkRule) => {
    try {
      // The pair's own rule again: the line follows its pair.
      await poolSetTalk(poolId, `session:${from}`, `session:${to}`, rule === talk.pair_rule ? null : rule);
      setNonce((value) => value + 1);
    } catch (cause) {
      showToast({ message: "Couldn't change who may talk", detail: errorText(cause), tone: "error" });
    }
  };
  const pair = `${roleLabel(talk.from_role)} → ${roleLabel(talk.to_role)}`;
  return (
    <div className="flex flex-col gap-1.5 px-4" data-testid="edge-talk">
      <span className="text-meta font-semibold text-ink-2">
        Talk · {fromName} → {toName}
      </span>
      <Segmented
        label={`${fromName} may message ${toName}`}
        value={talk.rule as TalkRule}
        segments={(["allowed", "ask", "blocked"] as const).map((rule) => ({ value: rule, label: WORDS[rule] }))}
        onChange={(rule) => void set(rule)}
        className="self-start"
        testId="edge-talk-rule"
      />
      <span className="text-meta text-ink-2">
        {talk.own
          ? `This line has its own rule; the role pair ${pair} says ${WORDS[talk.pair_rule as TalkRule]}. Choose that to follow the pair again.`
          : `The role pair ${pair} says ${WORDS[talk.pair_rule as TalkRule]}. This line follows it; change it here to set this pair only.`}
      </span>
      <span className="text-meta text-ink-2" data-testid="edge-talk-claude">
        {CLAUDE_CHANNEL_NOTE}
      </span>
    </div>
  );
}
