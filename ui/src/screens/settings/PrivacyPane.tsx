/**
 * AJ-E2110 — Settings › Privacy: what AI Juggler reads to know what's
 * open and for Last messages, in plain words; usage statistics, crash
 * reports and Search the words of every session (AJ-E1105, AJ-E1538);
 * then every line of what it reads, changes and sends, each naming the
 * tests that check it (0.1.5: the Privacy page's lines, in one place).
 */
import type { JSX } from "react";
import { Button } from "../../components/primitives";
import { PrivacyCard } from "../../components/PrivacyCard";
import { PrivacyClaims } from "../PrivacyScreen";
import { Pane, PaneSection, SettingRow } from "./parts";

const openLastMessages = () => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "last-messages" } }));

export function PrivacyPane({ anchor }: { anchor?: string }): JSX.Element {
  return (
    <Pane id="privacy" title="Privacy" anchor={anchor}>
      <PaneSection
        id="reads"
        title="What AI Juggler reads"
        footer="All of it on this Mac. Every line below names the tests that check it."
      >
        <SettingRow
          title="To know what's open"
          description="Each app's own session files, and the names of running programs and the apps that started them. Never a program's environment, where the Claude app keeps a sign-in token."
          testId="privacy-reads-open"
        />
        <SettingRow
          title="For Last messages"
          description="The end of a session's transcript, read while its page is open. Kept nowhere, and off with one switch."
          testId="privacy-reads-last"
          control={
            <Button size="sm" variant="ghost" onClick={openLastMessages} data-testid="privacy-last-messages-switch">
              Last messages…
            </Button>
          }
        />
      </PaneSection>
      <PaneSection
        id="stats"
        title="Usage statistics, crash reports and search"
        footer="Usage statistics are off until you choose. Crash reports stay on this Mac."
        card
      >
        <PrivacyCard />
      </PaneSection>
      {/* Everything else it reads, changes and sends, and what it never
          does: here, in one place (More's Privacy opens this pane). */}
      <PrivacyClaims />
    </Pane>
  );
}
