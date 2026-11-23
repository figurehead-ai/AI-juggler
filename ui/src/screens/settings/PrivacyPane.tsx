/**
 * AJ-E2110 — Settings › Privacy: what AI Juggler reads to know what's
 * open and for Last messages, and what it keeps to be quick (the local
 * cache, ADR-0023), in plain words; usage statistics, crash
 * reports and Search the words of every session (AJ-E1105, AJ-E1538);
 * then every line of what it reads, changes and sends, each naming the
 * tests that check it (0.1.5: the Privacy page's lines, in one place).
 */
import { useEffect, useState, type JSX } from "react";
import { Button } from "../../components/primitives";
import { PrivacyCard } from "../../components/PrivacyCard";
import { PrivacyClaims } from "../PrivacyScreen";
import { Pane, PaneSection, SettingRow, SwitchRow } from "./parts";
import { onRemoteChanged, remoteAbout, remoteCheckSet } from "../../lib/ipc/remote";
import { errorText } from "../../lib/bridge";

const openSection = (section: string) => window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section } }));

/** AJ-E1548 #15 (AJ-E2023 Req 3, D-129): the configuration check, on by
 * default, with what it fetches and when. */
function ConfigCheckRow(): JSX.Element {
  const [on, setOn] = useState<boolean | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    const read = () => void remoteAbout().then((about) => live && setOn(about.checking), () => undefined);
    read();
    const stop = onRemoteChanged(read);
    return () => {
      live = false;
      stop();
    };
  }, []);
  return (
    <SwitchRow
      title="Check for configuration updates"
      description="At most once a day, AI Juggler downloads its signed configuration file: which tool versions and models work here, and short notices. The download carries nothing about you, and a file that doesn't verify is never used."
      checked={on ?? true}
      disabled={on === null}
      onChange={(next) => {
        setOn(next);
        setFailed(null);
        remoteCheckSet(next).catch((cause: unknown) => {
          setOn(!next);
          setFailed(errorText(cause));
        });
      }}
      note={failed}
      noteTone="warning"
      testId="privacy-config-check"
    />
  );
}

export function PrivacyPane({ anchor }: { anchor?: string }): JSX.Element {
  return (
    <Pane id="privacy" title="Privacy" anchor={anchor}>
      <PaneSection
        id="reads"
        title="What AI Juggler reads"
        footer="All of it on this computer. Every line below names the tests that check it."
      >
        <SettingRow
          title="To know what's open"
          description="Each tool's own session files, and the names of running programs and the apps that started them. Never a program's environment, where the Claude app keeps a sign-in token."
          testId="privacy-reads-open"
        />
        <SettingRow
          title="For Last messages"
          description="The end of a session's transcript, read while its page is open, held in memory until a minute after it closes, and never written down. Off with one switch."
          testId="privacy-reads-last"
          control={
            <Button size="sm" variant="ghost" onClick={() => openSection("last-messages")} data-testid="privacy-last-messages-switch">
              Last messages…
            </Button>
          }
        />
        <SettingRow
          title="What it keeps to be quick"
          description="A cache on this computer, never sent: subagent lists (their prompts and results only while Last messages are on), app icons and the search index. Clear it or turn it off in Local cache."
          testId="privacy-reads-cache"
          control={
            <Button size="sm" variant="ghost" onClick={() => openSection("local-cache")} data-testid="privacy-local-cache">
              Local cache…
            </Button>
          }
        />
      </PaneSection>
      <PaneSection id="config" title="Configuration updates" footer="Off, AI Juggler uses the settings it shipped with.">
        <ConfigCheckRow />
      </PaneSection>
      <PaneSection
        id="stats"
        title="Usage statistics, crash reports and search"
        footer="Usage statistics and crash reports are off until you choose."
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
