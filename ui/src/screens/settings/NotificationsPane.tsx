/**
 * AJ-E2110 — Settings › Notifications: what reaches you in AI Juggler
 * and from macOS, the digest's quiet hours, the accounts left out, and
 * (AJ-E2503) the projects and kinds muted from the notification centre.
 */
import type { JSX } from "react";
import { NotificationSettingsCard } from "../../components/NotificationSettings";
import { NoticeMutes } from "../../components/NoticeMutes";
import { useAccounts } from "./AccountsPane";
import { Pane, PaneSection } from "./parts";

export function NotificationsPane({ anchor }: { anchor?: string }): JSX.Element {
  const accounts = useAccounts();
  return (
    <Pane id="notifications" title="Notifications" anchor={anchor}>
      <PaneSection
        id="notifications"
        title="Alerts and the digest"
        footer="Alerts you can act on show once; the rest waits for the hourly digest."
        card
      >
        <NotificationSettingsCard accounts={accounts} />
      </PaneSection>
      {/* AJ-E2503: what the notification centre's ⋯ muted. */}
      <PaneSection id="notification-mutes" title="Muted" footer="The notification centre keeps 30 days of history." card>
        <NoticeMutes />
      </PaneSection>
    </Pane>
  );
}
