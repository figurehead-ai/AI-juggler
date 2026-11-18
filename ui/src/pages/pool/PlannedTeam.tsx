/**
 * 0.4.6 (D-126 §3): a pool being set up, before any session has joined,
 * draws its planned team on the diagram — the lead on top (the App whose
 * next session leads, with Start one now where it can start one, or "No
 * lead yet"), the other members in a row under it (an App with its
 * own logo and kind, the reviewer marked, everyone in its projects as a
 * stack), a line from the lead to each, and a line between two that may
 * message each other — with what happens next under them. It pans and
 * zooms with the canvas; each card is a button that opens it in the
 * inspector (the rc2 walk P5: every card on the diagram takes a press,
 * focus and Enter). Once a session joins, the diagram is its sessions,
 * the members in their column (RuleNodes).
 */
import { useCallback, useId, useState, type JSX, type ReactNode, type Ref } from "react";

import { LogoTile } from "../../components/ToolLogo";
import { IconLead } from "../../components/icons-canvas";
import { logoOf, MemberLogo, type LogoSources } from "../../components/MemberLogo";
import { useDesktopMembers } from "../../lib/connector";
import type { PoolRuleInfo } from "../../lib/ipc/pool-model";
import { usePendingAdds } from "../../lib/pool-pending";
import { APP_KIND_WORDS, JOINS_BY_ITSELF, LEAD_WOULD, NO_LEAD_YET, asRole, asTalkRule, leadNextLine } from "../../lib/pool-words";
import { roleLabel } from "../../lib/pools";
import { cliFolderOf } from "../../lib/start-lead";
import type { Selection } from "./Canvas";
import { NODE_W, type View } from "./layout";
import { ConnectorFix, KIND_WORD, STACK } from "./RuleNodes";
import { SELECTED } from "./selection";
import { StartLeadButton } from "./setup/StartLead";
import { PLANNED } from "./setup-words";

/** The room between two cards in a row, and from the lead to the row. */
const GAP = 116;
const DROP = 96;
/** The lead's top, under the canvas's Add a member. */
const TOP = 24;
/** A row's pitch, and how many cards to a row. */
const PITCH = 200;
const PER_ROW = 3;

type Card = { ref: string; label: string; sub: string; body: string; role: string; logo: ReactNode; broad: boolean; pending?: boolean; start?: boolean };

/** Each card's place under the lead: centred rows of up to three. */
export function plannedPlaces(count: number, top: number): { x: number; y: number }[] {
  return Array.from({ length: count }, (_, at) => {
    const row = Math.floor(at / PER_ROW);
    const inRow = Math.min(PER_ROW, count - row * PER_ROW);
    const width = inRow * NODE_W + (inRow - 1) * GAP;
    return { x: NODE_W / 2 - width / 2 + (at % PER_ROW) * (NODE_W + GAP), y: top + row * PITCH };
  });
}

