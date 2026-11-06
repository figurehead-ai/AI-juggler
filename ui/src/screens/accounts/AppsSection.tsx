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
import { errorText, signedIn, type InstanceInfo } from "../../lib/bridge";
import type { ConversationInfo } from "../../lib/ipc/conversations";
import { placesOpen } from "../../lib/ipc/places";
import { showGoTo } from "../../lib/go-to";
import { appByConversation } from "../../lib/rail-apps";
import { itemsOf } from "../../lib/rail-items";
import { useSeen } from "../../lib/seen";
import { accountNameOf, type SubscriptionInfo } from "../../lib/subscriptions";
import { agoWords } from "../../lib/time-words";

/** The Apps in the order the section shows them: running, then the rest,
 * then found and never opened. */
export function appsInOrder(rows: AccountApp[]): AccountApp[] {
  const rank = (row: AccountApp) => (running(row.app) || row.open > 0 ? 0 : row.app.place?.found ? 2 : 1);
  return [...rows].sort((a, b) => rank(a) - rank(b) || b.lastMs - a.lastMs || a.app.name.localeCompare(b.app.name));
}

function running(app: AppModel): boolean {
  return app.place?.app_pid != null;
}

/** What's in it now, in words. */
function stateWords(row: AccountApp, now: number): string {
  const { app, open, sessions, lastMs } = row;
  if (open > 0) return `${open} open`;
  if (running(app)) return "Running, nothing open";
  if (app.place?.found) return "Found, never opened here";
  if (sessions > 0 && lastMs > 0) return `Last used ${agoWords(lastMs, now)}`;
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
  const rows = useMemo(() => appsInOrder(macApps(apps ?? [], items, byConversation)), [apps, items, byConversation]);
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
        <span className="flex min-w-0 max-w-full items-center gap-1.5 text-meta text-ink-2">
          <span className="truncate">{[account ? `on ${account}` : app.kind === "cli" && app.folder && !signedIn(app.folder.login_state) ? "Not signed in" : "Its account isn't known yet", stateWords(row, now)].join(" · ")}</span>
        </span>
      </button>
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
            <MenuItem onClick={() => (setMenu(null), askRestart({ placeId: app.place!.id, name: app.name, working: row.open }))} testId="accounts-app-restart">
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
