/**
 * 0.4.6 (AJ-E3011 §1, D-126 §3): the pool's setup as one checklist in the
 * inspector — never over the canvas, so no member card hides it, and it
 * never vanishes when members are added. Its progress is a ring beside
 * its title ("5/7", the header says the same); each step keeps the
 * setup's order with its value under its name ("Who leads · Claude's next
 * session") and a Change that opens the same control as the setup (the
 * scope sheet, Add a member, or the pool's own section in the inspector);
 * a step left at its default counts as done ("default"). The open item —
 * the first not done — is expanded in place with what it does and its
 * action, the view's one filled primary. D-126: the team tools question
 * is its last item ("Let them message each other"), not a banner over the
 * page. With every step done it says the pool is ready and what happens
 * next (round 4 #3); while something keeps it from working it says so,
 * never "done" (round 6). Where something else is selected it shrinks to
 * one line that opens it again.
 */
import { useLayoutEffect, useRef, type JSX, type ReactNode } from "react";

import { Button } from "../../../components/primitives";
import { IconCheck } from "../../../components/icons-controls";
import type { SetupRow } from "../../../lib/pool-setup";
import type { TeamTools } from "../../../lib/team-tools";
import { openAgentAccess } from "../../../lib/team-tools";
import { STEP_WORDS, type SETUP_STEPS } from "../../../lib/pool-words";
import { OPEN_ACTION, TOOLS_ITEM, leftWords } from "../setup-words";
import { SetupRing } from "../SetupRing";
import { ToolsItemBody } from "./ToolsItem";
import { revealInList } from "../../../lib/reveal";

type Step = (typeof SETUP_STEPS)[number];

/** The open item: its accent edge on a tinted surface (the mock-up's). */
const OPEN = "rounded-xl bg-[color-mix(in_srgb,var(--color-accent)_8%,var(--color-surface))] p-3 inset-ring-[1.5px] inset-ring-accent";
const ROW = "grid w-full grid-cols-[22px_minmax(0,1fr)_auto] items-start gap-x-3 rounded-[10px] px-2.5 py-2 text-left outline-none transition-colors focus-visible:ring-2 focus-visible:ring-accent/60";

/** A row's mark: a tick when done, its number when open. */
function Mark({ done, n }: { done: boolean; n: number }): JSX.Element {
  return (
    <span aria-hidden="true" className={"mt-px flex size-[22px] items-center justify-center rounded-full text-label font-semibold " + (done ? "bg-accent-soft text-accent-hi" : "bg-accent-fill text-white")}>
      {done ? <IconCheck width={12} height={12} strokeWidth={2.6} /> : n}
    </span>
  );
}

export function PoolChecklist({
  rows,
  onOpen,
  compact = false,
  ready = false,
  next: nextWords = null,
  toolsLeft = null,
  blockers = [],
  prompt = null,
  title = "Set up this pool",
  values = {},
  tools = null,
  primary = true,
}: {
  /** Round 6: what to tell the lead first, with Copy (`FirstPrompt`). */
  prompt?: ReactNode;
  /** Round 6: what keeps the pool from working (`setupBlockers`): said
   * plainly, and the setup is never "done" or "ready" while any is. */
  blockers?: readonly string[];
  rows: readonly SetupRow[];
  onOpen: (step: Step) => void;
  compact?: boolean;
  /** Who's in set and the team tools on (`setupReady`). */
  ready?: boolean;
  /** What happens next, once ready (`nextLine`). */
  next?: string | null;
  /** Every step done, the team tools not on and not an item here: what's
   * left (`toolsLeftLine`). */
  toolsLeft?: string | null;
  /** Its title ("Set up web team"). */
  title?: string;
  /** Each step's value, under its name. */
  values?: Partial<Record<Step, string>>;
  /** D-126: where the team tools stand, as the last item (null: no item). */
  tools?: TeamTools | null;
  /** The open item's action is the view's one filled primary. */
  primary?: boolean;
}): JSX.Element | null {
  const toolsDone = tools?.kind === "on";
  const done = rows.filter((row) => row.done).length + (tools && toolsDone ? 1 : 0);
  const total = rows.length + (tools ? 1 : 0);
  const blocked = blockers.length > 0;
  const steps = (
    <ol className="flex flex-col gap-0.5">
      {rows.map((row, at) => (
        <StepRow key={row.step} row={row} n={at + 1} open={false} value={values[row.step]} primary={primary} onOpen={onOpen} />
      ))}
      {tools && <ToolsRow state={tools} n={rows.length + 1} open={false} primary={primary} />}
    </ol>
  );
  if (ready && !blocked) return <Ready next={nextWords} compact={compact} prompt={prompt} done={done} total={total} steps={steps} />;
  // Every step done but the team tools off: the list stays, all ticked,
  // the team tools open as its last item (or the line says what's left).
  const next = rows.find((row) => !row.done);
  const toolsOpen = !next && tools !== null && !toolsDone;
  if (compact) return <CompactLine onOpen={() => onOpen(next?.step ?? "who")} words={blocked ? `Can't work yet: ${blockers[0]}` : `${done} of ${total} done · ${nextWordsOf(next, toolsOpen)}`} />;
  return (
    <section aria-labelledby="pool-checklist-title" className="mx-4 mb-3 flex flex-col gap-3" data-testid="pool-checklist">
      <ChecklistHead title={title} done={done} total={total} blocked={blocked} toolsOpen={toolsOpen} />
      <OpenInView open={openOf(next, toolsOpen)}>
        {rows.map((row, at) => (
          <StepRow key={row.step} row={row} n={at + 1} open={row.step === next?.step} value={values[row.step]} primary={primary} onOpen={onOpen} />
        ))}
        {tools && (
          <ToolsRow state={tools} n={rows.length + 1} open={toolsOpen} primary={primary} />
        )}
      </OpenInView>
      <ChecklistFoot blockers={blockers} toolsLeft={!next && !tools ? toolsLeft : null} />
    </section>
  );
}

