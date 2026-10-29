/**
 * AJ-E209 — the Switch Graph's parts: an epoch node and a labelled edge
 * (`SwitchGraph` lays them out). A failed edge and, 0.3, an edge whose
 * switch wrote a handover are buttons that open their detail.
 */
import type { JSX } from "react";
import { kindWords } from "../../lib/status-words";
import { providerDisplayName } from "../../lib/providers";
import type { EpochTask } from "../../lib/ipc/tasks";
import type { EpochNodeData, GraphEdgeData } from "../../lib/bridge";
import { providerMark } from "../indicator";

/** One epoch node — a button (keyboard-first; the menu opens on
 * click and reads its actions as a list). `fork`/`merge` chips come
 * from the work's edge shape (text, never colour-only). */
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
  const mark = providerMark(epoch.provider);
  const Icon = mark.Icon;
  return (
    <button
      type="button"
      data-row
      data-testid="switch-graph-node"
      data-resumable={epoch.resumable ? "true" : "false"}
      aria-pressed={open}
      aria-expanded={open}
      onClick={onToggle}
      title={`${epoch.account_label} on ${providerDisplayName(epoch.provider)}${epoch.resumable ? " — resumable" : ""}`}
      className={
        "flex shrink-0 flex-col items-center gap-1 rounded-lg border px-2.5 py-2 text-center text-label transition-colors focus:outline-none focus:ring-2 focus:ring-accent " +
        (open
          ? "border-accent/60 bg-surface"
          : "border-hairline hover:border-accent/40 hover:bg-surface")
      }
    >
      <span className="flex items-center gap-1.5" aria-hidden="true">
        <Icon width={12} height={12} />
        <span className="font-medium">{epoch.account_label}</span>
      </span>
      <span className="flex items-center gap-1 text-label text-muted">
        <span className="font-mono">{mark.abbrev}</span>
        <span aria-hidden="true">·</span>
        <span>{kindWords(epoch.kind)}</span>
        {shape.fork && <span className="rounded border border-warning/50 px-1 text-warning" data-testid="node-fork-chip">fork</span>}
        {shape.merge && <span className="rounded border border-accent/50 px-1 text-accent" data-testid="node-merge-chip">merge</span>}
      </span>
      <span className={`text-label ${epoch.resumable ? "text-success" : "text-muted"}`}>
        {epoch.resumable ? "resumable" : "no transcript"}
      </span>
      {/* AJ-E1904: the task this session worked; a switch keeps it. */}
      {epoch.task && <span className="max-w-[12rem] truncate text-label text-ink" title={epoch.task} data-testid="node-task">{epoch.task}</span>}
      <span className="sr-only">
        {`, ${providerDisplayName(epoch.provider)}, ${kindWords(epoch.kind).toLowerCase()} session${shape.fork ? ", forks here" : ""}${shape.merge ? ", converges here" : ""}${epoch.task ? `, its task ${epoch.task}` : ""}`}
      </span>
    </button>
  );
}

/** One edge — an inline labelled arrow; failed edges are dashed-red
 * buttons that open the §9 reason + recovery; an edge with a handover is
 * a button that opens what the other tool was told. */
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
        className="flex shrink-0 flex-col items-center gap-0.5 rounded px-2 py-1 text-label text-danger focus:outline-none focus:ring-2 focus:ring-accent"
      >
        <span aria-hidden="true" className="tracking-widest">
          ✕ ┄┄▶
        </span>
        <span className="underline decoration-dashed">failed — view reason</span>
        <span className="sr-only">{`Switch failed: ${edge.label}`}</span>
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
        onClick={onToggle}
        title={`What ${edge.handover.to_tool} was told`}
        className="flex shrink-0 flex-col items-center gap-0.5 rounded px-1 text-label text-muted hover:text-ink focus:outline-none focus:ring-2 focus:ring-accent"
      >
        <span aria-hidden="true" className="tracking-widest">
          {edge.switch_back ? "↺" : "→"}
        </span>
        <span className="max-w-[9rem] truncate text-center underline decoration-dotted underline-offset-2">{edge.label}</span>
        <span className="sr-only">{`, what ${edge.handover.to_tool} was told`}</span>
      </button>
    );
  }
  return (
    <span
      data-testid="switch-graph-edge"
      role="img"
      aria-label={edge.label}
      className="flex shrink-0 select-none flex-col items-center gap-0.5 px-1 text-label text-muted"
    >
      <span aria-hidden="true" className="tracking-widest">
        {edge.switch_back ? "↺" : "→"}
      </span>
      <span className="max-w-[9rem] truncate text-center" title={edge.label}>
        {edge.label}
      </span>
    </span>
  );
}
