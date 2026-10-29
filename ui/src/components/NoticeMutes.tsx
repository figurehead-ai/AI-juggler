/**
 * AJ-E2503 — Settings › Notifications › Muted: the projects and kinds the
 * user muted from the notification centre (they stop interrupting; Needs
 * you still counts them), a page at a time, each with Unmute.
 */
import type { JSX } from "react";
import { Button, Card } from "./primitives";
import { usePagedList } from "../lib/paged";
import { notificationsMutes, notificationsUnmute, type NoticeMuteInfo } from "../lib/ipc/notices";
import { acted } from "../lib/notices-store";

export function NoticeMutes(): JSX.Element {
  const list = usePagedList<NoticeMuteInfo>("notices:mutes", notificationsMutes, { keyOf: (mute) => `${mute.scope}:${mute.value}` });
  if (!list.loading && list.items.length === 0) {
    return (
      <Card className="p-4">
        <p className="text-meta text-muted" data-testid="notice-mutes-none">
          Nothing is muted. In the notification centre, ⋯ on an item mutes its project or its kind.
        </p>
      </Card>
    );
  }
  return (
    <Card className="flex flex-col gap-2 p-4" data-testid="notice-mutes">
      <ul className="flex flex-col divide-y divide-hairline/60 rounded-lg border border-hairline">
        {list.items.map((mute) => (
          <li key={`${mute.scope}:${mute.value}`} className="flex items-center gap-3 px-3 py-2">
            <span className="min-w-0 flex-1">
              <span className="block truncate text-body text-ink">{mute.label}</span>
              <span className="text-label text-muted">{mute.scope === "project" ? "A project" : "A kind of notification"}</span>
            </span>
            <Button size="sm" variant="secondary" onClick={() => void acted(() => notificationsUnmute(mute.scope, mute.value), "Couldn't unmute it").then(() => list.refresh())}>
              Unmute<span className="sr-only"> {mute.label}</span>
            </Button>
          </li>
        ))}
      </ul>
      {list.hasMore && (
        <Button size="sm" variant="ghost" onClick={list.loadMore} disabled={list.loadingMore} className="self-start">
          Show more
        </Button>
      )}
    </Card>
  );
}
