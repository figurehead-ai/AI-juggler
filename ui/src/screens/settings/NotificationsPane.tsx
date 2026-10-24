/**
 * AJ-E2110 — Settings › Notifications: what reaches you in AI Juggler
 * and from macOS, the digest's quiet hours, and the accounts left out.
 */
import type { JSX } from "react";
import { NotificationSettingsCard } from "../../components/NotificationSettings";
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
    </Pane>
  );
}