/** What keeps the pool from working, each in a line; else, with every
 * step done and the team tools no item here, what's left. */
function ChecklistFoot({ blockers, toolsLeft }: { blockers: readonly string[]; toolsLeft: string | null }): JSX.Element | null {
  if (blockers.length > 0)
    return (
      <ul className="flex flex-col gap-1 px-2" data-testid="pool-checklist-blocked">
        {blockers.map((line) => (
          <li key={line} className="text-meta text-warning">
            {line}
          </li>
        ))}
      </ul>
    );
  if (!toolsLeft) return null;
  return (
    <p className="px-2 text-meta text-ink-2" data-testid="pool-checklist-tools-left">
      {toolsLeft}
    </p>
  );
}

/** The steps; the open one whole in the panel's view, as the checklist
 * opens and when it moves on (at 1280 × 800 the last one, with the view's
 * one primary, sat under the panel's foot). */
function OpenInView({ open, children }: { open: string | null; children: ReactNode }): JSX.Element {
  const list = useRef<HTMLOListElement | null>(null);
  useLayoutEffect(() => {
    const item = list.current?.querySelector<HTMLElement>('[data-testid="pool-checklist-open"]');
    if (open && item) revealInList(item);
  }, [open]);
  return (
    <ol ref={list} className="flex flex-col gap-0.5">
      {children}
    </ol>
  );
}

/** Which item is open: the first step not done, else the team tools. */
const openOf = (next: SetupRow | undefined, toolsOpen: boolean): string | null => next?.step ?? (toolsOpen ? "tools" : null);

/** What's next, for the one line. */
function nextWordsOf(next: SetupRow | undefined, toolsOpen: boolean): string {
  if (next) return `next: ${STEP_WORDS[next.step].title}`;
  return toolsOpen ? `next: ${TOOLS_ITEM.title}` : "ready once the team tools are on";
}

/** Over something else selected: one line that opens the next step. */
function CompactLine({ words, onOpen }: { words: string; onOpen: () => void }): JSX.Element {
  return (
    <button
      type="button"
      onClick={onOpen}
      className="mx-4 mb-2 flex items-center gap-2 rounded-lg bg-surface-2 px-3 py-1.5 text-left text-meta text-ink-2 outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60"
      data-testid="pool-checklist-line"
    >
      <span className="section-label">Setup</span>
      <span className="min-w-0 flex-1 truncate">{words}</span>
    </button>
  );
}

/** The ring, the title and what's left (the count in words for screen
 * readers). */
function ChecklistHead({ title, done, total, blocked, toolsOpen }: { title: string; done: number; total: number; blocked: boolean; toolsOpen: boolean }): JSX.Element {
  return (
    <div className="flex items-center gap-3">
      <SetupRing done={done} total={total} blocked={blocked} decorative testId="pool-checklist-ring" />
      <div className="flex min-w-0 flex-col">
        <h4 id="pool-checklist-title" className="type-section">
          {title}
        </h4>
        <p className={"text-meta " + (blocked ? "font-medium text-warning" : "text-ink-2")}>{blocked ? "Can't work yet" : leftWords(total - done, toolsOpen)}</p>
      </div>
      <span className="sr-only" data-testid="pool-checklist-count">
        {blocked ? "Can't work yet" : `${done} of ${total} done`}
      </span>
    </div>
  );
}

/** One step: ticked with its value and Change, or open, expanded with
 * what it does and its action. */
