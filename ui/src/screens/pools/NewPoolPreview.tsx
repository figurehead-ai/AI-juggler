/**
 * 0.4.6 (D-126): New pool's live preview — the pool as its diagram will
 * show it, then its settings in a line each, matching the form on every
 * step. The lead sits on top (its crown in the accent), the members
 * below, each with its tool's logo; the step being set lights its part
 * of the drawing and its row. Before Who's in, the drawing is a ghost of
 * the shape to come. Under 960 px the layout folds it into
 * {@link PreviewStrip}, a strip above the form.
 *
 * The drawing is decorative (`aria-hidden`): the rows say the same in
 * words, as a list a screen reader reads.
 */
import type { JSX, ReactNode } from "react";

import { IconInfo, IconPlus, IconPool } from "../../components/icons-actions";
import { IconLead } from "../../components/icons-canvas";
import type { ControlIcon } from "../../components/controls";
import { LogoTile } from "../../components/ToolLogo";
import type { PoolRole } from "../../lib/ipc/pools";
import { NO_LEAD_YET } from "../../lib/pool-words";
import { MemberLogo, type LogoSpec } from "../../components/MemberLogo";
import { ROW_WORDS, type PoolRow } from "./rows";

/** One card on the drawing. */
export interface PreviewNode {
  ref: string;
  /** Its name ("Codex", "Everyone in web"). */
  name: string;
  /** Its role line ("Reviewer", "Lead · its next session"). */
  line: string;
  logo: LogoSpec;
  role: PoolRole;
  /** A broad member (everyone in the projects): drawn as a stack. */
  stacked?: boolean;
}

/** A preview row: one of the setup's six steps, by its id (Review's rows too). */
export type PreviewRowId = Extract<PoolRow, "where" | "who" | "lead" | "everyone" | "talk" | "own">;

export interface PreviewRow {
  id: PreviewRowId;
  label: string;
  value: string;
  /** Not chosen yet: the value it will have, in a quieter ink. */
  pending: boolean;
  /** Filled in for the user (a step not visited): "· prefilled". */
  prefilled: boolean;
}

/** What the drawing lights: the members (Who's in), the lead, or the reviewers. */
export type PreviewFocus = "members" | "lead" | "reviewers" | null;

/** At most this many cards in the bottom row; past it, "+N more". */
const ROW_MAX = 3;
/** The bottom row's cards across, as % of the drawing's width. */
const ACROSS: Record<number, number[]> = { 1: [50], 2: [27, 73], 3: [17, 50, 83] };

const NODE = "absolute flex w-max -translate-x-1/2 items-center gap-1.5 rounded-[10px] px-1.5 py-1.5 text-label leading-tight";
const NODE_AT_REST = "bg-surface shadow-[0_1px_2px_rgba(0,0,0,0.25)] inset-ring-1 inset-ring-hairline/75";
const NODE_LIT = "bg-surface ring-[4px] ring-accent-soft inset-ring-[1.5px] inset-ring-accent";
const STACK = "shadow-[3px_3px_0_-1px_var(--color-surface-3),3px_3px_0_0_var(--color-hairline)]";
const GHOST = "inset-ring-1 inset-ring-hairline text-faint";

/** A card's widest, by how many share its row (as wide as its words up
 * to it: `w-max`, or one at 73 % would shrink to the room right of it). */
const WIDEST: Record<number, string> = { 1: "60%", 2: "45%", 3: "32%" };

/** A member's card on a drawing: its logo (its role's mark at the
 * corner), its name and its line. The preview's, and a Pools card's
 * thumbnail (`oneLine`: its name on one line, in a smaller box). */
export function PreviewCard({
  node,
  lit = false,
  at,
  across = 3,
  oneLine = false,
  testId = "new-pool-preview-node",
}: {
  node: PreviewNode;
  lit?: boolean;
  at: { left: string; top: number | string };
  across?: number;
  oneLine?: boolean;
  testId?: string;
}): JSX.Element {
  const wrap = oneLine ? "truncate" : "line-clamp-2 break-words";
  return (
    <span className={`${NODE} ${lit ? NODE_LIT : NODE_AT_REST} ${node.stacked ? STACK : ""}`} style={{ left: at.left, top: at.top, maxWidth: WIDEST[across] ?? "31%" }} data-testid={testId} data-ref={node.ref} data-role={node.role} data-lit={lit || undefined}>
      <MemberLogo spec={node.logo} size={24} badge={node.role} />
      <span className="flex min-w-0 flex-col">
        {/* D-126 critique: a name wraps to two lines before it's cut. */}
        <span className={`${wrap} font-semibold text-ink`} data-testid={`${testId}-name`}>
          {node.name}
        </span>
        <span className={`${wrap} text-muted`}>{node.line}</span>
      </span>
    </span>
  );
}

