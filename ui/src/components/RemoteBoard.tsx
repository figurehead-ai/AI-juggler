/**
 * AJ-E709 — paired Macs' sessions in the rail, moved out of
 * WorkflowRail.tsx. AJ-E2104: other Macs fold after this Mac; "Other
 * Macs" in the View (or a machine's header) unfolds them.
 */
import { useState, type JSX } from "react";
import type { RemoteInstanceInfo, RemoteMachineInfo } from "../lib/bridge";
import { FoldButton, safeId } from "./rail-groups";

/** The search, applied to a peer's rows. */
export function filterRemote(rows: RemoteInstanceInfo[], query: string): RemoteInstanceInfo[] {
  const needle = query.trim().toLowerCase();
  return rows.filter((row) => {
    if (!needle) return true;
    return [row.account_label, row.provider, row.project_name ?? "", row.model ?? ""].some((field) =>
      field.toLowerCase().includes(needle),
    );
  });
}

/** AJ-E709: one paired machine's Instances on the board — the machine is
 * named on every row (text, never colour alone), quota is "unknown" until
 * that machine shares it (D-12: never inferred). */
function RemoteBoardSection({
  machine,
  rows,
  selected,
  onSelect,
}: {
  machine: RemoteMachineInfo;
  rows: RemoteInstanceInfo[];
  selected: { machineId: string; instanceId: string | null } | null;
  onSelect: (instanceId: string) => void;
}): JSX.Element {
  const state =
    machine.freshness === "live"
      ? "● live"
      : machine.freshness === "stale"
        ? "◐ stale · last known"
        : "○ offline · last known";
  return (
    <div className="mt-1 flex flex-col gap-0.5" data-testid="board-machine" data-machine={machine.machine_id}>
      <p className="flex items-baseline gap-1.5 px-3 pb-0.5 pt-1 text-[10px] font-medium uppercase tracking-[0.08em] text-muted">
        <span className="truncate normal-case tracking-normal">{machine.label}</span>
        <span className="shrink-0 font-normal normal-case tracking-normal">{state}</span>
      </p>
      {rows.length === 0 ? (
        <p className="px-3 py-1 text-[11px] text-muted">
          {machine.freshness === "offline" ? "Offline — nothing to show." : "No sessions match."}
        </p>
      ) : (
        rows.map((row) => {
          const active = selected?.machineId === machine.machine_id && selected.instanceId === row.id;
          // AJ-E708: an away machine's rows are its last known state.
          const lastKnown = machine.freshness !== "live";
          const status = (lastKnown ? "was " : "") + row.status.replace(/_/g, " ");
          return (
            <button
              key={row.id}
              type="button"
              onClick={() => onSelect(row.id)}
              className={
                "relative flex w-full min-w-0 flex-col gap-0.5 rounded-lg py-1.5 pl-3 pr-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (active ? "bg-surface" : "hover:bg-surface/60")
              }
              // WCAG 2.5.3: named by its own words ("Work Max on Mac mini …").
              data-testid="board-remote-row"
            >
              {active && (
                <span className="absolute left-0.5 top-1/2 h-4 w-[2px] -translate-y-1/2 rounded-full bg-accent" />
              )}
              <span className="flex min-w-0 items-center gap-1.5">
                <span
                  className={
                    "size-1.5 shrink-0 rounded-full " +
                    (lastKnown
                      ? "bg-faint"
                      : row.status === "running"
                        ? "bg-live"
                        : row.status === "waiting_user"
                          ? "bg-warning"
                          : "bg-faint")
                  }
                  aria-hidden="true"
                />
                <span className="min-w-0 flex-1 truncate text-[12.5px]">
                  {row.account_label}
                  {" "}<span className="sr-only">on</span>
                </span>
                <span className="shrink-0 rounded border border-hairline px-1 text-[9.5px] text-muted">{machine.label}</span>
              </span>
              <span className="flex min-w-0 gap-1 truncate pl-3 text-[10.5px] text-muted">
                <span>{row.provider}</span>
                <span>· {row.model ?? "model unknown"}</span>
                <span>· {status}</span>
                <span>· quota unknown</span>
              </span>
            </button>
          );
        })
      )}
    </div>
  );
}

/** Other Macs, after this Mac: each folded to its header ("Mac mini · 3
 * open") until the user unfolds it, or turns on Other Macs in the View. */
export function OtherMacs({
  machines,
  query,
  unfolded,
  selected,
  onSelect,
}: {
  machines: RemoteMachineInfo[];
  query: string;
  unfolded: boolean;
  selected: { machineId: string; instanceId: string | null } | null;
  onSelect: (machineId: string, instanceId: string) => void;
}): JSX.Element {
  const [opened, setOpened] = useState<ReadonlySet<string>>(new Set());
  return (
    <>
      {machines.map((machine) => {
        const rows = filterRemote(machine.instances, query);
        const open = unfolded !== opened.has(machine.machine_id);
        const id = safeId("rail-mac", machine.machine_id);
        return (
          <div key={machine.machine_id} className="flex flex-col" data-testid="rail-other-mac">
            {!open ? (
              <div className="flex min-w-0 items-center gap-0.5 pt-1.5">
                <FoldButton
                  open={false}
                  label={machine.label}
                  controls={id}
                  onToggle={() => setOpened((current) => new Set([...current, machine.machine_id]))}
                />
                <p className="min-w-0 truncate text-[11.5px] font-semibold text-muted">
                  {machine.label} <span className="font-normal">· {rows.length} open</span>
                </p>
              </div>
            ) : (
              <div id={id}>
                <RemoteBoardSection machine={machine} rows={rows} selected={selected} onSelect={(instanceId) => onSelect(machine.machine_id, instanceId)} />
              </div>
            )}
          </div>
        );
      })}
    </>
  );
}
