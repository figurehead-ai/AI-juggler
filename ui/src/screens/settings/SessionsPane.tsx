/**
 * AJ-E2110 — Settings › Sessions: when a quiet session looks stuck, how
 * its terminal reads, and what keeps work going (AJ-E1505, AJ-E1509).
 * One card owns these prefs, so no part saves over another's.
 */
import type { JSX } from "react";
import { SessionsSettingsCard } from "../../components/SessionsSettings";
import { Pane, PaneSection } from "./parts";

export function SessionsPane({ anchor }: { anchor?: string }): JSX.Element {
  return (
    <Pane id="sessions" title="Sessions" anchor={anchor}>
      <PaneSection
        id="sessions"
        title="Sessions AI Juggler starts"
        footer="These apply to the sessions AI Juggler runs; sessions other apps run keep their own settings."
        card
      >
        <SessionsSettingsCard />
      </PaneSection>
    </Pane>
  );
}
