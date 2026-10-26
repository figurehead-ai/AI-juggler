import { useEffect, useState, type JSX } from "react";
import { Button, Card, Chip } from "./primitives";
import { providerDisplayName } from "../lib/providers";
import {
  errorText,
  fleetRemoteFocus,
  fleetRemoteStop,
  type RemoteCommandOutcomeInfo,
  type RemoteInstanceInfo,
  type RemoteMachineInfo,
} from "../lib/bridge";

/** A remote selection: the machine, and the Instance on it (or none
 * when the machine itself is selected). */
export type RemoteContext = {
  machine: RemoteMachineInfo;
  instance: RemoteInstanceInfo | null;
  /** AJ-E711: the last remote focus outcome for this Instance, when the
   * palette started one. */
  lastOutcome?: RemoteCommandOutcomeInfo | null;
};

type Tab = "overview" | "usage" | "history" | "quality" | "processes" | "actions";

/**
 * AJ-E705 — the tab bodies for a remote selection, rendered inside the
 * SAME inspector shell (header, tab order, shortcuts) as local
 * selections. Every value names its source machine and when it was
 * measured; a field the machine doesn't send reads
 * "Unavailable — <reason>", never an estimate (D-07). Actions are
 * permission-gated and still need an explicit click; the owning
 * machine re-checks everything.
 */
export function RemoteTabBody({
  tab,
  remote,
  now,
  onSelectInstance,
  onActionDone,
}: {
  tab: Tab;
  remote: RemoteContext;
  now: number;
  onSelectInstance: (instanceId: string) => void;
  onActionDone: () => void;
}): JSX.Element {
  const { machine, instance } = remote;
  if (instance === null) {
    switch (tab) {
      case "overview":
        return <MachineOverview machine={machine} now={now} onSelectInstance={onSelectInstance} />;
      case "processes":
        return <MachineProcesses machine={machine} now={now} />;
      case "actions":
        return (
          <Card className="flex flex-col gap-2 p-4 text-xs" data-testid="remote-machine-actions">
            <p className="leading-relaxed text-muted">
              Pairing, permissions and unpairing for {machine.label} live in Fleet.
            </p>
          </Card>
        );
      default:
        return <Unavailable machine={machine} what={WHAT[tab]} />;
    }
  }
  switch (tab) {
    case "overview":
      return <InstanceOverview machine={machine} instance={instance} now={now} />;
    case "quality":
      return <InstanceQuality machine={machine} instance={instance} now={now} />;
    case "processes":
      return <InstanceProcesses machine={machine} instance={instance} now={now} />;
    case "actions":
      return (
        <InstanceActions
          machine={machine}
          instance={instance}
          onDone={onActionDone}
          initialOutcome={remote.lastOutcome ?? null}
        />
      );
    default:
      return <Unavailable machine={machine} what={WHAT[tab]} />;
  }
}

const WHAT: Record<Tab, string> = {
  overview: "this",
  usage: "usage and quota",
  history: "conversation history",
  quality: "quality signals",
  processes: "its process list",
  actions: "actions",
};

function ago(ms: number, now: number): string {
  const seconds = Math.max(0, Math.round((now - ms) / 1000));
  if (seconds < 60) return `${seconds} s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  return new Date(ms).toLocaleString();
}

/** The freshness line every remote value carries (icon + words). */
export function freshnessText(machine: RemoteMachineInfo, now: number): string {
  if (machine.freshness === "live") {
    return `● Live — from ${machine.label}, updated ${ago(machine.last_heard_ms ?? now, now)}`;
  }
  if (machine.freshness === "stale") {
    return machine.last_heard_ms === null
      ? `◐ Stale — connected to ${machine.label}, nothing received yet`
      : `◐ Stale — last update from ${machine.label} ${ago(machine.last_heard_ms, now)}`;
  }
  return machine.last_heard_ms === null
    ? `○ Offline — ${machine.label} hasn't reported yet`
    : `○ Offline — ${machine.label} last seen ${new Date(machine.last_heard_ms).toLocaleString()}`;
}

