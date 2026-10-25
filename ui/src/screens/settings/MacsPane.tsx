/**
 * AJ-E2110 — Settings › Your Macs (shown once a Mac is paired): what
 * each paired Mac may do on this one, through the Fleet screen's sheet.
 */
import type { JSX } from "react";
import { FleetPermissionsCard } from "../../components/SettingsSections";
import { Pane, PaneSection } from "./parts";

export function MacsPane({ anchor }: { anchor?: string }): JSX.Element {
  return (
    <Pane id="macs" title="Your Macs" anchor={anchor}>
      <PaneSection
        id="fleet"
        title="What your other Macs may do here"
        footer="Only this Mac can change it, and no Mac ever reads another's sign-ins."
        card
      >
        <FleetPermissionsCard />
      </PaneSection>
    </Pane>
  );
}
