/**
 * AJ-E611 — the alert/notification region: polite live-region
 * semantics (never assertive — anti-fatigue), badge-carrying copy
 * (Estimated reads differently), and the reset-queue rows with their
 * countdown / ready states + an explicit Resume click (the click is
 * the ONLY action; nothing here launches by itself).
 */
import { useContext, useEffect, useState, type JSX } from "react";
import {
  alertsTick,
  instancesList,
  fleetRemote,
  parkedDismiss,
  parkedOffers,
  parkedResume,
  type ParkedOfferInfo,
  resetQueueDismiss,
  resetQueueList,
  type AlertEventData,
  type RemoteMachineInfo,
  type ResetQueueRow,
  onUpdateStatus,
  statsEvent,
  updateRestart,
  updateStatus,
  type UpdateStatusInfo,
} from "../lib/bridge";
import { ChannelChip, updateErrorText } from "./UpdatesCard";
import { alertWords, keptNotices, machineAlertsTake, ThisMacContext, type MachineNotice } from "../lib/machine";
import { machineNotices, publishMachineNotices, subscribeMachineNotices } from "../lib/machine-notices";

/** AJ-E1103: an alert kind as the schema names it. */
function alertKind(kind: string): string {
  if (kind === "tray-amber" || kind === "actionable") return "quota_warning";
  if (kind === "exhausted") return "quota_wall";
  return "other";
}

function notification(kind: string, action: "shown" | "clicked" | "dismissed"): void {
  statsEvent("ui.notification", { kind, action });
}