function StepRow({ row, n, open, value, primary, onOpen }: { row: SetupRow; n: number; open: boolean; value: string | undefined; primary: boolean; onOpen: (step: Step) => void }): JSX.Element {
  const name = STEP_WORDS[row.step].short;
  const state = <span className="sr-only">{row.byDefault ? ", left at its default" : row.done ? ", done" : ", to do"}</span>;
  if (open)
    return (
      <li className={OPEN + " mt-1.5 flex flex-col gap-2.5"} data-testid="pool-checklist-open">
        <button type="button" onClick={() => onOpen(row.step)} aria-current="step" className={ROW + " -m-1 p-1 hover:bg-transparent"} data-testid="pool-checklist-step" data-step={row.step} data-done={row.done}>
          <Mark done={false} n={n} />
          <span className="flex min-w-0 flex-col gap-1">
            <span className="text-meta font-semibold text-ink">
              {name}
              {state}
            </span>
            <span className="text-meta text-ink-2">{STEP_WORDS[row.step].why}</span>
          </span>
        </button>
        <span className="pl-[34px]">
          <Button size="sm" variant={primary ? "primary" : "secondary"} onClick={() => onOpen(row.step)} data-testid="pool-checklist-action">
            {OPEN_ACTION[row.step]}
          </Button>
        </span>
      </li>
    );
  return (
    <li>
      <button type="button" onClick={() => onOpen(row.step)} className={ROW + " group hover:bg-surface-3/60"} data-testid="pool-checklist-step" data-step={row.step} data-done={row.done}>
        <Mark done n={n} />
        <span className="flex min-w-0 flex-col">
          <span className="text-meta font-medium text-ink">
            {name}
            {row.byDefault && (
              <span className="ml-1.5 text-label font-normal text-muted" data-testid="pool-checklist-default">
                default
              </span>
            )}
            {state}
          </span>
          {value && (
            <span className="truncate text-label text-muted" data-testid="pool-checklist-value">
              {value}
            </span>
          )}
        </span>
        <span className="mt-px text-label font-medium text-accent-hi group-hover:underline">Change</span>
      </button>
    </li>
  );
}

/** The team tools as the last item: open with its words and actions, or
 * ticked with its Change (Settings › Agent access). */
function ToolsRow({ state, n, open, primary }: { state: TeamTools; n: number; open: boolean; primary: boolean }): JSX.Element {
  if (state.kind === "on")
    return (
      <li>
        <button type="button" onClick={openAgentAccess} className={ROW + " group hover:bg-surface-3/60"} data-testid="pool-checklist-step" data-step="tools" data-done="true">
          <Mark done n={n} />
          <span className="flex min-w-0 flex-col">
            <span className="text-meta font-medium text-ink">
              {TOOLS_ITEM.title}
              <span className="sr-only">, done</span>
            </span>
            <span className="truncate text-label text-muted" data-testid="pool-checklist-value">
              {TOOLS_ITEM.on}
            </span>
          </span>
          <span className="mt-px text-label font-medium text-accent-hi group-hover:underline">Change</span>
        </button>
      </li>
    );
  return (
    <li className={(open ? OPEN + " mt-1.5 " : "px-2.5 py-2 ") + "flex flex-col gap-2.5"} data-testid={open ? "pool-checklist-open" : undefined} data-step="tools" data-done="false">
      <span className="grid grid-cols-[22px_minmax(0,1fr)] items-start gap-x-3" aria-current={open ? "step" : undefined}>
        <Mark done={false} n={n} />
        <span className="text-meta font-semibold text-ink">
          {TOOLS_ITEM.title}
          <span className="sr-only">, to do</span>
        </span>
      </span>
      {open && (
        <span className="pl-[34px]">
          <ToolsItemBody state={state} primary={primary} testId="pool-checklist-tools" />
        </span>
      )}
    </li>
  );
}

/** Ready: the pool works as it is; its ring full, and what happens next,
 * in one line. */
function Ready({ next, compact, prompt, done, total, steps }: { next: string | null; compact: boolean; prompt: ReactNode; done: number; total: number; steps: ReactNode }): JSX.Element {
  if (compact)
    return (
      <p className="mx-4 mb-2 flex items-center gap-2 rounded-lg bg-surface-2 px-3 py-1.5 text-meta text-ink-2" data-testid="pool-checklist-line">
        <span className="section-label">Setup</span>
        <span className="min-w-0 flex-1 truncate">Ready</span>
      </p>
    );
  return (
    <section aria-labelledby="pool-ready-title" className="mx-4 mb-3 flex flex-col gap-2.5" data-testid="pool-ready">
      <div className="flex items-center gap-3">
        <SetupRing done={done} total={total} decorative />
        <h4 id="pool-ready-title" className="type-section flex items-center gap-1.5">
          This pool is ready
          <IconCheck aria-hidden="true" width={14} height={14} strokeWidth={2.5} className="text-success" />
        </h4>
      </div>
      {/* Every step, ticked, each with its Change (Edit setup brings
          them back). */}
      {steps}
      {next && (
        <p className="text-meta text-ink-2" data-testid="pool-ready-next">
          <span className="font-medium text-ink">What happens next:</span> {next}.
        </p>
      )}
      {prompt}
    </section>
  );
}
