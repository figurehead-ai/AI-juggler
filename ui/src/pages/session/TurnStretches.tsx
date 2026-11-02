/**
 * 0.3.5 — a turn as the Claude app's Code tab draws it (the `stretches`
 * layout, `docs/research/native-conversation-display.md` §1): every text
 * in full, in order; each run of work between two texts as one line,
 * titled by the tool's own rules (the adapter writes the title), which
 * opens onto its calls; narration drawn like text. While the turn works,
 * its current run says what it's doing ("Running the tests"). No
 * whole-turn "Worked for" row: that is ChatGPT's Codex (`TurnWork`).
 */
import { useState, type JSX } from "react";
import type { TurnBlockInfo, TurnInfo } from "../../lib/ipc/conversations";
import { Markdown } from "./Markdown";
import { stepsSummary } from "./TurnWork";

export function Stretches({ turn, working }: { turn: TurnInfo; working: boolean }): JSX.Element {
  const last = turn.blocks.length - 1;
  return (
    <>
      {turn.blocks.map((block, index) =>
        block.kind === "steps" ? <Fold key={index} block={block} running={working && index === last} /> : block.text.trim() ? <Markdown key={index} text={block.text} /> : null,
      )}
    </>
  );
}

/** A run's line: its title, done or going. */
export function foldTitle(block: TurnBlockInfo, running: boolean): string {
  const title = running ? (block.title_running ?? block.title) : block.title;
  if (title) return title;
  return block.steps.length > 0 ? stepsSummary(block.steps) : running ? "Thinking" : "Thought";
}

function Chevron({ open }: { open: boolean }): JSX.Element {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className={"shrink-0 transition-transform " + (open ? "rotate-90" : "")}>
      <path d="M9 6l6 6-6 6" />
    </svg>
  );
}

function Fold({ block, running }: { block: TurnBlockInfo; running: boolean }): JSX.Element {
  const [open, setOpen] = useState(false);
  const title = foldTitle(block, running);
  const failed = block.calls.filter((call) => call.failed).length;
  const line = "inline-flex w-fit max-w-full items-center gap-2 rounded-lg px-1 py-1 text-left text-meta font-medium text-muted";
  const going = running && (
    <span aria-hidden="true" className="size-1.5 shrink-0 rounded-full bg-live motion-safe:animate-pulse" />
  );
  if (block.steps.length === 0) {
    // Thinking alone: nothing to open.
    return (
      <p className={line} data-testid="last-turn-fold" data-running={running ? "true" : undefined}>
        {going}
        <span className="truncate">{title}</span>
      </p>
    );
  }
  return (
    <div className="flex flex-col" data-testid="last-turn-fold" data-running={running ? "true" : undefined}>
      <button type="button" aria-expanded={open} onClick={() => setOpen(!open)} className={line + " outline-none transition-colors hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"} data-testid="last-turn-fold-toggle">
        <Chevron open={open} />
        {going}
        <span className="truncate">{title}</span>
        {failed > 0 && <span className="shrink-0 text-danger">· {failed === 1 ? "1 failed" : `${failed} failed`}</span>}
      </button>
      {open && (
        <ul className="mt-2 flex flex-col gap-1 border-l border-hairline pl-4" data-testid="last-turn-step-lines">
          {block.hidden > 0 && <li className="text-label text-muted">{block.hidden === 1 ? "1 earlier step" : `${block.hidden} earlier steps`} not shown</li>}
          {block.steps.map((step, index) => (
            <li key={index} className="break-words font-mono text-meta leading-[1.5] text-ink/90">
              {step}
              {block.calls[index]?.failed && <span className="ml-2 font-sans text-label font-medium text-danger">failed</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
