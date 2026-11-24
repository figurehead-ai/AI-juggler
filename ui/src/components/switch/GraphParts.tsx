/**
 * AJ-E209 — the Switch Graph's parts: an epoch (one step of how the work
 * travelled) and a labelled move between two (`SwitchGraph` lists them).
 * A failed move and, 0.3, a move whose switch wrote a handover are
 * buttons that open their detail.
 *
 * 0.5.1 F78 (D-142): drawn on the switch dialog's kit, as F71 did for
 * switching back: each step is the switch's target card (`TargetButton`:
 * the App's logo, its name, how it ran, tags as neutral pills; a tone
 * step, no outline), the moves between them a quiet line with the words
 * whole (never cut), one step under another (no sideways scroll), and a
 * failed move a red mark with plain words (no red box, no "failed ·").
 */
import type { JSX } from "react";
import { kindWords } from "../../lib/status-words";
import { cliAppName } from "../../lib/app-name";
import type { EpochTask } from "../../lib/ipc/tasks";
import type { EpochNodeData, GraphEdgeData } from "../../lib/ipc/switch-targets";
import { Pill } from "../Pill";
import { TargetButton, TargetLogo } from "./TargetCard";

/** One step: the switch's target card, a button that opens its actions
 * (keyboard-first; ↑/↓ move between steps). Branching is said in a
 * neutral pill (words, never colour alone). */
export function GraphNode({
  epoch,
  open,
  shape,
  onToggle,
}: {
  epoch: EpochNodeData & EpochTask;
  open: boolean;
  shape: { fork: boolean; merge: boolean };
  onToggle: () => void;
}): JSX.Element {
  const name = cliAppName(epoch.provider, epoch.account_label);
  const tags =
    shape.fork || shape.merge ? (
      <>
        {shape.fork && (
          <span className="contents" data-testid="node-fork-chip">
            <Pill tone="neutral">Branches here</Pill>
          </span>
        )}
        {shape.merge && (
          <span className="contents" data-testid="node-merge-chip">
            <Pill tone="neutral">Branches meet here</Pill>
          </span>
        )}
      </>
    ) : undefined;
  return (
    <TargetButton
      data-row
      data-testid="switch-graph-node"
      data-resumable={epoch.resumable ? "true" : "false"}
      aria-expanded={open}
      onClick={onToggle}
      chosen={open}
      logo={<TargetLogo tool={epoch.provider} kind={epoch.kind === "desktop" ? "desktop" : "cli"} folderId="" label={epoch.account_label} size={32} />}
      name={name}
      sub={`${kindWords(epoch.kind)} · ${epoch.resumable ? "can be resumed" : "no conversation kept"}`}
      tags={tags}
      detail={
        // AJ-E1904: the task this session worked; a switch keeps it.
        epoch.task ? (
          <span className="flex min-w-0 gap-1.5">
            <span className="shrink-0 text-muted">Task</span>
            <span className="min-w-0 truncate text-ink" title={epoch.task} data-testid="node-task">
              {epoch.task}
            </span>
          </span>
        ) : undefined
      }
    />
  );
}

/** The arrow's glyph: a switch back turns round. */
function Arrow({ back }: { back: boolean }): JSX.Element {
  return (
    <span aria-hidden="true" className="w-4 shrink-0 text-center text-faint">
      {back ? "↺" : "↓"}
    </span>
  );
}

const LINE = "flex w-full min-w-0 items-start gap-2 px-4 py-1.5 text-left text-meta";

/** One move between two steps: a quiet line, its words whole. A failed
 * move is a button with a red mark that opens why and the way back; a
 * move whose switch wrote a handover is a button that opens what the
 * other tool was told. */
export function GraphEdge({
  edge,
  open,
  onToggle,
}: {
  edge: GraphEdgeData;
  open: boolean;
  onToggle: () => void;
}): JSX.Element {
  const failed = edge.failed_reason !== undefined && edge.failed_reason !== null;
  if (failed) {
    return (
      <button
        type="button"
        data-testid="switch-graph-failed-edge"
        aria-expanded={open}
        onClick={onToggle}
        className={LINE + " rounded-lg text-ink outline-none hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60"}
      >
        <span aria-hidden="true" className="w-4 shrink-0 text-center font-semibold text-danger">
          ✕
        </span>
        <span className="min-w-0">
          A switch from here didn't happen <span className="text-muted">· {open ? "hide why" : "why"}</span>
        </span>
      </button>
    );
  }
  if (edge.handover && edge.id) {
    return (
      <button
        type="button"
        data-testid="switch-graph-edge"
        data-handover="true"
        aria-expanded={open}
        aria-label={`${edge.label}, what ${edge.handover.to_tool} was told`}
        onClick={onToggle}
        className={LINE + " rounded-lg text-ink-2 outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"}
      >
        <Arrow back={edge.switch_back} />
        <span className="min-w-0 [overflow-wrap:anywhere]">
          {edge.label} <span className="text-muted underline decoration-dotted underline-offset-2">· what {edge.handover.to_tool} was told</span>
        </span>
      </button>
    );
  }
  return (
    <span data-testid="switch-graph-edge" role="img" aria-label={edge.label} className={LINE + " select-none text-ink-2"}>
      <Arrow back={edge.switch_back} />
      <span aria-hidden="true" className="min-w-0 [overflow-wrap:anywhere]">
        {edge.label}
      </span>
    </span>
  );
}
