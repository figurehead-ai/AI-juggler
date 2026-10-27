/**
 * AJ-E2502 (§1.1, §4.14; the approved design of 2026-10-27) — the
 * header's bell: ONE count, the same number as Needs you on Home, the app
 * badge and the menu-bar item (the notification model's open needs), and
 * the notification centre it opens (or ⌘⇧N). The centre loads when it
 * first opens.
 */
import type { JSX } from "react";
import { IconBell } from "./icons-actions";
import { lazyNamed } from "../lib/lazy";
import { setCentreOpen, toggleCentre, useCentreOpen, useNoticeSummary } from "../lib/notices-store";
import { bellLabel } from "../lib/notice-words";
import type { InstanceInfo } from "../lib/bridge";

const NotificationCentre = lazyNamed(() => import("./NotificationCentre"), "NotificationCentre");

export function NotificationBell({ sessions }: { sessions: InstanceInfo[] }): JSX.Element {
  const summary = useNoticeSummary();
  const open = useCentreOpen();
  const count = summary?.count ?? 0;
  return (
    <div className="relative" data-notification-bell>
      <button
        type="button"
        onClick={toggleCentre}
        aria-label={bellLabel(count, summary?.unread ?? 0)}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-keyshortcuts="Meta+Shift+N"
        title="Notifications (⌘⇧N)"
        data-testid="notification-bell"
        className={"relative flex size-9 items-center justify-center rounded-lg border outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " + (open ? "border-hairline bg-surface-3 text-ink" : "border-transparent text-muted hover:bg-surface hover:text-ink")}
      >
        <IconBell width={18} height={18} aria-hidden="true" />
        {count > 0 && (
          <span aria-hidden="true" data-testid="bell-count" className="absolute -right-1 -top-1 flex h-[18px] min-w-[18px] items-center justify-center rounded-full bg-danger px-1 text-label font-semibold leading-none text-canvas ring-2 ring-surface">
            {count > 99 ? "99+" : count}
          </span>
        )}
      </button>
      {open && <NotificationCentre summary={summary} sessions={sessions} onClose={() => setCentreOpen(false)} />}
    </div>
  );
}
