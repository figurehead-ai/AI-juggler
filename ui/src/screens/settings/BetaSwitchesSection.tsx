/**
 * D-127: Settings › Switching › Beta, the one door to switches still in
 * beta. Off by default: AI Juggler offers a switch only once every part of
 * it was tried with the real tools. On, the others are listed too, each
 * marked Beta where it shows.
 */
import { useEffect, useState, type JSX } from "react";
import { errorText } from "../../lib/bridge";
import { appSettingGet, appSettingSet } from "../../lib/ipc/settings";
import { PaneSection, SwitchRow } from "./parts";

/** The setting (the backend's `features::validation::BETA_SWITCHES_KEY`). */
export const BETA_SWITCHES_KEY = "switching.beta";

export function BetaSwitchesSection(): JSX.Element {
  const [on, setOn] = useState<boolean | null>(null);
  const [note, setNote] = useState<string | null>(null);
  useEffect(() => {
    void appSettingGet(BETA_SWITCHES_KEY).then(
      (value) => setOn(value === "1"),
      () => setOn(false),
    );
  }, []);
  return (
    <PaneSection id="beta" title="Beta" footer="Accounts lists which switches are tested and how far each one is.">
      {on !== null && (
        <SwitchRow
          title="Show switches still in beta"
          description="Switches AI Juggler is still testing with these tools. Each one is marked Beta and says what it carries; some parts may not carry yet."
          checked={on}
          note={note}
          testId="switching-beta"
          onChange={(next) => {
            setNote(null);
            void appSettingSet(BETA_SWITCHES_KEY, next ? "1" : "").then(
              () => {
                setOn(next);
                setNote(next ? "Switches still in beta are listed, marked Beta." : "Only tested switches are listed.");
              },
              (cause) => setNote(errorText(cause)),
            );
          }}
        />
      )}
    </PaneSection>
  );
}
