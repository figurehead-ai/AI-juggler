/**
 * AJ-E209 — the per-Work Switch Graph timeline (the Project panel).
 * Nodes are epochs (Provider mark + Account label + kind + state),
 * edges are labelled arrows (what carried: `Full story, fresh start` / `Switched back · …`);
 * failed edges render dashed with their §9 reason and the
 * source-safe recovery. Epochs older than the last fork/merge
 * collapse to "+n switches" — the list is never unbounded. Clicking a
 * node offers Resume (only when the native session + transcript
 * survive), "Switch from here…", and "Save handover…" (through the
 * E208 redaction policy; where it went, what was hidden, Open the
 * handover and Reveal in Finder). 0.3: an edge whose switch wrote a
 * handover (to another tool) is a button that opens what that tool was
 * told: one line and the handover, read-only. Suggest and act only on
 * explicit clicks. A11y: nodes and failed edges are
 * buttons with ↑/↓ roving focus; every arrow is labelled;
 * ⌘R resumes the open node. Density: the collapse cut bounds each
 * timeline; long chains scroll horizontally.
 */
import { useCallback, useEffect, useRef, useState, type JSX } from "react";
import { epochResume, packExport, switchGraphOverview, errorText, type WorkGraphData } from "../lib/bridge";
import type { SavedHandoverInfo } from "../lib/ipc/handover";
import { providerMark } from "./indicator";
import { SavedHandover, SwitchHandover } from "./HandoverView";
import { GraphEdge, GraphNode } from "./switch/GraphParts";

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
  // The edge whose handover is open (its id).
  const [openEdge, setOpenEdge] = useState<string | null>(null);
  const [actionLine, setActionLine] = useState<string | null>(null);
  const [saved, setSaved] = useState<SavedHandoverInfo | null>(null);
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
    setSaved(null);
    try {
      const result = await epochResume(epochId);
      setActionLine(`Resumed — session ${result.session_id} is starting.`);
      onOpenSession(result.session_id);
    } catch (cause) {
      setActionError(errorText(cause));
    }
  };

  const doExport = async (epochId: string) => {
    setActionError(null);
    setActionLine(null);
    setSaved(null);
    try {
      setSaved(await packExport(epochId));
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
      <p className="text-xs text-ink-2" data-testid="switch-graph-loading">
        Loading the switch graph…
      </p>
    );
  }
  if (works.length === 0) {
    return (
      <p className="text-xs text-ink-2" data-testid="switch-graph-empty">
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
        const handoverEdge =
          openEdge === null ? null : (work.edges.find((edge) => edge.id === openEdge && !!edge.handover) ?? null);
        return (
          <div
            key={work.title}
            className="flex flex-col gap-1.5"
            data-testid="switch-graph-work"
          >
            <div className="flex flex-wrap items-baseline gap-x-2">
              <p className="text-xs font-medium">{work.title}</p>
              {work.hop_note !== null && work.hop_note !== undefined && (
                <p className="text-label text-warning" data-testid="switch-graph-hop-note">
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
                  className="shrink-0 rounded-full border border-hairline px-2 py-0.5 text-label text-muted"
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
                        open={incoming.id !== undefined && openEdge === incoming.id}
                        onToggle={() => {
                          setOpenFailedEdge(null);
                          setOpenEdge((current) => (current === incoming.id ? null : (incoming.id ?? null)));
                        }}
                      />
                    )}
                    <GraphNode
                      epoch={epoch}
                      open={openNode === epoch.id}
                      shape={{ fork: confirmedOut > 1, merge: confirmedIn > 1 }}
                      onToggle={() => {
                        setOpenEdge(null);
                        setOpenNode((current) => (current === epoch.id ? null : epoch.id));
                      }}
                    />
                    {trailingFailed.map((failed) => (
                      <GraphEdge
                        key={failed.label}
                        edge={failed}
                        open={openFailedEdge === edgeKey(failed.from, failed.to ?? null)}
                        onToggle={() => {
                          setOpenEdge(null);
                          setOpenFailedEdge((current) =>
                            current === edgeKey(failed.from, failed.to ?? null)
                              ? null
                              : edgeKey(failed.from, failed.to ?? null),
                          );
                        }}
                      />
                    ))}
                  </span>
                );
              })}
            </div>

            {failedEdge !== null && (
              <div
                data-testid="switch-graph-failed-detail"
                className="rounded-lg border border-danger/40 bg-danger-soft px-2.5 py-2 text-label"
              >
                <p className="text-danger">
                  Switch failed: {failedEdge.label}
                  {failedEdge.failed_reason ? ` — ${failedEdge.failed_reason}` : ""}
                </p>
                <p className="mt-0.5 text-ink-2">
                  Nothing was half-moved: the source stayed resumable and no target session
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
                    className="rounded border border-hairline px-2 py-1 text-label transition-colors hover:border-accent/40 disabled:opacity-50"
                  >
                    Open the source epoch
                  </button>
                </div>
              </div>
            )}

            {handoverEdge?.handover && handoverEdge.id && failedEdge === null && (
              <div
                data-testid="switch-graph-edge-detail"
                className="flex flex-col gap-1.5 rounded-lg border border-hairline bg-surface px-2.5 py-2"
              >
                <p className="text-section font-semibold text-ink">
                  What {handoverEdge.handover.to_tool} was told
                </p>
                <SwitchHandover edgeId={handoverEdge.id} summary={handoverEdge.handover} />
              </div>
            )}

            {openEpoch !== null && failedEdge === null && handoverEdge === null && (
              <div
                data-testid="switch-graph-node-menu"
                className="flex flex-col gap-1.5 rounded-lg border border-hairline bg-surface px-2.5 py-2"
              >
                <p className="text-section font-semibold text-ink">
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
                    className="rounded border border-accent/40 px-2 py-1 text-label text-accent-hi transition-colors hover:border-accent/60 disabled:border-hairline disabled:text-muted"
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
                    className="rounded border border-hairline px-2 py-1 text-label transition-colors hover:border-accent/40 disabled:opacity-50"
                  >
                    Switch from here…
                  </button>
                  <button
                    type="button"
                    data-testid="epoch-export-button"
                    onClick={() => void doExport(openEpoch.id)}
                    title="Saves this step's handover, with keys and passwords handled as you chose"
                    className="rounded border border-hairline px-2 py-1 text-label transition-colors hover:border-accent/40"
                  >
                    Save handover…
                  </button>
                </div>
              </div>
            )}
          </div>
        );
      })}
      {actionLine !== null && (
        <p className="text-label text-success" data-testid="pack-export-toast">
          {actionLine}
        </p>
      )}
      {saved !== null && (
        <div data-testid="pack-export-toast">
          <SavedHandover saved={saved} inline />
        </div>
      )}
      {actionError !== null && (
        <p className="text-label text-danger" data-testid="switch-graph-action-error">
          {actionError}
        </p>
      )}
    </div>
  );
}
