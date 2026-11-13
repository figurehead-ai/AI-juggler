/**
 * AJ-E3208 — the Accounts screen's Apps: every App beside the Accounts,
 * each as easy to reach (the maintainer, 2026-11-01: "expanded into both
 * Apps and Accounts so both can be easily get to"). A row is an App: its
 * icon and mark, its whole name, the Account it runs on, what's open in
 * it, and its one action (Start a closed desktop App, Go to a running
 * one, New session on a CLI App); its ⋯ menu holds its page, Keep alive
 * and Restart… (D-71). Running ones first, then the rest, then the
 * profiles found but never opened.
 */
import { useMemo, useState, type JSX, type MouseEvent } from "react";
import { AppIcon } from "../../components/AppIcon";
import { ProfileBadge } from "../../components/ProfileBadge";
import { Button } from "../../components/primitives";
import { MenuItem, RowMenu, menuPoint, type MenuAt } from "../../components/RowMenu";
import { askRestart } from "../../components/RestartApp";
import { keepAliveOf } from "../../components/place-actions";
import { showToast } from "../../components/Toast";
import { macApps, type AccountApp } from "../../lib/account-items";
import { openApp, type AppModel } from "../../lib/apps";
import { errorText, signedIn } from "../../lib/bridge";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import { placesOpen } from "../../lib/ipc/places";
import { showGoTo } from "../../lib/go-to";
import { appByConversation } from "../../lib/rail-apps";
import { itemsOf } from "../../lib/rail-items";
import { useSeen } from "../../lib/seen";
import { accountNameOf, type SubscriptionInfo } from "../../lib/subscriptions";
import { agoWords } from "../../lib/time-words";
import { placeWords } from "../../lib/places";
import { homeRelative } from "../../lib/project-words";
import { AppKindBadge } from "../../components/AppKindBadge";
import { ProfileRoomChip } from "../../components/RoomChip";
import { useLastUsed, type LastUsed } from "../../lib/app-last-used";

/** The Apps in the order the section shows them: running, then the rest,
 * then found and never opened. */
export function appsInOrder(rows: AccountApp[]): AccountApp[] {
  const rank = (row: AccountApp) => (running(row.app) || row.open > 0 ? 0 : row.app.place?.found ? 2 : 1);
  return [...rows].sort((a, b) => rank(a) - rank(b) || b.lastMs - a.lastMs || a.app.name.localeCompare(b.app.name));
}

function running(app: AppModel): boolean {
  return app.place?.app_pid != null;
}

/** A row with its last use as the backend's list has it, when the read
 * sessions had none. */
function withLastUsed(row: AccountApp, last: LastUsed | undefined): AccountApp {
  return last && row.sessions === 0 ? { ...row, sessions: 1, lastMs: Math.max(row.lastMs, last.lastMs), lastCwd: last.cwd } : row;
}

/** What's in it now, in words. */
export function stateWords(row: AccountApp, now: number): string {
  const { app, open, sessions, lastMs } = row;
  // 0.3.8 (AJ-E3208 R4): Go to it then asks to restart it; Restart… is in its ⋯.
  if (app.place?.lost) return "Lost its window when the system restarted the display";
  if (open > 0) return `${open} open`;
  if (running(app)) return "Running, nothing open";
  if (app.place?.found) return "Found, never opened here";
  // D-103.5: a closed desktop App reads as Home's tile and its page do:
  // "Closed · last open 12 min ago".
  if (app.kind === "desktop" && app.place) return placeWords(app.place, now);
  // D-100: a CLI App says where it was last used ("in ~/lab/aj-processes"):
  // the maintainer's are agents' lab runs on a profile's folder.
  if (sessions > 0 && lastMs > 0) return [`Last used ${agoWords(lastMs, now)}`, app.kind === "cli" && row.lastCwd ? `in ${homeRelative(row.lastCwd)}` : null].filter(Boolean).join(" ");
  return app.kind === "cli" ? "No sessions yet" : "Closed";
}

export function AppsSection({
  apps,
  subscriptions,
  sessions,
  conversations,
  now,
}: {
  apps: AppModel[] | null;
  subscriptions: SubscriptionInfo[];
  sessions: InstanceInfo[];
  conversations: ConversationInfo[];
  now: number;
}): JSX.Element {
  const seen = useSeen();
  const items = useMemo(() => itemsOf({ sessions, conversations, projects: [], seen }), [sessions, conversations, seen]);
  const byConversation = useMemo(() => appByConversation((apps ?? []).flatMap((app) => (app.place ? [app.place] : []))), [apps]);
  const counted = useMemo(() => appsInOrder(macApps(apps ?? [], items, byConversation)), [apps, items, byConversation]);
  // A CLI App the read sessions say nothing of: its own newest one, asked.
  const last = useLastUsed(counted.filter((row) => row.app.kind === "cli" && row.sessions === 0).map((row) => row.app.id));
  const rows = useMemo(() => counted.map((row) => withLastUsed(row, last.get(row.app.id))), [counted, last]);
  if (apps === null) return <p className="text-meta text-ink-2">Reading your apps…</p>;
  if (rows.length === 0) return <p className="text-meta text-ink-2" data-testid="accounts-apps-empty">No apps yet. Add one with Add an app.</p>;
  return (
    <ul className="flex flex-col divide-y divide-hairline/60 rounded-xl border border-hairline bg-surface" data-testid="accounts-apps">
      {rows.map((row) => (
        <AppRow key={row.app.id} row={row} account={accountOf(row.app, subscriptions)} now={now} />
      ))}
    </ul>
  );
}

