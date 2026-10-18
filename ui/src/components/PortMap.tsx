import { useEffect, useState } from "react";
import type { PortInventoryInfo, PortRowInfo } from "../lib/bridge";
import { Button } from "./primitives";
import { errorText } from "../lib/bridge";

/**
 * AJ-E505: the port occupancy map — "users think in ports, so the
 * port is the headline and the PID is evidence" (Nielsen H2). Rows
 * read `port → PID → Instance → Provider · Account · Project ·
 * listener age`; `Not managed by Juggler` rows are visually muted
 * with icon+text (WCAG 1.4.1 — never colour alone) and carry no
 * actions (Apple HIG: don't offer actions that cannot be taken
 * safely).
 */

const CLASS_LABELS: Record<string, string> = {
  "managed-orphan": "Managed orphan",
  "managed-running": "Managed running",
  "re-adopted": "Re-adopted",
  "not-managed": "Not managed by Juggler",
};

function ageText(ms: number | null): string {
  if (ms === null) return "age unknown";
  const secs = Math.max(0, Math.floor(ms / 1000));
  if (secs < 60) return `${secs}s`;
  if (secs < 3_600) return `${Math.floor(secs / 60)}m`;
  if (secs < 86_400) return `${Math.floor(secs / 3_600)}h`;
  return `${Math.floor(secs / 86_400)}d`;
}

function PortRow({ row }: { row: PortRowInfo }) {
  const notManaged = row.class === "not-managed";
  return (
    <li
      className={`flex flex-col gap-[3px] border-t border-hairline py-2 first:border-t-0 ${
        notManaged ? "opacity-60" : ""
      }`}
      data-testid={`port-row-${row.port}`}
    >
      <div className="flex items-center gap-2">
        <span className="font-mono text-[11.5px] font-medium text-ink">:{row.port}</span>
        <span className="font-mono text-[10.5px] text-muted">PID {row.pid}</span>
        <span
          className={`ml-auto inline-flex items-center gap-1 rounded-full border px-1.5 py-[1px] text-[9.5px] ${
            notManaged
              ? "border-hairline text-muted"
              : row.class === "managed-orphan"
                ? "border-accent/50 text-accent"
                : "border-hairline text-ink"
          }`}
          data-testid={`port-class-${row.port}`}
        >
          {/* Icon+text, never colour alone (WCAG 1.4.1). */}
          <span aria-hidden="true">{notManaged ? "○" : row.class === "managed-orphan" ? "⚑" : "●"}</span>
          {CLASS_LABELS[row.class] ?? row.class}
        </span>
      </div>
      <div className="flex items-center gap-2 text-[10px] text-muted">
        {row.provider !== null ? (
          <span className="truncate">
            {row.provider}
            {row.account !== null ? ` · ${row.account}` : ""}
            {row.project !== null ? ` · ${row.project}` : ""}
          </span>
        ) : (
          <span>listener outside Juggler's ledger — display only</span>
        )}
        <span className="ml-auto whitespace-nowrap">
          {ageText(row.listener_age_ms)}
          {row.hint !== null ? ` · ${row.hint}` : ""}
        </span>
      </div>
    </li>
  );
}

export type PortsBridge = {
  load: () => Promise<PortInventoryInfo>;
};

// Dynamic import (the vi.mock TDZ rule): components statically
// value-importing the bridge break test-module hoisting.
export const defaultPortsBridge: PortsBridge = {
  load: () => import("../lib/bridge").then((bridge) => bridge.portInventory()),
};

/**
 * The port inventory list (read-only in this task — kill affordances
 * arrive with the AJ-E507 verified sheet).
 */
export function PortInventoryList(props: { bridge?: PortsBridge }) {
  const bridge = props.bridge ?? defaultPortsBridge;
  const [info, setInfo] = useState<PortInventoryInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    bridge
      .load()
      .then((result) => {
        if (alive) {
          setInfo(result);
          setError(null);
        }
      })
      .catch((cause: unknown) => {
        if (alive) setError(errorText(cause));
      });
    return () => {
      alive = false;
    };
  }, [bridge]);

  if (error !== null) {
    return (
      <p className="text-[10.5px] text-muted" role="alert" data-testid="port-inventory-error">
        Port inventory unavailable: {error}
      </p>
    );
  }
  if (info === null) {
    return (
      <p className="text-[10.5px] text-muted" data-testid="port-inventory-loading">
        Reading the listener table…
      </p>
    );
  }
  if (info.note !== null) {
    return (
      <p className="text-[10.5px] text-muted" data-testid="port-inventory-note">
        {info.note}
      </p>
    );
  }
  if (info.rows.length === 0) {
    return (
      <p className="text-[10.5px] text-muted" data-testid="port-inventory-empty">
        No listeners on this machine.
      </p>
    );
  }
  return (
    <section className="flex flex-col" data-testid="port-inventory">
      <h4 className="text-[10.5px] font-medium text-ink">Port inventory</h4>
      <ul className="mt-1">
        {info.rows.map((row) => (
          <PortRow key={`${row.port}-${row.pid}`} row={row} />
        ))}
      </ul>
    </section>
  );
}

/**
 * The pre-launch collision sheet (Req.4): advisor informs, never
 * blocks — `Launch anyway` always proceeds; `Free this port` routes
 * to the AJ-E507 verified sheet (the Processes tab until it ships).
 */
export function PortWarningSheet(props: {
  warning: { message: string; port: number; pid: number };
  onFreePort: () => void;
  onLaunchAnyway: () => void;
}) {
  return (
    <div
      className="flex flex-col gap-3 rounded-xl border border-accent/40 bg-panel p-4"
      role="alertdialog"
      aria-label="Port already held"
      data-testid="port-warning-sheet"
    >
      <p className="text-[12px] text-ink" data-testid="port-warning-copy">
        {props.warning.message}
      </p>
      <div className="flex items-center gap-2">
        <Button size="sm" onClick={props.onFreePort}>
          Free this port
        </Button>
        <Button size="sm" variant="secondary" onClick={props.onLaunchAnyway}>
          Launch anyway
        </Button>
      </div>
    </div>
  );
}
