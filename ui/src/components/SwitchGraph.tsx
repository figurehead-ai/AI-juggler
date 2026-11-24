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
import { epochResume, packExport, switchGraphOverview, type WorkGraphData } from "../lib/ipc/switch-targets";
import { errorText } from "../lib/bridge";
import { ariaOf, keysOf } from "../lib/shortcuts";
import type { SavedHandoverInfo } from "../lib/ipc/handover";
import { Button } from "./primitives";
import { cliAppName } from "../lib/app-name";
import type { EpochNodeData } from "../lib/ipc/switch-targets";
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
  const listRef = useRef<HTMLOListElement>(null);

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

  const onKeyDown = (event: React.KeyboardEvent<HTMLOListElement>) => {
    const delta = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (delta === 0) return;
    const buttons = Array.from(
      listRef.current?.querySelectorAll<HTMLButtonElement>("button[data-row]") ?? [],
    );
    if (buttons.length === 0) return;
    event.preventDefault();
    const index = buttons.findIndex((button) => button === document.activeElement);
    buttons[(index + delta + buttons.length) % buttons.length]!.focus();
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
    <div className="flex flex-col gap-5" data-testid="switch-graph">
      {works.map((work) => {
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
        const edgeKey = (from: string, to: string | null) => `${from}->${to ?? "none"}`;
        return (
          <section
            key={work.title}
            className="flex flex-col gap-1.5"
            aria-label={work.title}
            data-testid="switch-graph-work"
          >
            <div className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
              <h4 className="text-body font-semibold text-ink">{work.title}</h4>
              {work.hop_note !== null && work.hop_note !== undefined && (
                <p className="text-meta text-muted" data-testid="switch-graph-hop-note">
                  {work.hop_note}
                </p>
              )}
            </div>
            {work.collapsed > 0 && (
              <p data-testid="switch-graph-collapse" className="text-meta text-muted">
                {work.collapsed === 1 ? "1 earlier switch" : `${work.collapsed} earlier switches`}, kept in its history
              </p>
            )}
            {/* One step under another, each a target card, the moves
                between them a line with their words whole (F78). */}
            <ol
              ref={listRef}
              onKeyDown={onKeyDown}
              className="flex flex-col gap-1"
              role="group"
              aria-label={`Timeline for ${work.title}`}
            >
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
                // The edge shape around this epoch: fork = 2+ confirmed
                // edges out, merge = 2+ confirmed edges in (Req.4).
                const confirmedOut = work.edges.filter(
                  (edge) => edge.from === epoch.id && !edge.failed_reason,
                ).length;
                const confirmedIn = work.edges.filter(
                  (edge) => edge.to === epoch.id && !edge.failed_reason,
                ).length;
                const handover = incoming?.handover && incoming.id && openEdge === incoming.id ? incoming : null;
                return (
                  <li key={epoch.id} className="flex flex-col gap-1">
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
                    {handover?.handover && handover.id && (
                      <div data-testid="switch-graph-edge-detail" className="ml-10 flex flex-col gap-1.5 pb-1">
                        <p className="text-meta font-semibold text-ink">What {handover.handover.to_tool} was told</p>
                        <SwitchHandover edgeId={handover.id} summary={handover.handover} />
                      </div>
                    )}
                    <GraphNode
                      epoch={epoch}
                      open={openNode === epoch.id}
                      shape={{ fork: confirmedOut > 1, merge: confirmedIn > 1 }}
                      onToggle={() => {
                        setOpenEdge(null);
                        setOpenFailedEdge(null);
                        setOpenNode((current) => (current === epoch.id ? null : epoch.id));
                      }}
                    />
                    {openNode === epoch.id && <StepActions epoch={epoch} onResume={() => void doResume(epoch.id)} onExport={() => void doExport(epoch.id)} onOpenSession={onOpenSession} />}
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
                    {failedEdge !== null && failedEdge.from === epoch.id && (
                      <div data-testid="switch-graph-failed-detail" className="ml-10 flex flex-col gap-2 pb-1 text-meta">
                        <p className="text-ink">
                          It didn't switch{failedEdge.failed_reason ? `: ${failedEdge.failed_reason}` : ""}.
                        </p>
                        <p className="text-ink-2">
                          Nothing was half-moved: the session it started from stayed resumable and nothing
                          was left on the other side. Switch again from that session.
                        </p>
                        {failedSource?.session_id && (
                          <Button
                            size="sm"
                            variant="secondary"
                            className="self-start"
                            onClick={() => {
                              if (failedSource.session_id) onOpenSession(failedSource.session_id);
                            }}
                          >
                            Open the session it started from
                          </Button>
                        )}
                      </div>
                    )}
                  </li>
                );
              })}
            </ol>
          </section>
        );
      })}
      {actionLine !== null && (
        <p className="text-meta text-success" data-testid="pack-export-toast">
          {actionLine}
        </p>
      )}
      {saved !== null && (
        <div data-testid="pack-export-toast">
          <SavedHandover saved={saved} inline />
        </div>
      )}
      {actionError !== null && (
        <p className="text-meta text-ink" data-testid="switch-graph-action-error">
          <span aria-hidden="true" className="mr-1.5 font-semibold text-danger">✕</span>
          {actionError}
        </p>
      )}
    </div>
  );
}

/** An open step's actions, the kit's buttons: Resume where its
 * conversation was kept, Switch from here… where its session is still
 * listed, Save handover… always (D-127: what can't run isn't offered;
 * the card says why, "no conversation kept"). */
function StepActions({ epoch, onResume, onExport, onOpenSession }: { epoch: EpochNodeData; onResume: () => void; onExport: () => void; onOpenSession: (sessionId: string) => void }): JSX.Element {
  const key = keysOf("epoch-resume")[0];
  return (
    <div data-testid="switch-graph-node-menu" data-step={cliAppName(epoch.provider, epoch.account_label)} className="ml-10 flex flex-wrap items-center gap-2 pb-1">
      {epoch.resumable && (
        <Button size="sm" variant="secondary" data-testid="epoch-resume-button" aria-keyshortcuts={ariaOf("epoch-resume")} onClick={onResume}>
          Resume
          {key && <span className="text-muted">{key}</span>}
        </Button>
      )}
      {epoch.session_id && (
        <Button size="sm" variant="secondary" data-testid="epoch-switch-from-button" onClick={() => epoch.session_id && onOpenSession(epoch.session_id)}>
          Switch from here…
        </Button>
      )}
      <Button size="sm" variant="secondary" data-testid="epoch-export-button" onClick={onExport}>
        Save handover…
      </Button>
    </div>
  );
}
