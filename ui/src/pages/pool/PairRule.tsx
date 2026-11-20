/**
 * SK-13 (the soak's LA2 step 3): who may message whom between two members
 * was set only by pressing their line on the canvas, and the List view
 * (the canvas's twin, for the keyboard and screen readers) listed only
 * pairs that had talked this week. Here any two members' line opens in
 * the inspector, where its rule is set (`EdgeTalk`, as the line's own);
 * the role pairs' rules are in the pool's settings, one click away.
 */
import { useState, type JSX } from "react";

import { Picker } from "../../components/controls";
import { Button } from "../../components/primitives";
import { isActiveStatus } from "../../components/indicator";
import type { Selection } from "./Canvas";
import { edgeKey, type MemberNode } from "./useCanvas";

export function PairRule({ members, names, onSelect }: { members: readonly MemberNode[]; names: ReadonlyMap<string, string>; onSelect: (selection: Selection) => void }): JSX.Element {
  const [from, setFrom] = useState<string | null>(null);
  const [to, setTo] = useState<string | null>(null);
  const options = members.map((member) => ({ value: member.id, label: names.get(member.id) ?? member.name }));
  const open = members.filter((member) => isActiveStatus(member.session.status)).map((member) => member.id);
  const groups = [{ label: "Open now", values: open }];
  const ready = from !== null && to !== null && from !== to;
  return (
    <div className="flex flex-col gap-2 rounded-card border border-hairline bg-surface px-4 py-3" data-testid="pool-list-pair">
      <p className="text-meta text-ink-2">Who may message whom between two members: pick them, then set their line's rule beside.</p>
      <div className="flex flex-wrap items-center gap-2">
        <Picker groups={groups} size="sm" ariaLabel="From" prefix="From:" value={from} options={options} onChange={setFrom} testId="pool-list-pair-from" />
        <span aria-hidden="true" className="text-muted">
          →
        </span>
        <Picker groups={groups} size="sm" ariaLabel="To" prefix="To:" value={to} options={options.map((option) => ({ ...option, disabled: option.value === from, reason: option.value === from ? "That's who it's from" : undefined }))} onChange={setTo} testId="pool-list-pair-to" />
        <Button size="sm" variant="secondary" disabled={!ready} onClick={() => ready && onSelect({ kind: "edge", key: edgeKey(from, to) })} data-testid="pool-list-pair-open">
          Set its rule
        </Button>
        <button type="button" onClick={() => onSelect({ kind: "pool", focus: "talk" })} className="ml-auto rounded text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60" data-testid="pool-list-pair-roles">
          Rules by role
        </button>
      </div>
      {!ready && <span className="text-label text-muted">{members.length < 2 ? "It takes two members on the diagram." : "Pick who it's from and who it's to."}</span>}
    </div>
  );
}