function Ghost({ icon: Shape, words, at }: { icon: ControlIcon; words: string; at: { left: string; top: number } }): JSX.Element {
  return (
    <span className={`${NODE} ${GHOST} whitespace-nowrap`} style={{ left: at.left, top: at.top }} data-testid="new-pool-preview-ghost">
      <Shape aria-hidden="true" width={14} height={14} />
      {words}
    </span>
  );
}

/** The drawing: the lead (or its place) on top, the members below. */
export function PreviewDiagram({ lead, members, reached, focus }: { lead: PreviewNode | null; members: readonly PreviewNode[]; reached: boolean; focus: PreviewFocus }): JSX.Element {
  const shown = members.length > ROW_MAX ? members.slice(0, ROW_MAX - 1) : members;
  const more = members.length - shown.length;
  const count = reached ? Math.max(1, shown.length + (more > 0 ? 1 : 0)) : 2;
  const across = ACROSS[Math.min(ROW_MAX, count)]!;
  const lit = (node: PreviewNode) => (focus === "members" ? node.role !== "lead" : focus === "reviewers" ? node.role === "reviewer" : focus === "lead" ? node.role === "lead" : false);
  return (
    <div
      aria-hidden="true"
      className="relative mx-3 h-[212px] overflow-hidden rounded-[10px] bg-canvas-hi bg-[radial-gradient(var(--color-hairline)_1px,transparent_1.2px)] bg-[length:14px_14px]"
      data-testid="new-pool-preview-diagram"
    >
      <svg className="absolute inset-0 size-full" viewBox="0 0 100 100" preserveAspectRatio="none" fill="none">
        {across.map((x) => (
          <path key={x} d={`M50 27 C50 46 ${x} 42 ${x} 60`} stroke="var(--color-faint)" strokeOpacity={reached ? 0.7 : 0.45} strokeWidth={1.25} vectorEffect="non-scaling-stroke" />
        ))}
      </svg>
      {!reached ? (
        <>
          <Ghost icon={IconLead} words="Lead" at={{ left: "50%", top: 22 }} />
          {across.map((x) => (
            <Ghost key={x} icon={IconPlus} words="Members" at={{ left: `${x}%`, top: 132 }} />
          ))}
        </>
      ) : (
        <>
          {lead ? <PreviewCard node={lead} lit={lit(lead)} across={1} at={{ left: "50%", top: 20 }} /> : <Ghost icon={IconLead} words={NO_LEAD_YET} at={{ left: "50%", top: 24 }} />}
          {shown.map((node, at) => (
            <PreviewCard key={node.ref} node={node} lit={lit(node)} across={across.length} at={{ left: `${across[at]}%`, top: 128 }} />
          ))}
          {more > 0 && <Ghost icon={IconPlus} words={`${more} more`} at={{ left: `${across[shown.length]}%`, top: 132 }} />}
        </>
      )}
    </div>
  );
}

/** Its settings, a line each; the step being set is lit. */
/** A row's step: going there from the preview, when it's reached. */
export interface PreviewPick {
  go: (row: PreviewRowId) => void;
  can: (row: PreviewRowId) => boolean;
}