export function NotificationRegion({
  onQueueResume,
  onOpenRemote,
  hideMachineAlerts = false,
}: {
  onQueueResume?: (row: ResetQueueRow) => void;
  /** AJ-E709: open the machine a Fleet alert came from. */
  onOpenRemote?: (machineId: string) => void;
  /** Home shows the Mac's alerts once, in Needs you: the banner keeps
   * them for the other screens (the 0.0.12 report). */
  hideMachineAlerts?: boolean;
}): JSX.Element {
  const [events, setEvents] = useState<AlertEventData[]>([]);
  const [queue, setQueue] = useState<ResetQueueRow[]>([]);
  const [remote, setRemote] = useState<RemoteMachineInfo[]>([]);
  const [dismissed, setDismissed] = useState<Set<string>>(() => new Set());
  const [offers, setOffers] = useState<ParkedOfferInfo[]>([]);
  // AJ-E1408: overload alerts — told once, dismissable, never an action
  // on anything (passive only).
  const [machineAlerts, setMachineAlertsState] = useState<MachineNotice[]>(machineNotices);
  // A notice whose condition has ended goes (This Mac's alerts are the
  // ones still on, 45 s grace included).
  const thisMac = useContext(ThisMacContext);
  useEffect(() => {
    const now = machineNotices();
    if (!thisMac || now.length === 0) return;
    const active = new Set(thisMac.alerts.map((alert) => alert.key));
    const kept = keptNotices(now, active, Date.now());
    if (kept.length !== now.length) publishMachineNotices(kept);
  }, [thisMac]);
  useEffect(() => {
    const stop = subscribeMachineNotices(setMachineAlertsState);
    return () => {
      stop();
    };
  }, []);
  // AJ-E1010: an update announced by the launch check — one notice per
  // version, never an install. Once one is installed, "Restart to update"
  // — offered only while it is safe (the backend holds it back during a
  // switch, a bring-in or live sessions), never forced.
  const [update, setUpdate] = useState<{ version: string; channel: string } | null>(null);
  const [ready, setReady] = useState<{ version: string; channel: string } | null>(null);
  const [restartError, setRestartError] = useState<string | null>(null);
  useEffect(() => {
    const seen = (key: string): string => {
      try {
        return localStorage.getItem(key) ?? "";
      } catch {
        return ""; /* storage unavailable: show it */
      }
    };
    const apply = (status: UpdateStatusInfo) => {
      const version = status.available?.version ?? null;
      if (status.state === "available" && version && seen("aij.update.noticed") !== version) {
        notification("update", "shown");
        setUpdate({ version, channel: status.channel });
      }
      if (status.state === "installed" && version && !status.restart_wait && seen("aij.update.restart.later") !== version) {
        setUpdate(null);
        setReady({ version, channel: status.channel });
      } else {
        setReady(null);
      }
    };
    let alive = true;
    void updateStatus()
      .then((status) => {
        if (alive) apply(status);
      })
      .catch(() => null);
    const stop = onUpdateStatus(apply);
    return () => {
      alive = false;
      stop();
    };
  }, []);
  const closeUpdate = (version: string) => {
    try {
      localStorage.setItem("aij.update.noticed", version);
    } catch {
      /* best effort */
    }
    setUpdate(null);
  };
  const later = (version: string) => {
    try {
      localStorage.setItem("aij.update.restart.later", version);
    } catch {
      /* best effort */
    }
    setReady(null);
  };

  // AJ-E709 Req.3: Fleet alerts — a local read of what peers reported
  // (the link delivers within the second; this only paints it).
  useEffect(() => {
    let alive = true;
    const poll = () => {
      fleetRemote()
        .then((machines) => {
          if (alive) setRemote(machines);
        })
        .catch(() => null);
    };
    poll();
    const timer = window.setInterval(poll, 2_000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);
  const remoteAlerts = remote.flatMap((machine) =>
    (machine.alerts ?? [])
      .map((alert) => ({ machine, alert, key: `${machine.machine_id}-${alert.kind}-${alert.at_ms}-${alert.account}` }))
      .filter((entry) => !dismissed.has(entry.key))
      .slice(0, 3),
  );

  useEffect(() => {
    let alive = true;
    const poll = () => {
      machineAlertsTake()
        .then(async (fresh) => {
          if (fresh.length === 0) return;
          const sessions = await instancesList().catch(() => []);
          const nameOf = (id: string) => {
            const row = sessions.find((s) => s.id === id);
            if (!row) return null;
            return row.git_branch ?? ((row.cwd ?? "").replace(/\/+$/, "").split("/").pop() || row.account_label);
          };
          // Counted as "other": the usage-stats schema has no machine kind.
          fresh.forEach(() => notification("other", "shown"));
          // Kept outside the component: a drained alert survives a
          // remount (switching screens) until it is dismissed.
          const toldAt = Date.now();
          publishMachineNotices(
            keptNotices(
              [
                ...[...fresh].reverse().map((alert) => ({
                  key: `${alert.key}-${alert.since_ms}`,
                  alertKey: alert.key,
                  kind: alert.kind,
                  text: (({ title, detail }) => `${title} — ${detail}`)(alertWords([alert], nameOf)!),
                  toldAt,
                })),
                ...machineNotices(),
              ],
              null,
              toldAt,
            ).slice(0, 3),
          );
        })
        .catch(() => null);
      alertsTick().then((emitted) => {
        for (const event of emitted) notification(alertKind(event.kind), "shown");
        if (alive && emitted.length > 0) setEvents((prior) => [...emitted, ...prior].slice(0, 5));
      });
      resetQueueList().then((rows) => {
        if (alive) setQueue(rows);
      });
      // AJ-E1003: parked Instances whose quota reset — with the same
      // poll as the alerts (no timer of its own).
      parkedOffers()
        .then((rows) => {
          if (alive) setOffers(rows);
        })
        .catch(() => null);
    };
    poll();
    const timer = window.setInterval(poll, 30_000); // ≥30 s — never a busy loop
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);

  return (
    <div aria-live="polite" role="status" data-testid="notification-region" className="flex flex-col gap-1">
      {events.map((event) => (
        <p
          key={`${event.at_ms}-${event.account}-${event.kind}`}
          data-testid={`alert-${event.kind}`}
          className={`rounded border px-2 py-1 text-label ${
            event.kind === "exhausted" || event.kind === "actionable"
              ? "border-warning/50 text-warning"
              : event.kind === "digest"
                ? "border-dashed border-hairline text-muted"
                : "border-hairline text-muted"
          }`}
        >
          {event.kind === "digest" ? "Digest · " : ""}
          {event.text}
        </p>
      ))}
      {!hideMachineAlerts && machineAlerts.map((alert) => (
        <p
          key={alert.key}
          data-testid="machine-alert"
          className="flex flex-wrap items-center gap-2 rounded border border-warning/50 px-2 py-1 text-label text-warning"
        >
          <span className="min-w-0 flex-1">{alert.text}.</span>
          <button
            type="button"
            className="text-muted underline focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() => {
              notification("other", "dismissed");
              publishMachineNotices(machineNotices().filter((entry) => entry.key !== alert.key));
            }}
          >
            dismiss
          </button>
        </p>
      ))}
      {remoteAlerts.map(({ machine, alert, key }) => (
        <p
          key={key}
          data-testid={`remote-alert-${alert.kind}`}
          className={`flex flex-wrap items-center gap-2 rounded border px-2 py-1 text-label ${
            alert.kind === "exhausted" || alert.kind === "actionable" || alert.kind === "peer-offline"
              ? "border-warning/50 text-warning"
              : "border-hairline text-muted"
          }`}
        >
          <span className="font-medium">{machine.label}</span>
          <span>·</span>
          <span className="min-w-0 flex-1">{alert.text}</span>
          {onOpenRemote && (
            <button
              type="button"
              className="underline focus:outline-none focus:ring-2 focus:ring-accent"
              onClick={() => {
                notification("fleet", "clicked");
                onOpenRemote(machine.machine_id);
              }}
            >
              Open
            </button>
          )}
          <button
            type="button"
            className="text-muted underline focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() => {
              notification("fleet", "dismissed");
              setDismissed((prior) => new Set(prior).add(key));
            }}
          >
            dismiss
          </button>
        </p>
      ))}
      {update && (
        <div
          data-testid="update-notice"
          className="flex flex-wrap items-center gap-2 rounded border border-accent/40 bg-accent-soft px-2 py-1.5 text-label text-ink"
        >
          <span className="min-w-0 flex-1">
            AI Juggler {update.version} is available. Nothing is installed until you choose.
          </span>
          <ChannelChip channel={update.channel} testId="update-notice-channel" />
          <button
            type="button"
            className="rounded border border-accent/50 px-2 py-0.5 font-medium text-accent-hi focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() => {
              notification("update", "clicked");
              window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "updates" } }));
              closeUpdate(update.version);
            }}
            data-testid="update-notice-review"
          >
            Review
          </button>
          <button
            type="button"
            className="text-ink underline focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() => {
              notification("update", "dismissed");
              statsEvent("update.flow", { step: "deferred", reason: "none" });
              closeUpdate(update.version);
            }}
            data-testid="update-notice-later"
          >
            Not now
          </button>
        </div>
      )}
      {ready && (
        <div
          data-testid="update-ready"
          className="flex flex-wrap items-center gap-2 rounded border border-accent/40 bg-accent-soft px-2 py-1.5 text-label text-ink"
        >
          <span className="min-w-0 flex-1">
            AI Juggler {ready.version} is installed. Restart when you're ready — nothing restarts by itself.
          </span>
          <ChannelChip channel={ready.channel} testId="update-ready-channel" />
          <button
            type="button"
            className="rounded border border-accent/50 px-2 py-0.5 font-medium text-accent-hi focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() => {
              setRestartError(null);
              updateRestart().catch((cause: unknown) => setRestartError(updateErrorText(cause)));
            }}
            data-testid="update-ready-restart"
          >
            Restart to update
          </button>
          <button
            type="button"
            className="text-ink underline focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() => later(ready.version)}
            data-testid="update-ready-later"
          >
            Later
          </button>
          {restartError && (
            <p role="alert" className="basis-full" data-testid="update-ready-error">
              {restartError}
            </p>
          )}
        </div>
      )}
      {offers.map((offer) => (
        <div
          key={`parked-${offer.session_id}`}
          data-testid="parked-offer"
          className="flex flex-wrap items-center gap-2 rounded border border-accent/40 bg-accent-soft px-2 py-1.5 text-label text-ink"
        >
          <span className="min-w-0 flex-1">{offer.text}</span>
          <button
            type="button"
            className="rounded border border-accent/50 px-2 py-0.5 font-medium text-accent-hi focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() =>
              void parkedResume(offer.session_id).then(() =>
                setOffers((rows) => rows.filter((row) => row.session_id !== offer.session_id)),
              )
            }
            data-testid="parked-offer-resume"
          >
            Resume
          </button>
          <button
            type="button"
            className="text-ink underline focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() =>
              void parkedDismiss(offer.session_id).then(() =>
                setOffers((rows) => rows.filter((row) => row.session_id !== offer.session_id)),
              )
            }
            data-testid={offer.last ? "parked-offer-stop" : "parked-offer-snooze"}
          >
            {offer.last ? "Stop reminding me" : "Remind me in 30 min"}
          </button>
        </div>
      ))}
      {queue.map((row) => (
        <p key={row.id} data-testid={`queue-${row.state}`} className="flex items-center gap-2 rounded border border-hairline px-2 py-1 text-label">
          <span>
            {row.state === "ready"
              ? `reset reached — ${row.account} ready to resume`
              : row.state === "stale"
                ? `queued resume went stale (session ${row.session} is gone)`
                : `queued at ${new Date(row.resume_at_ms).toLocaleTimeString()} — ${row.account}`}
          </span>
          {row.state === "ready" && onQueueResume && (
            <button
              type="button"
              className="underline focus:outline-none focus:ring-2 focus:ring-accent"
              onClick={() => onQueueResume(row)}
            >
              Resume
            </button>
          )}
          <button
            type="button"
            className="text-muted underline focus:outline-none focus:ring-2 focus:ring-accent"
            onClick={() => void resetQueueDismiss(row.id).then(() => setQueue((rows) => rows.filter((entry) => entry.id !== row.id)))}
          >
            dismiss
          </button>
        </p>
      ))}
    </div>
  );
}
