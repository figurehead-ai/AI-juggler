/**
 * AJ-E2502 (§1.1, §4.14; the approved design of 2026-10-27) — the
 * header's bell: ONE count, the same number as Needs you on Home, the app
 * badge and the menu-bar item (the notification model's open needs at
 * attention level 2 or 3, D-117), and
 * the notification centre it opens (or ⌥⌘N). The centre loads when it
 * first opens.
 *
 * F-rc3f-2 (0.4.6 rc3f): the centre opens on the body, fixed under the
 * bell, as your menu's does: placed in the top bar, it painted under a
 * page's own panels (a pool's Tasks) that come later at the same level.
 */
import { useRef, type JSX } from "react";
import { createPortal } from "react-dom";
import { IconBell } from "./icons-actions";
import { lazyNamed } from "../lib/lazy";
import { setCentreOpen, toggleCentre, useCentreOpen, useNoticeSummary } from "../lib/notices-store";
import { bellLabel } from "../lib/notice-words";
import { useKeys } from "../lib/use-keys";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import { useBackendNeeds } from "../lib/needs-you";
import { brokenOf, counted, highest, noticeLevel } from "../lib/attention";

const NotificationCentre = lazyNamed(() => import("./NotificationCentre"), "NotificationCentre");

export function NotificationBell({ sessions }: { sessions: InstanceInfo[] }): JSX.Element {
  const keys = useKeys("notifications");
  const summary = useNoticeSummary();
  const open = useCentreOpen();
  // NY1 (0.4.6 rc3a): the rows Needs you lists, counted (one source): the
  // summary's own count said 1 over "You're all caught up" for 50 s.
  // D-117: the count holds L2 and L3; its badge is neutral (by luminance,
  // not hue) and red only when one of them is broken (L3).
  const { rows, count } = useBackendNeeds();
  const level = highest(rows.map(noticeLevel).filter(counted));
  const broken = brokenOf(rows, noticeLevel, (row) => row.kind);
  const anchor = useRef<HTMLDivElement | null>(null);
  return (
    <div ref={anchor} className="relative" data-notification-bell>
      <button
        type="button"
        onClick={toggleCentre}
        aria-label={bellLabel(count, summary?.unread ?? 0, broken)}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-keyshortcuts={keys.aria}
        title={`Notifications (${keys.words})`}
        data-testid="notification-bell"
        className={"relative flex size-9 items-center justify-center rounded-lg border outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " + (open ? "border-hairline bg-surface-3 text-ink" : "border-transparent text-muted hover:bg-surface hover:text-ink")}
      >
        <IconBell width={18} height={18} aria-hidden="true" />
        {count > 0 && (
          <span aria-hidden="true" data-testid="bell-count" data-level={level} className={"absolute -right-1 -top-1 flex h-[18px] min-w-[18px] items-center justify-center rounded-full px-1 text-label font-semibold leading-none ring-2 ring-surface " + (level === 3 ? "bg-count-alarm text-count-alarm-ink" : "bg-count text-count-ink")}>
            {count > 99 ? "99+" : count}
          </span>
        )}
      </button>
      {open && createPortal(<NotificationCentre summary={summary} sessions={sessions} anchor={anchor} onClose={() => setCentreOpen(false)} />, document.body)}
    </div>
  );
}
