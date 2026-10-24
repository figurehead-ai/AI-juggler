/**
 * AJ-E2110 — Settings › Privacy: what AI Juggler reads to know what's
 * open and for Last messages, in plain words, with the way to the
 * Privacy page (every line there names the tests that check it); then
 * usage statistics, crash reports and Search the words of every session
 * (AJ-E1105, AJ-E1538).
 */
import type { JSX } from "react";
import { Button } from "../../components/primitives";
import { PrivacyCard } from "../../components/PrivacyCard";
import { Pane, PaneSection, SettingRow } from "./parts";

const openPrivacyPage = () => window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id: "privacy" } }));
const openLastMessages = () => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "last-messages" } }));

export function PrivacyPane({ anchor }: { anchor?: string }): JSX.Element {
  return (
    <Pane id="privacy" title="Privacy" anchor={anchor}>
      <PaneSection
        id="reads"
        title="What AI Juggler reads"
        footer="All of it on this Mac. Every line on the Privacy page names the tests that check it."
      >
        <SettingRow
          title="To know what's open"
          description="Each app's own session files, and the names of running programs and the apps that started them. Never a program's environment, where the Claude app keeps a sign-in token."
          testId="privacy-reads-open"
        />
        <SettingRow
          title="For Last messages"
          description="The end of a session's transcript, read when you open its page. Kept nowhere, and off with one switch."
          testId="privacy-reads-last"
          control={
            <Button size="sm" variant="ghost" onClick={openLastMessages} data-testid="privacy-last-messages-switch">
              Last messages…
            </Button>
          }
        />
        <SettingRow
          title="Everything else it reads, changes and sends"
          description="And what it never does."
          control={
            <Button size="sm" variant="secondary" onClick={openPrivacyPage} data-testid="settings-privacy-page">
              Open the Privacy page
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
    </Pane>
  );
}
