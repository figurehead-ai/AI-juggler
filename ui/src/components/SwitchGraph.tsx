/**
 * AJ-E209 — the per-Work Switch Graph timeline (the Project panel).
 * Nodes are epochs (Provider mark + Account label + kind + state),
 * edges are labelled arrows (`transplant F1 · 12:04` / `switch-back ↺`);
 * failed edges render dashed with their §9 reason and the
 * source-safe recovery. Epochs older than the last fork/merge
 * collapse to "+n switches" — the list is never unbounded. Clicking a
 * node offers Resume (only when the native session + transcript
 * survive), "Switch from here…", and "Export pack…" (through the
 * E208 redaction policy; the path lands in a toast line). Suggest
 * and act only on explicit clicks. A11y: nodes and failed edges are
 * buttons with ↑/↓ roving focus; every arrow is labelled;
 * ⌘R resumes the open node. Density: the collapse cut bounds each
 * timeline; long chains scroll horizontally.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import type { EpochNodeData, GraphEdgeData, WorkGraphData } from "../lib/bridge";
import { epochResume, packExport, switchGraphOverview, errorText,} from "../lib/bridge";
import { providerMark } from "./indicator";

/** One epoch node — a button (keyboard-first; the menu opens on
 * click and reads its actions as a list). `fork`/`merge` chips come
 * from the work's edge shape (text, never colour-only). */
function GraphNode({
  epoch,
  open,
  shape,
  onToggle,
}: {
  epoch: EpochNodeData;
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
      title={`${epoch.account_label} on ${epoch.provider}${epoch.resumable ? " — resumable" : ""}`}
      className={
        "flex shrink-0 flex-col items-center gap-1 rounded-lg border px-2.5 py-2 text-center text-[11px] transition-colors focus:outline-none focus:ring-2 focus:ring-accent " +
        (open
          ? "border-accent/60 bg-surface"
          : "border-hairline hover:border-accent/40 hover:bg-surface")
      }
    >
      <span className="flex items-center gap-1.5" aria-hidden="true">
        <Icon width={12} height={12} />
        <span className="font-medium">{epoch.account_label}</span>
      </span>
      <span className="flex items-center gap-1 text-[10px] text-muted">
        <span className="font-mono">{mark.abbrev}</span>
        <span aria-hidden="true">·</span>
        <span>{epoch.kind}</span>
        {shape.fork && (
          <span className="rounded border border-warning/50 px-1 text-warning" data-testid="node-fork-chip">
            fork
          </span>
        )}
        {shape.merge && (
          <span className="rounded border border-accent/50 px-1 text-accent" data-testid="node-merge-chip">
            merge
          </span>
        )}
      </span>
      <span className={`text-[10px] ${epoch.resumable ? "text-success" : "text-muted"}`}>
        {epoch.resumable ? "resumable" : "no transcript"}
      </span>
      <span className="sr-only">
        {`, provider ${epoch.provider}, ${epoch.kind} instance${shape.fork ? ", forks here" : ""}${shape.merge ? ", converges here" : ""}`}
      </span>
    </button>
  );
}

/** One edge — an inline labelled arrow; failed edges are dashed-red
 * buttons that open the §9 reason + recovery. */
