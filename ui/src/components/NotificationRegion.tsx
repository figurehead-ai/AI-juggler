/**
 * AJ-E611, AJ-E2504 — the band of banners above a screen, one polite live
 * region, every banner the shared §2 Banner (the approved design of
 * 2026-10-27): an account almost out of room (from the notification
 * model, with its meter, Continue on another account…, Remind me at the
 * reset, Snooze 1 h, and a close that leaves it in the centre), a
 * reminder that's due, an update, a parked session's reset, this
 * computer's alerts and another computer's. Plain words, real actions;
 * nothing here launches by itself — every action is a click.
 *
 * 0.3.2 (the maintainer's 0.3.0 walk: stacked banners pushed the page
 * down and cut it off, and a close took about a minute): one banner shows,
 * the most pressing, with "N more" opening the rest in place; a close or
 * a snooze hides it at once and the backend catches up (it comes back,
 * with the reason, if the backend refuses); a reminder shows nothing while
 * it waits, only when it's due.
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
import { quotaDetail, secondOf } from "../lib/notice-words";
import { REMINDER_CONVERSATION } from "../lib/notice-actions";
import { useActionHost } from "./ItemMenu";
import { useConversations } from "../lib/conversations";

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
  onOpenRemote,
  hideMachineAlerts = false,
  sessions = [],
  accounts = [],
}: {
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
  // Closed or snoozed here, not yet confirmed by the backend (0.3.2).
  const [hidden, setHidden] = useState<ReadonlySet<string>>(() => new Set());
  const [more, setMore] = useState(false);
  const host = useActionHost();
  const { all: conversations } = useConversations();
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
  const accountName = (id: string) => accounts.find((account) => account.id === id || account.label === id)?.label ?? "Its account";
  const { update, ready } = updates;
  /** Hides it now; the write follows, and a refusal puts it back. */
  const hideWhile = (key: string, write: () => Promise<unknown>) => {
    setHidden((prior) => new Set(prior).add(key));
    void write().then(
      (done) => done === false && setHidden((prior) => without(prior, key)),
      () => setHidden((prior) => without(prior, key)),
    );
  };
  const reminderOf = (row: ResetQueueRow) => {
    if (row.session.startsWith(REMINDER_CONVERSATION)) {
      const conversation = conversations.find((c) => c.id === row.session.slice(REMINDER_CONVERSATION.length));
      return { name: conversation ? (conversation.title ?? conversation.git_branch ?? folderOf(conversation.cwd)) : null, open: conversation ? () => host.openConversation?.(conversation) : null };
    }
    const session = sessions.find((s) => s.id === row.session);
    return { name: session ? (session.git_branch ?? folderOf(session.cwd)) : null, open: session ? () => host.openSessionById?.(session.id) : null };
  };
  // The band's banners, the most pressing first: what stops work, then
  // what's due, then news.
  const items: Array<{ key: string; node: JSX.Element }> = [];
  if (!hideMachineAlerts)
    for (const notice of banners.items) {
      const key = `notice-${notice.id}`;
      if (hidden.has(key)) continue;
      const second = secondOf(notice);
      items.push({
        key,
        node: (
          <Banner
            key={key}
            tone="warning"
            title={notice.title}
            detail={quotaDetail(notice, now)}
            meter={notice.meter !== null ? { used: 100 - notice.meter, label: `${100 - notice.meter}% used` } : undefined}
            actions={[
              // F16: the switch for its open session, else (named so) its account.
              { label: actions.canHandOver(notice) ? "Continue on another account…" : "Open its account", primary: true, onClick: () => actions.primary(notice), testId: "quota-banner-hand-over" },
              ...(second?.id === "remind-reset" ? [{ label: second.label, onClick: () => actions.second(notice, "remind-reset"), testId: "quota-banner-remind" }] : []),
              { label: "Snooze 1 h", quiet: true, testId: "quota-banner-snooze", onClick: () => hideWhile(key, () => actions.snooze(notice, "hour")) },
            ]}
            onDismiss={() => hideWhile(key, () => acted(() => notificationsDismiss(notice.id)))}
            dismissLabel={`Close: ${notice.title} (it stays in Notifications)`}
            testId="quota-banner"
          />
        ),
      });
    }
  for (const row of queue) {
    const key = `reminder-${row.id}`;
    if (row.state !== "ready" || hidden.has(key)) continue;
    const { name, open } = reminderOf(row);
    const done = () => hideWhile(key, () => resetQueueDismiss(row.id));
    items.push({
      key,
      node: (
        <Banner
          key={key}
          tone="accent"
          title={name ? `${accountName(row.account)} has room again: ${name} can carry on` : `${accountName(row.account)} has room again`}
          detail={open ? "The reminder you set. Nothing restarted by itself." : "The reminder you set. Its session is no longer open."}
          actions={open ? [{ label: name ? `Open ${name}` : "Open it", primary: true, testId: "reminder-open", onClick: () => (done(), open()) }] : []}
          onDismiss={done}
          dismissLabel="Close the reminder"
          testId="reminder-ready"
        />
      ),
    });
  }
  for (const offer of offers) {
    const key = `parked-${offer.session_id}`;
    if (hidden.has(key)) continue;
    items.push({
      key,
      node: (
        <Banner
          key={key}
          tone="accent"
          title={offer.text}
          actions={[
            { label: "Resume", primary: true, testId: "parked-offer-resume", onClick: () => hideWhile(key, () => parkedResume(offer.session_id).then(() => setOffers((rows) => rows.filter((r) => r.session_id !== offer.session_id)))) },
            { label: offer.last ? "Stop reminding me" : "Remind me in 30 min", quiet: true, testId: offer.last ? "parked-offer-stop" : "parked-offer-snooze", onClick: () => hideWhile(key, () => parkedDismiss(offer.session_id).then(() => setOffers((rows) => rows.filter((r) => r.session_id !== offer.session_id)))) },
          ]}
          testId="parked-offer"
        />
      ),
    });
  }
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

function without(set: ReadonlySet<string>, key: string): ReadonlySet<string> {
  const next = new Set(set);
  next.delete(key);
  return next;
}

function folderOf(cwd: string | null | undefined): string | null {
  return (cwd ?? "").replace(/\/+$/, "").split("/").pop() || null;
}
