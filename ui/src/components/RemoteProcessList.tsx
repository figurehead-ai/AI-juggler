/**
 * AJ-E707: a paired Mac's session's processes as that Mac reported them.
 * Cleaning happens on the Processes page, under "Your other Macs", where
 * each verified row has a confirmed Clean; the line under the list opens
 * it there.
 */
import type { JSX } from "react";
import { Chip } from "./primitives";
import { openScreenAt } from "../lib/nav";
import type { RemoteMachineInfo } from "../lib/bridge";

export function InstanceProcessList({ machine, instanceId }: { machine: RemoteMachineInfo; instanceId: string }): JSX.Element {
  const rows = (machine.processes ?? []).filter((row) => row.instance_id === instanceId);
  if (rows.length === 0) {
    return (
      <p className="text-label text-muted">
        Unavailable — {machine.label} hasn't reported this session's processes.
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-1">
      <ul className="flex flex-col divide-y divide-hairline rounded-lg border border-hairline" data-testid="remote-process-list">
        {rows.map((row) => (
          <li key={row.pid} className="flex flex-wrap items-center gap-2 px-2.5 py-1.5 text-xs">
            <span className="font-mono text-label">{row.name || "process"}</span>
            <span className="text-muted">pid {row.pid}</span>
            <span className="text-muted">· {row.role}</span>
            <span className="text-muted">· {row.state_label}</span>
            {row.ports && <span className="text-muted">· ports {row.ports}</span>}
            <span className="flex-1" />
            <Chip tone={row.verified ? "success" : "neutral"}>
              {row.verified ? "✓ Verified owned" : "Not owned — never touched"}
            </Chip>
          </li>
        ))}
      </ul>
      <p className="text-label text-muted">
        Clean verified processes in{" "}
        <button
          type="button"
          className="rounded text-accent underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent"
          onClick={() => openScreenAt("cleanup", "remote-cleanup")}
          data-testid="remote-clean-link"
        >
          Processes › Your other Macs
        </button>
        .
      </p>
    </div>
  );
}
