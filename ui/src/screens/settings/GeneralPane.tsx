/**
 * AJ-E2110 — Settings › General: how AI Juggler looks, which editor
 * opens a project, and About & updates (the update prompt's Review
 * lands on it).
 */
import type { JSX } from "react";
import { AppearanceCard } from "../../components/AppearanceCard";
import { EditorCard } from "../../components/EditorCard";
import { UpdatesCard } from "../../components/UpdatesCard";
import { Pane, PaneSection } from "./parts";

export function GeneralPane({ anchor }: { anchor?: string }): JSX.Element {
  return (
    <Pane id="general" title="General" anchor={anchor}>
      <PaneSection id="appearance" title="Appearance" footer="The theme, and how much each row in the session list shows." card>
        <AppearanceCard />
      </PaneSection>
      <PaneSection id="editor" title="Open projects in" footer="The editor a project opens in from its menu; Finder when none is chosen." card>
        <EditorCard />
      </PaneSection>
      <PaneSection id="about" title="About & updates" footer="Which version this is, its channel, and when AI Juggler looks for a new one." card>
        <UpdatesCard />
      </PaneSection>
    </Pane>
  );
}