function GraphEdge({
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
        className="flex shrink-0 flex-col items-center gap-0.5 rounded px-2 py-1 text-[10px] text-danger focus:outline-none focus:ring-2 focus:ring-accent"
      >
        <span aria-hidden="true" className="tracking-widest">
          ✕ ┄┄▶
        </span>
        <span className="underline decoration-dashed">failed — view reason</span>
        <span className="sr-only">{`Switch failed: ${edge.label}`}</span>
      </button>
    );
  }
  return (
    <span
      data-testid="switch-graph-edge"
      role="img"
      aria-label={edge.label}
      className="flex shrink-0 select-none flex-col items-center gap-0.5 px-1 text-[10px] text-muted"
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

export function SwitchGraph({
  projectId,
  onOpenSession,
}: {
  projectId: string;
  /** Opens the epoch's session inspector ("Switch from here…" and
   * the failed-edge recovery both route through the source's own
   * ranked targets — never a direct launch). */
  onOpenSession: (sessionId: string) => void;
}): JSX.Element {
  const [works, setWorks] = useState<WorkGraphData[] | null>(null);
  const [openNode, setOpenNode] = useState<string | null>(null);
  const [openFailedEdge, setOpenFailedEdge] = useState<string | null>(null);
  const [actionLine, setActionLine] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const load = useCallback(() => {
    let alive = true;
    switchGraphOverview(projectId)
      .then((rows) => {
        if (alive) setWorks(rows);
      })
      .catch(() => {
        if (alive) setWorks([]);
      });
    return () => {
      alive = false;
    };
  }, [projectId]);

  useEffect(() => load(), [load]);

  // ⌘R resumes the open node (the graph's keyboard shortcut).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== "r") return;
      const node = openNode;
      if (node === null) return;
      const epoch = (works ?? []).flatMap((work) => work.epochs).find((e) => e.id === node);
      if (!epoch?.resumable) return;
      event.preventDefault();
      void doResume(epoch.id);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [openNode, works]);

  const doResume = async (epochId: string) => {
    setActionError(null);
    setActionLine(null);
    try {
      const result = await epochResume(epochId);
      setActionLine(`Resumed — instance ${result.session_id} is starting.`);
      onOpenSession(result.session_id);
    } catch (cause) {
      setActionError(errorText(cause));
    }
  };

  const doExport = async (epochId: string) => {
    setActionError(null);
    setActionLine(null);
    try {
      const path = await packExport(epochId);
      setActionLine(`Pack exported to ${path}`);
    } catch (cause) {
      setActionError(errorText(cause));
    }
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const delta = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (delta === 0) return;
    const buttons = Array.from(
      listRef.current?.querySelectorAll<HTMLButtonElement>("button[data-row]") ?? [],
    );
    if (buttons.length === 0) return;
    event.preventDefault();
    const index = buttons.findIndex((button) => button === document.activeElement);
    buttons[(index + delta + buttons.length) % buttons.length].focus();
  };

  if (works === null) {
    return (
      <p className="text-xs text-muted" data-testid="switch-graph-loading">
        Loading the switch graph…
      </p>
    );
  }
  if (works.length === 0) {
    return (
      <p className="text-xs text-muted" data-testid="switch-graph-empty">
        No switches yet — your work starts here.
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-3" data-testid="switch-graph">
      {works.map((work) => {
        const openEpoch = work.epochs.find((epoch) => epoch.id === openNode) ?? null;
        const failedEdge =
          work.edges.find(
            (edge) =>
              !!edge.failed_reason &&
              edge.from + "->" + (edge.to ?? "none") === openFailedEdge,
          ) ?? null;
        const failedSource =
          failedEdge === null
            ? null
            : (work.epochs.find((epoch) => epoch.id === failedEdge.from) ?? null);
        return (
          <div
            key={work.title}
            className="flex flex-col gap-1.5"
            data-testid="switch-graph-work"
          >
            <div className="flex flex-wrap items-baseline gap-x-2">
              <p className="text-xs font-medium">{work.title}</p>
              {work.hop_note !== null && work.hop_note !== undefined && (
                <p className="text-[10px] text-warning" data-testid="switch-graph-hop-note">
                  {work.hop_note}
                </p>
              )}
            </div>
            <div
              ref={listRef}
              onKeyDown={onKeyDown}
              className="flex items-center gap-1 overflow-x-auto pb-1"
              role="group"
              aria-label={`Timeline for ${work.title}`}
            >
              {work.collapsed > 0 && (
                <span
                  data-testid="switch-graph-collapse"
                  title={`Collapsed: ${work.collapsed} older switches stay in the graph store`}
                  className="shrink-0 rounded-full border border-hairline px-2 py-0.5 text-[10px] text-muted"
                >
                  +{work.collapsed} switches
                </span>
              )}
              {work.epochs.map((epoch, index) => {
                // The incoming arrow: the confirmed edge that CREATED
                // this epoch (never a failed one — failures never
                // created anything).
                const incoming =
                  index === 0
                    ? null
                    : (work.edges.find(
                        (edge) => edge.to === epoch.id && !edge.failed_reason,
                      ) ?? null);
                // Failed attempts trail their SOURCE node — a failed
                // switch is part of how the work travelled (Req.4).
                const trailingFailed = work.edges.filter(
                  (edge) => edge.from === epoch.id && !!edge.failed_reason,
                );
                const edgeKey = (from: string, to: string | null) => `${from}->${to ?? "none"}`;
                // The edge shape around this epoch: fork = 2+ confirmed
                // edges out, merge = 2+ confirmed edges in (Req.4).
                const confirmedOut = work.edges.filter(
                  (edge) => edge.from === epoch.id && !edge.failed_reason,
                ).length;
                const confirmedIn = work.edges.filter(
                  (edge) => edge.to === epoch.id && !edge.failed_reason,
                ).length;
                return (
                  <span key={epoch.id} className="flex shrink-0 items-center gap-1">
                    {incoming !== null && (
                      <GraphEdge
                        edge={incoming}
                        open={false}
                        onToggle={() => {}}
                      />
                    )}
                    <GraphNode
                      epoch={epoch}
                      open={openNode === epoch.id}
                      shape={{ fork: confirmedOut > 1, merge: confirmedIn > 1 }}
                      onToggle={() =>
                        setOpenNode((current) => (current === epoch.id ? null : epoch.id))
                      }
                    />
                    {trailingFailed.map((failed) => (
                      <GraphEdge
                        key={failed.label}
                        edge={failed}
                        open={openFailedEdge === edgeKey(failed.from, failed.to ?? null)}
                        onToggle={() =>
                          setOpenFailedEdge((current) =>
                            current === edgeKey(failed.from, failed.to ?? null)
                              ? null
                              : edgeKey(failed.from, failed.to ?? null),
                          )
                        }
                      />
                    ))}
                  </span>
                );
              })}
            </div>

            {failedEdge !== null && (
              <div
                data-testid="switch-graph-failed-detail"
                className="rounded-lg border border-danger/40 bg-danger-soft px-2.5 py-2 text-[11px]"
              >
                <p className="text-danger">
                  Switch failed: {failedEdge.label}
                  {failedEdge.failed_reason ? ` — ${failedEdge.failed_reason}` : ""}
                </p>
                <p className="mt-0.5 text-muted">
                  Nothing was half-moved: the source stayed resumable and no target instance
                  was left behind. Recovery starts from the source — its own ranked targets
                  are one click away.
                </p>
                <div className="mt-1.5 flex gap-2">
                  <button
                    type="button"
                    disabled={!failedSource?.session_id}
                    title={
                      failedSource?.session_id
                        ? "Opens the source epoch's session"
                        : "The source epoch has no live session row"
                    }
                    onClick={() => {
                      if (failedSource?.session_id) onOpenSession(failedSource.session_id);
                    }}
                    className="rounded border border-hairline px-2 py-1 text-[11px] transition-colors hover:border-accent/40 disabled:opacity-50"
                  >
                    Open the source epoch
                  </button>
                </div>
              </div>
            )}

            {openEpoch !== null && failedEdge === null && (
              <div
                data-testid="switch-graph-node-menu"
                className="flex flex-col gap-1.5 rounded-lg border border-hairline bg-surface px-2.5 py-2"
              >
                <p className="text-[10px] uppercase tracking-wider text-muted">
                  {openEpoch.account_label} · {providerMark(openEpoch.provider).abbrev}
                </p>
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    data-testid="epoch-resume-button"
                    disabled={!openEpoch.resumable}
                    title={
                      openEpoch.resumable
                        ? "Resumes this epoch's native session (⌘R)"
                        : "No native session transcript survives for this epoch"
                    }
                    onClick={() => void doResume(openEpoch.id)}
                    className="rounded border border-accent/40 px-2 py-1 text-[11px] text-accent-hi transition-colors hover:border-accent/60 disabled:border-hairline disabled:text-faint"
                  >
                    Resume {openEpoch.resumable && "(⌘R)"}
                  </button>
                  <button
                    type="button"
                    data-testid="epoch-switch-from-button"
                    disabled={openEpoch.session_id === undefined || openEpoch.session_id === null}
                    title={
                      openEpoch.session_id
                        ? "Opens this epoch's session — its ranked switch targets are there"
                        : "This epoch has no live session row"
                    }
                    onClick={() => {
                      if (openEpoch.session_id) onOpenSession(openEpoch.session_id);
                    }}
                    className="rounded border border-hairline px-2 py-1 text-[11px] transition-colors hover:border-accent/40 disabled:opacity-50"
                  >
                    Switch from here…
                  </button>
                  <button
                    type="button"
                    data-testid="epoch-export-button"
                    onClick={() => void doExport(openEpoch.id)}
                    title="Exports this epoch's pack through your redaction policy"
                    className="rounded border border-hairline px-2 py-1 text-[11px] transition-colors hover:border-accent/40"
                  >
                    Export pack…
                  </button>
                </div>
              </div>
            )}
          </div>
        );
      })}
      {actionLine !== null && (
        <p className="text-[11px] text-success" data-testid="pack-export-toast">
          {actionLine}
        </p>
      )}
      {actionError !== null && (
        <p className="text-[11px] text-danger" data-testid="switch-graph-action-error">
          {actionError}
        </p>
      )}
    </div>
  );
}