function Unavailable({ machine, what }: { machine: RemoteMachineInfo; what: string }): JSX.Element {
  return (
    <Card className="p-4 text-xs" data-testid="remote-unavailable">
      <p className="font-medium">Unavailable — {machine.label} doesn't share {what} over the link yet.</p>
      <p className="mt-1 leading-relaxed text-muted">
        Nothing here is estimated: a value appears only when {machine.label} reports it.
      </p>
    </Card>
  );
}

function Field({
  label,
  value,
  source,
  testId,
}: {
  label: string;
  value: string | null;
  source: string;
  testId?: string;
}): JSX.Element {
  return (
    <div className="flex flex-col gap-0.5 py-1.5" data-testid={testId}>
      <dt className="text-label text-muted">{label}</dt>
      <dd className="text-sm">
        {value ?? <span className="text-muted">Unavailable — not reported</span>}
      </dd>
      {value !== null && <dd className="text-label text-muted">{source}</dd>}
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  return `${Math.round(bytes / 1024 ** 2)} MB`;
}

function InstanceOverview({
  machine,
  instance,
  now,
}: {
  machine: RemoteMachineInfo;
  instance: RemoteInstanceInfo;
  now: number;
}): JSX.Element {
  const source = `from ${machine.label} · measured ${ago(instance.measured_at_ms, now)}`;
  const model =
    instance.model === null ? null : instance.model_source === "launch" ? `${instance.model} (launch setting — not verified live)` : instance.model;
  return (
    <Card className="p-4" data-testid="remote-overview">
      <dl className="grid grid-cols-1 gap-x-6 sm:grid-cols-2">
        <Field label="Machine" value={machine.label} source={`paired · ${machine.connection}`} />
        <Field label="Tool" value={providerDisplayName(instance.provider)} source={source} />
        <Field label="Account" value={instance.account_label} source={source} />
        <Field label="Model" value={model} source={source} testId="remote-model" />
        <Field label="Status" value={instance.status.replace(/_/g, " ")} source={source} />
        <Field label="Project" value={instance.project_name} source={source} />
        <Field
          label="Started"
          value={new Date(instance.started_at_ms).toLocaleString()}
          source={`${machine.label}'s clock`}
        />
        <Field
          label="Last activity"
          value={instance.last_activity_ms === null ? null : new Date(instance.last_activity_ms).toLocaleString()}
          source={`${machine.label}'s clock`}
        />
      </dl>
      {/* AJ-E2108: a session's messages stay on its own Mac. */}
      <p className="mt-3 text-label text-muted" data-testid="last-messages-remote">Last messages: open on {machine.label} to read.</p>
    </Card>
  );
}

function InstanceQuality({
  machine,
  instance,
  now,
}: {
  machine: RemoteMachineInfo;
  instance: RemoteInstanceInfo;
  now: number;
}): JSX.Element {
  const source = `from ${machine.label} · measured ${ago(instance.measured_at_ms, now)}`;
  return (
    <Card className="flex flex-col gap-2 p-4" data-testid="remote-quality">
      <p className="text-label text-muted">Measurable signals only — no composite score.</p>
      <dl>
        <Field
          label="Process health"
          value={
            instance.process_count === null
              ? null
              : `${instance.process_count} process${instance.process_count === 1 ? "" : "es"} running`
          }
          source={source}
        />
        <Field label="Errors and retries" value={null} source={source} />
        <Field label="Stuck / waiting" value={instance.status === "waiting_user" ? "Waiting for you" : null} source={source} />
        <Field label="Context use" value={null} source={source} />
      </dl>
    </Card>
  );
}

function InstanceProcesses({
  machine,
  instance,
  now,
}: {
  machine: RemoteMachineInfo;
  instance: RemoteInstanceInfo;
  now: number;
}): JSX.Element {
  const source = `from ${machine.label} · measured ${ago(instance.measured_at_ms, now)}`;
  return (
    <Card className="flex flex-col gap-2 p-4" data-testid="remote-processes">
      <dl className="grid grid-cols-1 gap-x-6 sm:grid-cols-3">
        <Field
          label="Processes"
          value={instance.process_count === null ? null : String(instance.process_count)}
          source={source}
        />
        <Field
          label="CPU"
          value={instance.cpu_percent === null ? null : `${instance.cpu_percent.toFixed(1)} %`}
          source={source}
        />
        <Field
          label="Memory"
          value={instance.rss_bytes === null ? null : formatBytes(instance.rss_bytes)}
          source={source}
        />
      </dl>
      <InstanceProcessList machine={machine} instanceId={instance.id} />
    </Card>
  );
}

function InstanceActions({
  machine,
  instance,
  onDone,
  initialOutcome,
}: {
  machine: RemoteMachineInfo;
  instance: RemoteInstanceInfo;
  onDone: () => void;
  initialOutcome: RemoteCommandOutcomeInfo | null;
}): JSX.Element {
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<RemoteCommandOutcomeInfo | null>(initialOutcome);
  useEffect(() => {
    if (initialOutcome) setOutcome(initialOutcome);
  }, [initialOutcome]);
  const [error, setError] = useState<string | null>(null);
  const canControl = machine.permission === "control";
  const connected = machine.connection === "connected";
  const running = instance.status === "running" || instance.status === "starting" || instance.status === "waiting_user";
  const reason = !canControl
    ? `View-only pairing — change in Fleet settings on ${machine.label}`
    : !connected
      ? `${machine.label} isn't connected right now`
      : !running
        ? "This session isn't running"
        : null;
  return (
    <Card className="flex flex-col gap-3 p-4" data-testid="remote-actions">
      <div className="flex flex-wrap items-center gap-3">
        <span title={reason ?? undefined} data-testid="remote-stop-wrap">
          <Button
            variant="danger"
            size="sm"
            disabled={reason !== null || busy}
            aria-describedby={reason ? "remote-stop-reason" : undefined}
            onClick={async () => {
              setBusy(true);
              setError(null);
              try {
                const result = await fleetRemoteStop(machine.machine_id, instance.id);
                setOutcome(result);
                if (result.accepted) onDone();
              } catch (cause) {
                setError(errorText(cause));
              } finally {
                setBusy(false);
              }
            }}
            data-testid="remote-stop"
          >
            Stop on {machine.label}
          </Button>
        </span>
        {instance.kind === "desktop" && (
          <span title={reason ?? undefined}>
            <Button
              variant="secondary"
              size="sm"
              disabled={reason !== null || busy}
              aria-describedby={reason ? "remote-stop-reason" : undefined}
              onClick={async () => {
                setBusy(true);
                setError(null);
                try {
                  setOutcome(await fleetRemoteFocus(machine.machine_id, instance.id));
                } catch (cause) {
                  setError(errorText(cause));
                } finally {
                  setBusy(false);
                }
              }}
              data-testid="remote-focus"
            >
              Focus on {machine.label}
            </Button>
          </span>
        )}
        {reason && (
          <span id="remote-stop-reason" className="text-label text-muted" data-testid="remote-stop-reason">
            {reason}
          </span>
        )}
      </div>
      <p className="text-label leading-relaxed text-muted">
        {machine.label} checks the pairing permission, that the request is fresh, and that this
        Session exists before it acts — and records the request.
      </p>
      <div aria-live="polite" data-testid="remote-outcome">
        {outcome && (
          <p className={"text-xs " + (outcome.accepted ? "text-success" : "text-warning")}>
            {outcome.accepted ? "✓ " : "⚠ "}
            {outcome.accepted ? `${machine.label}: ${outcome.message}` : `${machine.label} refused: ${outcome.message}`}
          </p>
        )}
        {error && <p className="text-xs text-danger">{error}</p>}
      </div>
    </Card>
  );
}

function MachineOverview({
  machine,
  now,
  onSelectInstance,
}: {
  machine: RemoteMachineInfo;
  now: number;
  onSelectInstance: (instanceId: string) => void;
}): JSX.Element {
  const resources = machine.resources;
  const source = resources ? `from ${machine.label} · measured ${ago(resources.measured_at_ms, now)}` : "";
  return (
    <div className="flex flex-col gap-3" data-testid="remote-machine-overview">
      <Card className="p-4">
        <dl className="grid grid-cols-1 gap-x-6 sm:grid-cols-3">
          <Field
            label="What this Mac may do there"
            value={
              machine.permission === "control"
                ? "View + control"
                : machine.permission === "view"
                  ? "View only"
                  : machine.permission === "no_access"
                    ? "No access"
                    : null
            }
            source={`set on ${machine.label}`}
          />
          <Field
            label="CPU"
            value={resources ? `${resources.cpu_percent.toFixed(0)} %` : null}
            source={source}
          />
          <Field
            label="Memory"
            value={
              resources
                ? `${formatBytes(resources.memory_used_bytes)} of ${formatBytes(resources.memory_total_bytes)}`
                : null
            }
            source={source}
          />
        </dl>
      </Card>
      <Card className="flex flex-col gap-1 p-4">
        <p className="text-xs font-medium">Sessions on {machine.label}</p>
        {machine.instances.length === 0 ? (
          <p className="text-xs text-muted">
            {machine.freshness === "offline" ? "Unavailable while offline." : "None running."}
          </p>
        ) : (
          <ul className="flex flex-col divide-y divide-hairline">
            {machine.instances.map((row) => (
              <li key={row.id}>
                <button
                  type="button"
                  className="flex w-full items-center gap-2 py-2 text-left text-xs hover:text-ink"
                  onClick={() => onSelectInstance(row.id)}
                  data-testid="remote-machine-instance"
                >
                  <span className="font-medium">{row.account_label}</span>
                  <span className="text-muted">{row.provider}</span>
                  {row.project_name && <span className="text-muted">· {row.project_name}</span>}
                  <span className="flex-1" />
                  <Chip tone="neutral">{row.status.replace(/_/g, " ")}</Chip>
                </button>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </div>
  );
}

function MachineProcesses({ machine, now }: { machine: RemoteMachineInfo; now: number }): JSX.Element {
  const counted = machine.instances.filter((row) => row.process_count !== null);
  return (
    <Card className="flex flex-col gap-2 p-4" data-testid="remote-machine-processes">
      {counted.length === 0 ? (
        <p className="text-xs text-muted">Unavailable — {machine.label} hasn't reported process numbers.</p>
      ) : (
        <ul className="flex flex-col gap-1 text-xs">
          {counted.map((row) => (
            <li key={row.id} className="flex gap-2">
              <span className="font-medium">{row.account_label}</span>
              <span className="text-muted">
                {row.process_count} process{row.process_count === 1 ? "" : "es"}
                {row.cpu_percent !== null ? ` · ${row.cpu_percent.toFixed(1)} % CPU` : ""}
                {row.rss_bytes !== null ? ` · ${formatBytes(row.rss_bytes)}` : ""}
              </span>
              <span className="flex-1" />
              <span className="text-label text-muted">measured {ago(row.measured_at_ms, now)}</span>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

/** AJ-E707: the Instance's processes as its machine reported them;
 * cleaning happens in the Cleanup Centre (All machines), where each
 * verified row has a confirmed Clean. */
function InstanceProcessList({ machine, instanceId }: { machine: RemoteMachineInfo; instanceId: string }): JSX.Element {
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
      <p className="text-label text-muted">Clean verified processes in Cleanup → All machines.</p>
    </div>
  );
}
