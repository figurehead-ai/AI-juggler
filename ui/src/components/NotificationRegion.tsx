/**
 * AJ-E611, AJ-E2504 — the band of banners above a screen, one polite live
 * region, every banner the shared §2 Banner (the approved design of
 * 2026-10-27): an account almost out of room (from the notification
 * model, with its meter, Continue on another account…, Queue for the reset, Snooze 1 h,
 * and a close that leaves it in the centre; at most three), an update,
 * a parked session's reset, a queued resume, this computer's alerts and
 * another computer's. Plain words, real actions; nothing here launches by
 * itself — every action is a click.
 *
 * The alert pass still runs here on its 30 s poll: it feeds the
 * notification model (and the paired consoles); the model's banners are
 * what shows.
 */
import { useContext, useEffect, useState, type JSX } from "react";
import {
  alertsTick,
  fleetRemote,
  instancesList,
  parkedDismiss,
  parkedOffers,
  parkedResume,
  resetQueueDismiss,
  resetQueueList,
  statsEvent,
  updateRestart,
  updateStatus,
  onUpdateStatus,
  type AccountInfo,
  type InstanceInfo,
  type ParkedOfferInfo,
  type RemoteMachineInfo,
  type ResetQueueRow,
  type UpdateStatusInfo,
} from "../lib/bridge";
import { Banner } from "./Banner";
import { updateErrorText } from "./UpdatesCard";
import { ChannelChip } from "./ChannelChip";
import { alertWords, keptNotices, machineAlertsTake, ThisMacContext, type MachineNotice } from "../lib/machine";
import { machineNotices, publishMachineNotices, subscribeMachineNotices } from "../lib/machine-notices";
import { usePagedList } from "../lib/paged";
import { notificationsDismiss, notificationsPage, type NoticeInfo } from "../lib/ipc/notices";
import { acted } from "../lib/notices-store";
import { useNoticeActions } from "../lib/notice-actions";
import { quotaDetail } from "../lib/notice-words";
import { whenWords } from "../lib/time-words";

/** AJ-E1103: an alert kind as the schema names it. */
function alertKind(kind: string): string {
  if (kind === "tray-amber" || kind === "actionable") return "quota_warning";
  if (kind === "exhausted") return "quota_wall";
  return "other";
}

function notification(kind: string, action: "shown" | "clicked" | "dismissed"): void {
  statsEvent("ui.notification", { kind, action });
}

function seen(key: string): string {
  try {
    return localStorage.getItem(key) ?? "";
  } catch {
    return ""; /* storage unavailable: show it */
  }
}

function remember(key: string, version: string): void {
  try {
    localStorage.setItem(key, version);
  } catch {
    /* best effort */
  }
}

/** AJ-E1010: an update announced by the launch check — one notice per
 * version, never an install — and, once installed, "Restart to update",
 * offered only while it is safe, never forced. */
function useUpdateNotices() {
  const [update, setUpdate] = useState<{ version: string; channel: string } | null>(null);
  const [ready, setReady] = useState<{ version: string; channel: string } | null>(null);
  useEffect(() => {
    const apply = (status: UpdateStatusInfo) => {
      const version = status.available?.version ?? null;
      if (status.state === "available" && version && seen("aij.update.noticed") !== version) {
        notification("update", "shown");
        setUpdate({ version, channel: status.channel });
      }
      if (status.state === "installed" && version && !status.restart_wait && seen("aij.update.restart.later") !== version) {
        setUpdate(null);
        setReady({ version, channel: status.channel });
      } else setReady(null);
    };
    let alive = true;
    void updateStatus().then((status) => alive && apply(status), () => null);
    const stop = onUpdateStatus(apply);
    return () => {
      alive = false;
      stop();
    };
  }, []);
  return { update, ready, closeUpdate: (version: string) => (remember("aij.update.noticed", version), setUpdate(null)), later: (version: string) => (remember("aij.update.restart.later", version), setReady(null)) };
}

/** The 30 s pass: the alert pass (into the model), this computer's alerts
 * drained once, the reset queue and parked offers. */