export function PlannedTeam({
  poolId,
  rules,
  labelOf,
  logos,
  leadNext,
  noLead,
  where,
  view,
  selection,
  onSelect,
  onStartLead,
  talk = [],
}: {
  poolId: string;
  /** Its broad members (an App, Account, Tool, Mac, everyone). */
  rules: readonly PoolRuleInfo[];
  labelOf: (rule: PoolRuleInfo) => string;
  /** Where its members' logos are looked up (its Apps). */
  logos: LogoSources;
  /** The App whose next session leads, by ref. */
  leadNext: string | null;
  /** No session leads and none is waited for: "No lead for now" on top. */
  noLead: boolean;
  where: string;
  view: View;
  selection: Selection;
  onSelect: (selection: Selection) => void;
  onStartLead?: (member: string) => void;
  /** The pool's talk rows: two roles that may message each other have a
   * line between them. */
  talk?: readonly { from: string; to: string; rule: string }[];
}): JSX.Element {
  const mayTalk = (a: string, b: string) => [[a, b], [b, a]].some(([from, to]) => asTalkRule(talk.find((row) => row.from === `role:${from}` && row.to === `role:${to}`)?.rule) !== "blocked");
  const desktop = useDesktopMembers(poolId);
  const adding = usePendingAdds(poolId).filter((one) => one.kind !== "session" && !rules.some((rule) => rule.member === one.ref));
  const [leadH, setLeadH] = useState(150);
  // The row hangs under the lead's card as tall as it's drawn (its Start
  // one now, a connector's Add): read when it's drawn, and as it changes.
  const top = useCallback((node: HTMLDivElement | null) => {
    if (!node) return undefined;
    const read = () => setLeadH(node.offsetHeight || 150);
    read();
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(read);
    observer?.observe(node);
    return () => observer?.disconnect();
  }, []);
  const card = (rule: PoolRuleInfo): Card => {
    const logo = logoOf(rule.member, logos);
    const lead = rule.member === leadNext;
    const kind = rule.kind === "app" && logo.kind === "tool" ? APP_KIND_WORDS[logo.look] : rule.member === "all" ? PLANNED.everyoneSub : (KIND_WORD[rule.kind] ?? rule.kind);
    const role = lead ? "lead" : asRole(rule.role);
    return {
      ref: rule.member,
      label: labelOf(rule),
      sub: lead ? leadNextLine(where) : `${roleLabel(rule.role)} · ${kind}`,
      body: lead ? PLANNED.leadBody : rule.member === "all" ? `${JOINS_BY_ITSELF}.` : PLANNED.appBody(where),
      role,
      // Its role's mark only for a lead or a reviewer (MemberLogo's own rule).
      logo: <MemberLogo spec={logo} size={32} badge={role} />,
      broad: rule.kind !== "app" && rule.kind !== "tool",
      start: lead && !!onStartLead && cliFolderOf(rule.member) !== null,
    };
  };
  const leadRule = rules.find((rule) => rule.member === leadNext);
  const head: Card | null = leadRule ? card(leadRule) : null;
  const rest: Card[] = [
    ...rules.filter((rule) => rule !== leadRule).map(card),
    ...adding.map((one) => ({ ref: one.ref, label: one.label, sub: KIND_WORD[one.kind] ?? one.kind, body: PLANNED.adding, role: "member", logo: <MemberLogo spec={logoOf(one.ref, logos)} size={32} />, broad: true, pending: true })),
  ];
  const hasTop = head !== null || noLead;
  const rowTop = hasTop ? TOP + leadH + DROP : TOP;
  const places = plannedPlaces(rest.length, rowTop);
  const lines: string[] = [];
  if (hasTop) for (const at of places) lines.push(`M${NODE_W / 2} ${TOP + leadH} C${NODE_W / 2} ${TOP + leadH + 60} ${at.x + NODE_W / 2} ${at.y - 60} ${at.x + NODE_W / 2} ${at.y}`);
  places.forEach((at, i) => {
    const next = places[i + 1];
    if (next && next.y === at.y && mayTalk(rest[i]!.role, rest[i + 1]!.role)) lines.push(`M${at.x + NODE_W} ${at.y + 48} H${next.x}`);
  });
  const picked = (ref: string) => selection.kind === "rule" && selection.ref === ref;
  return (
    <>
      <div className="pointer-events-none absolute left-0 top-0 origin-top-left" style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})` }} data-testid="planned-team">
        <svg aria-hidden="true" className="absolute left-0 top-0 overflow-visible" width="1" height="1" data-testid="planned-team-lines">
          {lines.map((d) => (
            <path key={d} d={d} fill="none" strokeWidth={1.5} className="stroke-hairline" />
          ))}
        </svg>
        {head && (
          <PlannedCard card={head} at={{ x: 0, y: TOP }} lead box={top} selected={picked(head.ref)} onSelect={() => onSelect({ kind: "rule", ref: head.ref })}>
            {head.start && onStartLead && <StartLeadButton member={head.ref} name={head.label} where={where} onStart={onStartLead} testId="planned-start-lead" />}
            <ConnectorFix info={desktop?.get(head.ref)} label={head.label} onOpen={() => onSelect({ kind: "rule", ref: head.ref })} />
          </PlannedCard>
        )}
        {!head && noLead && (
          <PlannedCard
            card={{ ref: "lead", label: NO_LEAD_YET, sub: roleLabel("lead"), body: LEAD_WOULD, role: "lead", logo: <LogoTile icon={IconLead} size={32} />, broad: false }}
            at={{ x: 0, y: TOP }}
            box={top}
            selected={selection.kind === "pool" && selection.focus === "lead"}
            onSelect={() => onSelect({ kind: "pool", focus: "lead" })}
          />
        )}
        {rest.map((one, at) => (
          <PlannedCard key={one.ref} card={one} at={places[at]!} selected={picked(one.ref)} onSelect={() => onSelect({ kind: "rule", ref: one.ref })}>
            <ConnectorFix info={one.pending ? undefined : desktop?.get(one.ref)} label={one.label} onOpen={() => onSelect({ kind: "rule", ref: one.ref })} />
          </PlannedCard>
        ))}
      </div>
      <div className="pointer-events-none absolute inset-x-0 bottom-20 flex flex-col items-center gap-2 px-6 text-center" data-testid="planned-team-next">
        <p className="type-section">{PLANNED.title}</p>
        <p className="max-w-[26rem] text-meta text-ink-2">{PLANNED.line(where)}</p>
      </div>
    </>
  );
}

/** One planned member: a card whose button covers it (its name), its
 * kind and role under it, what it does; its own actions above. */
function PlannedCard({ card, at, lead = false, box, selected, onSelect, children }: { card: Card; at: { x: number; y: number }; lead?: boolean; box?: Ref<HTMLDivElement>; selected: boolean; onSelect: () => void; children?: ReactNode }): JSX.Element {
  const id = useId();
  const look = selected ? SELECTED : `bg-surface shadow-card ${lead ? "inset-ring-[1.5px] inset-ring-accent/70" : "inset-ring-1 inset-ring-hairline/75"} ${card.broad ? STACK : ""}`;
  return (
    <div
      ref={box}
      data-node
      data-testid="planned-card"
      data-rule-ref={card.ref}
      data-selected={selected || undefined}
      aria-busy={card.pending || undefined}
      className={"pointer-events-auto absolute flex flex-col gap-2.5 rounded-[14px] p-3.5 " + look + (card.pending ? " opacity-70" : "")}
      style={{ left: at.x, top: at.y, width: NODE_W }}
    >
      <span className="flex min-w-0 items-center gap-2.5">
        {card.logo}
        <span className="flex min-w-0 flex-col">
          <button
            type="button"
            onClick={onSelect}
            aria-pressed={selected}
            aria-describedby={`${id}-sub ${id}-body`}
            className="truncate rounded text-left text-body font-semibold text-ink outline-none after:absolute after:inset-0 after:rounded-[14px] focus-visible:after:outline-2 focus-visible:after:outline-offset-4 focus-visible:after:outline-ink-2"
            title={card.label}
          >
            {card.label}
          </button>
          <span id={`${id}-sub`} className="truncate text-label text-muted">
            {card.sub}
          </span>
        </span>
      </span>
      <span id={`${id}-body`} className="text-meta text-ink-2">
        {card.body}
      </span>
      {/* Its own actions, above the card's cover. */}
      {children && <span className="relative z-10 flex flex-col items-start gap-2 empty:hidden">{children}</span>}
    </div>
  );
}