/** Its line after its kind: what it's doing, then its Account ("Last
 * used 2 h ago in ~/lab/aj-processes · on Work"). */
function appLine(row: AccountApp, account: string | null, now: number): string {
  const { app } = row;
  const on = account ? `on ${account}` : app.kind === "cli" && app.folder && !signedIn(app.folder.login_state) ? "not signed in" : "account not known yet";
  return [stateWords(row, now), on].join(" · ");
}

/** The Account an App runs on, by name; `null` when none is known. */
function accountOf(app: AppModel, subscriptions: SubscriptionInfo[]): string | null {
  const folder = app.folder?.id ?? app.place?.account_id ?? null;
  if (!folder) return null;
  const name = accountNameOf(subscriptions, folder, "");
  return name || null;
}

function AppRow({ row, account, now }: { row: AccountApp; account: string | null; now: number }): JSX.Element {
  const { app } = row;
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const keepAlive = keepAliveOf(app.place);
  const openMenu = (event: MouseEvent<HTMLElement>) => {
    event.preventDefault();
    setMenu(menuPoint(event));
  };
  return (
    <li className="flex min-w-0 items-center gap-3 px-3 py-2.5" data-testid="accounts-app" data-app-id={app.id} onContextMenu={openMenu}>
      <AppIcon appId={app.appId} appPid={app.place?.app_pid ?? null} name={app.app} size={24} />
      <button type="button" onClick={() => openApp(app.id)} className="flex min-w-0 flex-1 flex-col items-start text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/60" title={`Open ${app.name}'s page`}>
        <span className="flex min-w-0 max-w-full items-center gap-1.5 text-body font-medium text-ink">
          {app.profile && <ProfileBadge app={app.slot} name={app.profile} />}
          <span className="truncate hover:underline">{app.name}</span>
        </span>
        {/* D-100: its kind first, by icon and word ("CLI", "Desktop"), so
            two Apps on one Account never read as one; then what it's
            doing and the Account it runs on. */}
        <span className="flex min-w-0 max-w-full items-center gap-1.5 text-meta text-ink-2">
          <AppKindBadge kind={app.kind} variant="chip" />
          <span className="truncate" data-testid="accounts-app-line">{appLine(row, account, now)}</span>
        </span>
      </button>
      {/* 0.4.3: its Account out or low, the one mark it wears on the rail
          and Home (spec 11 rule 6), from its own folder. */}
      <ProfileRoomChip profileId={app.folder?.id} testId="accounts-app-room" />
      <AppPrimary app={app} />
      <button
        type="button"
        aria-label={`Actions for ${app.name}`}
        onClick={openMenu}
        className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60"
        data-testid="accounts-app-menu"
      >
        ⋯
      </button>
      {menu && (
        <RowMenu heading={app.name} ariaLabel={`Actions for ${app.name}`} at={menu} onClose={() => setMenu(null)} testId="accounts-app-actions">
          <MenuItem onClick={() => (setMenu(null), openApp(app.id))} testId="accounts-app-page">
            Open its page
          </MenuItem>
          {running(app) && app.place && (
            <MenuItem onClick={() => (setMenu(null), askRestart({ placeId: app.place!.id, name: app.name, open: row.open, working: row.working }))} testId="accounts-app-restart">
              Restart…
            </MenuItem>
          )}
          {keepAlive && (
            <MenuItem onClick={() => (setMenu(null), keepAlive.toggle())} checked={keepAlive.on} multi testId="accounts-app-keep">
              <span className="flex items-center gap-2">
                <span className="flex-1">Keep alive</span>
                <span aria-hidden="true" className={"shrink-0 text-accent-hi " + (keepAlive.on ? "" : "invisible")}>
                  ✓
                </span>
              </span>
            </MenuItem>
          )}
        </RowMenu>
      )}
    </li>
  );
}

/** Its one action: New session (CLI), Start (closed), Go to it (running). */
function AppPrimary({ app }: { app: AppModel }): JSX.Element | null {
  if (app.kind === "cli") {
    if (!app.folder || !signedIn(app.folder.login_state)) return null;
    const folder = app.folder.id;
    return (
      <Button size="sm" variant="secondary" onClick={() => window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: folder } }))} data-testid="accounts-app-new">
        New session
      </Button>
    );
  }
  const place = app.place;
  if (!place) return null;
  const isRunning = place.app_pid !== null;
  return (
    <Button
      size="sm"
      variant="secondary"
      disabled={!isRunning && place.cannot_start !== null}
      title={isRunning ? undefined : (place.cannot_start ?? undefined)}
      onClick={() => {
        showToast({ id: "go-to", message: isRunning ? `Bringing ${app.name} forward…` : `Starting ${app.name}…` });
        placesOpen(place.id).then(showGoTo, (cause) => showToast({ id: "go-to", message: errorText(cause) }));
      }}
      data-testid="accounts-app-go"
    >
      {isRunning ? "Go to it" : "Start"}
    </Button>
  );
}