function usePoll(setQueue: (rows: ResetQueueRow[]) => void, setOffers: (rows: ParkedOfferInfo[]) => void): void {
  useEffect(() => {
    let alive = true;
    const poll = () => {
      machineAlertsTake()
        .then(async (fresh) => {
          if (fresh.length === 0) return;
          const sessions = await instancesList().catch(() => []);
          const nameOf = (id: string) => {
            const row = sessions.find((s) => s.id === id);
            return row ? (row.git_branch ?? ((row.cwd ?? "").replace(/\/+$/, "").split("/").pop() || row.account_label)) : null;
          };
          fresh.forEach(() => notification("other", "shown"));
          // Kept outside the component: a drained alert survives a remount.
          const toldAt = Date.now();
          const told = [...fresh].reverse().map((alert) => ({ key: `${alert.key}-${alert.since_ms}`, alertKey: alert.key, kind: alert.kind, text: (({ title, detail }) => `${title} — ${detail}`)(alertWords([alert], nameOf)!), toldAt }));
          publishMachineNotices(keptNotices([...told, ...machineNotices()], null, toldAt).slice(0, 3));
        })
        .catch(() => null);
      alertsTick().then((emitted) => emitted.forEach((event) => notification(alertKind(event.kind), "shown")), () => null);
      resetQueueList().then((rows) => alive && setQueue(rows), () => null);
      // AJ-E1003: parked sessions whose quota reset, on the same poll.
      parkedOffers().then((rows) => alive && setOffers(rows), () => null);
    };
    poll();
    const timer = window.setInterval(poll, 30_000); // ≥30 s — never a busy loop
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [setQueue, setOffers]);
}

export function NotificationRegion({
  onQueueResume,
  onOpenRemote,
  hideMachineAlerts = false,
  sessions = [],
  accounts = [],
}: {
  onQueueResume?: (row: ResetQueueRow) => void;
  /** AJ-E709: open the machine a Fleet alert came from. */
  onOpenRemote?: (machineId: string) => void;
  /** Home lists this computer's alerts and the accounts almost out of
   * room in Needs you: the banners keep them for the other screens. */
  hideMachineAlerts?: boolean;
  sessions?: InstanceInfo[];
  accounts?: AccountInfo[];
}): JSX.Element {
  const [queue, setQueue] = useState<ResetQueueRow[]>([]);
  const [remote, setRemote] = useState<RemoteMachineInfo[]>([]);
  const [dismissed, setDismissed] = useState<Set<string>>(() => new Set());
  const [offers, setOffers] = useState<ParkedOfferInfo[]>([]);
  const [restartError, setRestartError] = useState<string | null>(null);
  // AJ-E1408: overload alerts — told once, dismissable, passive only.
  const [machineAlerts, setMachineAlertsState] = useState<MachineNotice[]>(machineNotices);
  const thisMac = useContext(ThisMacContext);
  const updates = useUpdateNotices();
  const actions = useNoticeActions(sessions);
  const banners = usePagedList<NoticeInfo>("notices:banners", (request) => notificationsPage("banners", request), { keyOf: (row) => String(row.id) });
  const [now] = useState(() => Date.now());
  // A notice whose condition has ended goes (45 s grace included).
  useEffect(() => {
    const told = machineNotices();
    if (!thisMac || told.length === 0) return;
    const active = new Set(thisMac.alerts.map((alert) => alert.key));
    const kept = keptNotices(told, active, Date.now());
    if (kept.length !== told.length) publishMachineNotices(kept);
  }, [thisMac]);
  useEffect(() => subscribeMachineNotices(setMachineAlertsState), []);
  // AJ-E709 Req.3: Fleet alerts — a local read of what peers reported.
  useEffect(() => {
    let alive = true;
    const poll = () => void fleetRemote().then((machines) => alive && setRemote(machines), () => null);
    poll();
    const timer = window.setInterval(poll, 2_000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, []);
  usePoll(setQueue, setOffers);
  const remoteAlerts = remote.flatMap((machine) =>
    (machine.alerts ?? [])
      .map((alert) => ({ machine, alert, key: `${machine.machine_id}-${alert.kind}-${alert.at_ms}-${alert.account}` }))
      .filter((entry) => !dismissed.has(entry.key))
      .slice(0, 3),
  );
  const accountName = (id: string) => accounts.find((account) => account.id === id || account.label === id)?.label ?? "its account";
  const { update, ready } = updates;
  return (
    <div aria-live="polite" role="status" data-testid="notification-region" className="flex flex-col gap-2 [&:not(:empty)]:mb-3">
      {!hideMachineAlerts &&
        banners.items.map((notice) => (
          <Banner
            key={notice.id}
            tone="warning"
            title={notice.title}
            detail={quotaDetail(notice, now)}
            meter={notice.meter !== null ? { used: 100 - notice.meter, label: `${100 - notice.meter}% used` } : undefined}
            actions={[
              // F16: the switch for its open session, else (named so) its account.
              { label: actions.canHandOver(notice) ? "Continue on another account…" : "Open its account", primary: true, onClick: () => actions.primary(notice), testId: "quota-banner-hand-over" },
              { label: "Queue for the reset", onClick: () => actions.second(notice, "queue-reset"), testId: "quota-banner-queue" },
              { label: "Snooze 1 h", quiet: true, onClick: () => actions.snooze(notice, "hour") },
            ]}
            onDismiss={() => void acted(() => notificationsDismiss(notice.id))}
            dismissLabel={`Close: ${notice.title} (it stays in Notifications)`}
            testId="quota-banner"
          />
        ))}
      {!hideMachineAlerts &&
        machineAlerts.map((alert) => (
          <Banner key={alert.key} tone="warning" title={`${alert.text}.`} onDismiss={() => (notification("other", "dismissed"), publishMachineNotices(machineNotices().filter((entry) => entry.key !== alert.key)))} testId="machine-alert" />
        ))}
      {remoteAlerts.map(({ machine, alert, key }) => (
        <Banner
          key={key}
          tone={alert.kind === "exhausted" || alert.kind === "actionable" || alert.kind === "peer-offline" ? "warning" : "neutral"}
          title={`${machine.label} · ${alert.text}`}
          actions={onOpenRemote ? [{ label: "Open", onClick: () => (notification("fleet", "clicked"), onOpenRemote(machine.machine_id)) }] : []}
          onDismiss={() => (notification("fleet", "dismissed"), setDismissed((prior) => new Set(prior).add(key)))}
          testId={`remote-alert-${alert.kind}`}
        />
      ))}
      {update && (
        <Banner
          tone="accent"
          title={`AI Juggler ${update.version} is available`}
          detail="Nothing is installed until you choose."
          aside={<ChannelChip channel={update.channel} testId="update-notice-channel" />}
          actions={[
            { label: "Review", primary: true, testId: "update-notice-review", onClick: () => (notification("update", "clicked"), window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "updates" } })), updates.closeUpdate(update.version)) },
            { label: "Not now", quiet: true, testId: "update-notice-later", onClick: () => (notification("update", "dismissed"), statsEvent("update.flow", { step: "deferred", reason: "none" }), updates.closeUpdate(update.version)) },
          ]}
          testId="update-notice"
        />
      )}
      {ready && (
        <Banner
          tone="accent"
          title={`AI Juggler ${ready.version} is installed`}
          detail="Restart when you're ready — nothing restarts by itself."
          aside={<ChannelChip channel={ready.channel} testId="update-ready-channel" />}
          actions={[
            { label: "Restart to update", primary: true, testId: "update-ready-restart", onClick: () => (setRestartError(null), void updateRestart().catch((cause: unknown) => setRestartError(updateErrorText(cause)))) },
            { label: "Later", quiet: true, testId: "update-ready-later", onClick: () => updates.later(ready.version) },
          ]}
          testId="update-ready"
        >
          {restartError && (
            <p role="alert" className="mt-1 text-meta text-danger" data-testid="update-ready-error">
              {restartError}
            </p>
          )}
        </Banner>
      )}
      {offers.map((offer) => (
        <Banner
          key={`parked-${offer.session_id}`}
          tone="accent"
          title={offer.text}
          actions={[
            { label: "Resume", primary: true, testId: "parked-offer-resume", onClick: () => void parkedResume(offer.session_id).then(() => setOffers(offers.filter((row) => row.session_id !== offer.session_id))) },
            { label: offer.last ? "Stop reminding me" : "Remind me in 30 min", quiet: true, testId: offer.last ? "parked-offer-stop" : "parked-offer-snooze", onClick: () => void parkedDismiss(offer.session_id).then(() => setOffers(offers.filter((row) => row.session_id !== offer.session_id))) },
          ]}
          testId="parked-offer"
        />
      ))}
      {queue.map((row) => (
        <Banner
          key={row.id}
          tone="neutral"
          title={row.state === "ready" ? `${accountName(row.account)} has room again — ready to resume` : row.state === "stale" ? "A queued resume can't go ahead: its session is gone" : `Queued for ${accountName(row.account)}'s reset at ${whenWords(row.resume_at_ms)}`}
          actions={row.state === "ready" && onQueueResume ? [{ label: "Resume", primary: true, onClick: () => onQueueResume(row) }] : []}
          onDismiss={() => void resetQueueDismiss(row.id).then(() => setQueue(queue.filter((entry) => entry.id !== row.id)))}
          testId={`queue-${row.state}`}
        />
      ))}
    </div>
  );
}
