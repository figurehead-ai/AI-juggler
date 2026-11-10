/**
 * A5 (the AJ-E3306 rc22 walk): an Account out of room, or low, says on its
 * own page what that affects — the sessions open on it, each with its
 * switch — so moving the work is one click from here, not two pages away.
 *
 * Each switch is the one its session's page, its row menu and ⌘⇧H run
 * (the action registry's `handover` action: AI Juggler's session or one
 * another app runs), so it opens that session's own switch sheet; there is
 * no second path. The account's notice keeps its Remind me and Snooze 1 h,
 * as on a session's page. Nothing open says so, with when it resets.
 *
 * Inline on the Overview, never a band over the page (D-87); it offers,
 * it never moves anything by itself (D-31).
 */
import { useId, useState, type JSX } from "react";
import { Section } from "../../components/charts/ChartCard";
import { Button } from "../../components/primitives";
import { RoomChip } from "../../components/RoomChip";
import { IconGaugeEmpty, IconGaugeLow } from "../../components/icons";
import type { ActionHost } from "../../lib/actions";
import { resolveActions, type ResolvedAction } from "../../lib/actions";
import { usePagedList } from "../../lib/paged";
import { notificationsPage, type NoticeInfo } from "../../lib/ipc/notices";
import { useNoticeActions } from "../../lib/notice-actions";
import { secondOf } from "../../lib/notice-words";
import { resetsWords } from "../../lib/time-words";
import type { RailItem } from "../../lib/rail-items";
import type { RoomMark } from "../../lib/room-mark";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { ListRow } from "../SessionRow";
import type { Shown } from "../app/AppOverview";

/** The switch for one open item: the action ⌘⇧H runs on it. */
function switchOf(item: RailItem, host: ActionHost): ResolvedAction | null {
  const actions = item.session ? resolveActions("session", item.session, host) : item.conversation ? resolveActions("conversation", item.conversation, host) : [];
  return actions.find((action) => action.shortcutId === "handover") ?? null;
}

export function AccountAffects({
  mark,
  folders,
  open,
  rows,
  host,
  sessions,
  now,
}: {
  mark: RoomMark;
  /** The Account's folders: its notice is about one of them. */
  folders: ReadonlySet<string>;
  /** What's open on it now, newest first. */
  open: RailItem[];
  rows: (list: RailItem[]) => Shown[];
  host: ActionHost;
  sessions: InstanceInfo[];
  now: number;
}): JSX.Element {
  const Icon = mark.kind === "out" ? IconGaugeEmpty : IconGaugeLow;
  const shown = rows(open);
  const byKey = new Map(open.map((item) => [item.key, item]));
  const reset = mark.resetsAtMs !== null && mark.resetsAtMs > now ? resetsWords(mark.resetsAtMs, now) : null;
  return (
    <Section
      title="What this affects"
      icon={<Icon width={16} height={16} />}
      sub={<RoomChip mark={mark} testId="account-affects-mark" />}
      actions={<NoticeActions folders={folders} sessions={sessions} />}
      testId="account-affects"
    >
      {shown.length === 0 ? (
        <p className="text-meta text-ink-2" data-testid="account-affects-none">
          Nothing open on this account now{reset ? ` · ${reset}` : ""}.
        </p>
      ) : (
        <ul className="-mx-2 flex flex-col">
          {shown.map(({ row, project }) => {
            const item = byKey.get(row.rail.key);
            const action = item ? switchOf(item, host) : null;
            return (
              <li key={row.key} className="border-t border-hairline/60 first:border-t-0" data-testid="account-affects-item" data-key={row.key}>
                <ListRow row={row} project={project} badge={action && <SwitchButton action={action} title={row.title} />} />
              </li>
            );
          })}
        </ul>
      )}
    </Section>
  );
}

function SwitchButton({ action, title }: { action: ResolvedAction; title: string }): JSX.Element {
  const why = useId();
  return (
    <>
      <Button
        size="sm"
        variant="secondary"
        disabled={action.reason !== undefined}
        title={action.reason}
        aria-describedby={action.reason ? why : undefined}
        onClick={() => void action.run()}
        data-testid="account-affects-switch"
      >
        {action.label}
        <span className="sr-only"> {title}</span>
      </Button>
      {action.reason && (
        <span id={why} className="sr-only">
          {action.reason}
        </span>
      )}
    </>
  );
}

/** The account's notice's own Remind me and Snooze 1 h, as a session's
 * page offers them; gone once snoozed, or with no notice. */
function NoticeActions({ folders, sessions }: { folders: ReadonlySet<string>; sessions: InstanceInfo[] }): JSX.Element | null {
  const actions = useNoticeActions(sessions);
  const banners = usePagedList<NoticeInfo>("notices:banners", (request) => notificationsPage("banners", request), { keyOf: (row) => String(row.id) });
  const [snoozed, setSnoozed] = useState<number | null>(null);
  const notice = banners.items.find((row) => row.kind === "quota" && row.target.account_id !== null && folders.has(row.target.account_id));
  if (!notice || snoozed === notice.id) return null;
  const second = secondOf(notice);
  return (
    <>
      {second?.id === "remind-reset" && (
        <Button size="sm" variant="secondary" onClick={() => actions.second(notice, "remind-reset")} data-testid="account-affects-remind">
          {second.label}
        </Button>
      )}
      <Button
        size="sm"
        variant="ghost"
        onClick={() => {
          setSnoozed(notice.id);
          void actions.snooze(notice, "hour").then((done) => done || setSnoozed(null), () => setSnoozed(null));
        }}
        data-testid="account-affects-snooze"
      >
        Snooze 1 h
      </Button>
    </>
  );
}