function PreviewRows({ rows, current, pick }: { rows: readonly PreviewRow[]; current: PreviewRowId | null; pick?: PreviewPick }): JSX.Element {
  return (
    <dl className="flex flex-col gap-0.5 px-2 pb-2.5 pt-3" data-testid="new-pool-preview-rows">
      {rows.map((row) => {
        const Shape = ROW_WORDS[row.id].icon;
        const on = row.id === current;
        return (
          <div key={row.id} className={"flex items-start gap-2.5 rounded-lg px-2.5 py-[7px] text-meta " + (on ? "bg-accent-soft shadow-[inset_3px_0_0_var(--color-accent)]" : "")} data-testid="new-pool-preview-row" data-row={row.id} data-current={on || undefined}>
            <Shape aria-hidden="true" width={16} height={16} className={"mt-px shrink-0 " + (on ? "text-accent-hi" : "text-faint")} />
            <dt className="w-[92px] shrink-0 text-muted">
              {/* A door to its step (rc3: and the rows, which scroll on a
                  short window, are reached by the keyboard through it). */}
              {pick ? (
                <button type="button" disabled={!pick.can(row.id)} onClick={() => pick.go(row.id)} className="rounded-sm text-left outline-none hover:text-ink hover:underline focus-visible:ring-2 focus-visible:ring-accent/60 disabled:no-underline disabled:hover:text-muted" data-testid="new-pool-preview-go">
                  {row.label}
                  <span className="sr-only">, go to its step</span>
                </button>
              ) : (
                row.label
              )}
            </dt>
            <dd className={"min-w-0 " + (row.pending ? "text-ink-2" : "text-ink")} data-testid="new-pool-preview-value">
              {row.value}
              {row.prefilled && <span className="text-faint"> · prefilled</span>}
            </dd>
          </div>
        );
      })}
    </dl>
  );
}

/** The preview: its head, the drawing, then the rows (or Review's own
 * `extra`), and a line saying all of it can change later. */
export function NewPoolPreview({
  name,
  subtitle = "Preview · how its diagram will look",
  diagram,
  rows,
  current,
  extra,
  foot = true,
  pick,
}: {
  name: string;
  subtitle?: string;
  diagram: ReactNode;
  rows: readonly PreviewRow[];
  current: PreviewRowId | null;
  /** In place of the rows (Review: what to tell the lead first). */
  extra?: ReactNode;
  foot?: boolean;
  /** Each row a door to its step. */
  pick?: PreviewPick;
}): JSX.Element {
  return (
    // At most the window's height less its head and foot (D-126 critique:
    // at 1280×800 a taller one scrolled its head away), its rows
    // scrolling inside, so it stays whole in view beside the form.
    <aside aria-label="Preview" className="flex max-h-[calc(100dvh-200px)] min-h-0 flex-col overflow-hidden rounded-card bg-surface shadow-card inset-ring-1 inset-ring-hairline/75" data-testid="new-pool-preview">
      <div className="flex shrink-0 items-center gap-2 px-4 pb-3 pt-3.5">
        <LogoTile icon={IconPool} size={24} on />
        <span className="flex min-w-0 flex-col">
          <span className="truncate type-section" data-testid="new-pool-preview-name">
            {name}
          </span>
          <span className="text-label text-faint">{subtitle}</span>
        </span>
      </div>
      <div className="shrink-0">{diagram}</div>
      {/* It scrolls on a short window; its rows' doors (or Review's Copy)
          are how the keyboard reaches it (axe scrollable-region-focusable). */}
      <div className="min-h-0 overflow-y-auto" data-testid="new-pool-preview-scroll">
        {extra ?? <PreviewRows rows={rows} current={current} pick={pick} />}
      </div>
      {foot && (
        <p className="flex shrink-0 items-center gap-2 border-t border-divider px-[18px] pb-3.5 pt-2.5 text-label text-faint">
          <IconInfo aria-hidden="true" width={14} height={14} className="shrink-0" />
          Everything here can be changed later on the pool's diagram.
        </p>
      )}
    </aside>
  );
}

/** Under 960 px: the preview as a strip above the form. */
export function PreviewStrip({ name, rows }: { name: string; rows: readonly PreviewRow[] }): JSX.Element {
  const who = rows.find((row) => row.id === "who");
  const lead = rows.find((row) => row.id === "lead");
  return (
    <div className="flex min-w-0 items-center gap-3 rounded-xl bg-surface px-3.5 py-2.5 inset-ring-1 inset-ring-hairline/75" data-testid="new-pool-summary">
      <LogoTile icon={IconPool} size={24} on />
      <span className="min-w-0 flex-1 truncate text-meta text-ink-2">
        <span className="font-semibold text-ink">{name}</span>
        {who && !who.pending && <> · {who.value}</>}
        {lead && <> · {lead.value}</>}
      </span>
    </div>
  );
}
