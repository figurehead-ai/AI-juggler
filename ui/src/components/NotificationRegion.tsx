/**
 * AJ-E611, AJ-E2504 — the band of banners above a screen, one polite live
 * region, every banner the shared §2 Banner: only what concerns the whole
 * app — this computer's alerts, another computer's, an update ready or
 * installed. Plain words, real actions; nothing here launches by itself.
 *
 * D-87 (2026-11-06; the maintainer's finding from 0.3, again in 0.4): no
 * account's or session's notice is ever here. A band over one session's
 * page about another session's account was nearly clicked for the page's
 * own session. Those are in the bell and the notification centre; on a
 * session's page its own account's notice, its due reminder and its
 * parked offer show inline under its header (`SessionNotices`).
 *
 * 0.3.2 (the maintainer's 0.3.0 walk: stacked banners pushed the page
 * down and cut it off): one banner shows, the most pressing, with "N
 * more" opening the rest in place.
 *
 * The alert pass still runs here on its 30 s poll: it feeds the
 * notification model (and the paired consoles).
 */
import { useContext, useEffect, useState, type JSX } from "react";
import { alertsTick } from "../lib/ipc/alerts";
import { fleetRemote } from "../lib/ipc/fleet-remote";
import { instancesList } from "../lib/ipc/instances";
import { statsEvent } from "../lib/ipc/usage-stats";
import { updateRestart, updateStatus, onUpdateStatus } from "../lib/ipc/updates";
import type { RemoteMachineInfo } from "../types/generated/RemoteMachineInfo";
import type { UpdateStatusInfo } from "../types/generated/UpdateStatusInfo";
import { Banner } from "./Banner";
import { updateErrorText } from "./UpdatesCard";
import { ChannelChip } from "./ChannelChip";
import { alertWords, keptNotices, ThisMacContext, type MachineNotice } from "../lib/machine";
import { machineAlertsTake } from "../lib/ipc/this-mac";
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

/** The 30 s pass: the alert pass (into the model) and this computer's
 * alerts drained once. (D-87: a reminder that's due and a parked
 * session's offer are on that session's own page, `SessionNotices`.) */
function usePoll(): void {
  useEffect(() => {
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
    };
    poll();
    const timer = window.setInterval(poll, 30_000); // ≥30 s — never a busy loop
    return () => window.clearInterval(timer);
  }, []);
}

export function NotificationRegion({
  onOpenRemote,
  hideMachineAlerts = false,
}: {
  /** AJ-E709: open the machine a Fleet alert came from. */
  onOpenRemote?: (machineId: string) => void;
  /** Home lists this computer's alerts and the accounts almost out of
   * room in Needs you: the banners keep them for the other screens. */
  hideMachineAlerts?: boolean;
}): JSX.Element {
  const [remote, setRemote] = useState<RemoteMachineInfo[]>([]);
  const [dismissed, setDismissed] = useState<Set<string>>(() => new Set());
  const [restartError, setRestartError] = useState<string | null>(null);
  const [more, setMore] = useState(false);
  // AJ-E1408: overload alerts — told once, dismissable, passive only.
  const [machineAlerts, setMachineAlertsState] = useState<MachineNotice[]>(machineNotices);
  const thisMac = useContext(ThisMacContext);
  const updates = useUpdateNotices();
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
  usePoll();
  const remoteAlerts = remote.flatMap((machine) =>
    (machine.alerts ?? [])
      .map((alert) => ({ machine, alert, key: `${machine.machine_id}-${alert.kind}-${alert.at_ms}-${alert.account}` }))
      .filter((entry) => !dismissed.has(entry.key))
      .slice(0, 3),
  );
  const { update, ready } = updates;
  // The band's banners, the most pressing first: what stops work, then
  // what's due, then news.
  const items: Array<{ key: string; node: JSX.Element }> = [];
  if (!hideMachineAlerts)
    for (const alert of machineAlerts)
      items.push({
        key: `machine-${alert.key}`,
        node: <Banner key={alert.key} tone="warning" title={`${alert.text}.`} onDismiss={() => (notification("other", "dismissed"), publishMachineNotices(machineNotices().filter((entry) => entry.key !== alert.key)))} testId="machine-alert" />,
      });
  for (const { machine, alert, key } of remoteAlerts)
    items.push({
      key,
      node: (
        <Banner
          key={key}
          tone={alert.kind === "exhausted" || alert.kind === "actionable" || alert.kind === "peer-offline" ? "warning" : "neutral"}
          title={`${machine.label} · ${alert.text}`}
          actions={onOpenRemote ? [{ label: "Open", onClick: () => (notification("fleet", "clicked"), onOpenRemote(machine.machine_id)) }] : []}
          onDismiss={() => (notification("fleet", "dismissed"), setDismissed((prior) => new Set(prior).add(key)))}
          testId={`remote-alert-${alert.kind}`}
        />
      ),
    });
  if (update)
    items.push({
      key: "update",
      node: (
        <Banner
          key="update"
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
      ),
    });
  if (ready)
    items.push({
      key: "ready",
      node: (
        <Banner
          key="ready"
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
      ),
    });
  const rest = items.length - 1;
  return (
    <div aria-live="polite" role="status" data-testid="notification-region" className="flex shrink-0 flex-col gap-2 [&:not(:empty)]:mb-3">
      {(more ? items : items.slice(0, 1)).map((item) => item.node)}
      {rest > 0 && (
        <button type="button" onClick={() => setMore((open) => !open)} aria-expanded={more} data-testid="banners-more" className="-mt-1 self-end rounded-md px-2 py-0.5 text-meta text-muted hover:bg-surface-2 hover:text-ink">
          {more ? "Show fewer" : rest === 1 ? "1 more notice" : `${rest} more notices`}
        </button>
      )}
    </div>
  );
}
