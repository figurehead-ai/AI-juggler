/**
 * D-64 (0.3.4) — an app with nothing open, in the rail's Inactive apps
 * (the maintainer: a Claude app that quit overnight vanished from the
 * rail, with no way to start it). Drawn as Open now draws a place's
 * header (its fold, icon, app, profile), with when it was last active,
 * and its one action on the right: Start a desktop app that isn't
 * running, Bring forward one running with nothing open, New session on a
 * CLI app's folder. Its fold shows its last 3 inactive sessions under it;
 * a right-click (or Shift+F10) opens its menu: the action, Keep alive,
 * Show its last 3.
 */
import { useState, type JSX, type MouseEvent } from "react";
import { MenuItem, RowMenu, menuPoint, type MenuAt } from "./RowMenu";
import { ProfileBadge } from "./ProfileBadge";
import { AppIcon } from "./AppIcon";
import { FoldButton } from "./rail-groups";
import { IconUpRight } from "./icons-places";
import { IconStateRunning } from "./icons";
import { IconPlus } from "./icons-actions";
import { showToast } from "./Toast";
import { placesOpen } from "../lib/ipc/places";
import { agoWords } from "../lib/time-words";
import { RECENT_INLINE, type IdleApp } from "../lib/rail-apps";

/** What the app's one action is, in words and as an icon. */
export function idleAction(app: IdleApp): { words: string; Icon: typeof IconPlus } {
  if (app.kind === "cli") return { words: "New session", Icon: IconPlus };
  return app.appPid !== null ? { words: "Bring forward", Icon: IconUpRight } : { words: "Start", Icon: IconStateRunning };
}

/** Short enough for the rail's width: "Not running · 20 min ago",
 * "Running · 3 h ago · 2 sessions"; a CLI app "20 h ago · 2 sessions".
 * When: its newest session, else when it was last seen running. */
export function idleWords(app: IdleApp, now: number): string {
  const count = app.sessions.length;
  return [
    app.kind === "desktop" ? (app.appPid !== null ? "Running" : "Not running") : null,
    app.lastMs > 0 ? agoWords(app.lastMs, now) : null,
    count > 0 ? `${count} ${count === 1 ? "session" : "sessions"}` : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

export function RailIdleApp({
  app,
  open,
  onToggle,
  now,
  keepAlive = null,
}: {
  app: IdleApp;
  /** Its last ones show under it. */
  open: boolean;
  onToggle: () => void;
  now: number;
  /** A desktop app's Keep alive (D-63). */
  keepAlive?: { on: boolean; toggle: () => void } | null;
}): JSX.Element {
  const [menu, setMenu] = useState<MenuAt | null>(null);
  const { words, Icon } = idleAction(app);
  const blocked = app.kind === "desktop" && app.appPid === null ? app.cannotStart : null;
  const act = () => {
    if (app.kind === "cli") {
      window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId: app.accountId } }));
      return;
    }
    if (app.placeId === null) return;
    const running = app.appPid !== null;
    showToast({ id: "go-to", message: running ? `Bringing ${app.name} forward…` : `Starting ${app.name}…` });
    placesOpen(app.placeId).then(
      (outcome) => showToast({ id: "go-to", message: outcome.outcome }),
      () => showToast({ id: "go-to", message: running ? `Couldn't bring ${app.name} forward` : `Couldn't start ${app.name}` }),
    );
  };
  const run = (action: () => void) => () => (setMenu(null), action());
  const has = app.sessions.length > 0;
  const lastWords = `Show its last ${Math.min(app.sessions.length, RECENT_INLINE)}`;
  return (
    <div
      onContextMenu={(event: MouseEvent<HTMLDivElement>) => {
        event.preventDefault();
        setMenu(menuPoint(event));
      }}
      onKeyDown={(event) => {
        if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
          event.preventDefault();
          const box = event.currentTarget.getBoundingClientRect();
          setMenu({ x: box.left + 8, y: box.bottom + 4 });
        }
      }}
      className="flex min-h-[44px] min-w-0 items-center gap-0.5 rounded-md pr-1"
      data-testid="rail-idle-app"
      data-app={app.key}
      data-kind={app.kind}
    >
      {has ? <FoldButton open={open} label={app.name} onToggle={onToggle} /> : <span aria-hidden="true" className="w-6 shrink-0" />}
      <button
        type="button"
        onClick={has ? onToggle : act}
        data-rail-nav
        aria-expanded={has ? open : undefined}
        className="relative flex min-h-10 min-w-0 flex-1 flex-col justify-center gap-0.5 rounded-md px-1 py-0.5 text-left outline-none hover:bg-surface/60 focus-visible:ring-1 focus-visible:ring-accent/60"
      >
        <span className="sr-only">{`${app.name}, ${idleWords(app, now)}`}</span>
        <span aria-hidden="true" className="flex min-w-0 items-center gap-1">
          <AppIcon appId={app.appId} appPid={app.appPid} name={app.app} size={16} />
          <span className="ml-0.5 shrink-0 truncate text-label font-semibold text-ink-2">{app.app}</span>
          {app.profile && (
            <span className="flex min-w-[4ch] shrink items-center gap-[3px] text-label font-medium text-muted">
              <ProfileBadge name={app.profile} size={6} />
              <span className="truncate">{app.profile}</span>
            </span>
          )}
        </span>
        <span aria-hidden="true" className="truncate pl-[21px] text-label text-muted" data-testid="rail-idle-when">
          {idleWords(app, now)}
        </span>
      </button>
      <button
        type="button"
        onClick={act}
        disabled={blocked !== null}
        aria-label={`${words}: ${app.name}`}
        title={blocked ?? `${words}: ${app.name}`}
        data-testid="rail-idle-action"
        data-action={words}
        className="flex size-6 shrink-0 items-center justify-center rounded-md border border-hairline text-muted outline-none hover:border-accent/40 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60 disabled:cursor-default disabled:opacity-50"
      >
        <Icon width={12} height={12} />
      </button>
      {menu && (
        <RowMenu heading={app.name} ariaLabel={`Actions for ${app.name}`} at={menu} onClose={() => setMenu(null)} testId="rail-idle-menu">
          <MenuItem onClick={run(act)} disabled={blocked !== null} title={blocked ?? undefined} testId="rail-idle-menu-act">
            {words}
          </MenuItem>
          {keepAlive && (
            <MenuItem onClick={run(keepAlive.toggle)} checked={keepAlive.on} multi testId="rail-idle-menu-keep">
              <span className="flex items-center gap-2">
                <span className="flex-1">Keep alive</span>
                <span aria-hidden="true" className={"shrink-0 text-accent-hi " + (keepAlive.on ? "" : "invisible")}>
                  ✓
                </span>
              </span>
            </MenuItem>
          )}
          {has && (
            <MenuItem onClick={run(onToggle)} testId="rail-idle-menu-fold">
              {open ? "Hide its last ones" : lastWords}
            </MenuItem>
          )}
        </RowMenu>
      )}
    </div>
  );
}
