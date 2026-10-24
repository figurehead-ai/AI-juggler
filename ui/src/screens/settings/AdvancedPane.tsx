/**
 * AJ-E2110 — Settings › Advanced: Cleanup (D-08), Data (storage,
 * retention, backup and restore), Diagnostics, Figurehead & Cloud (the
 * optional Figurehead account, D-18) and what to do before uninstalling.
 */
import type { JSX } from "react";
import { BackupRestoreCard } from "../../components/BackupRestoreCard";
import { DiagnosticsCard } from "../../components/DiagnosticsCard";
import { FigureheadCloudCard } from "../../components/FigureheadCloudCard";
import { CleanupSettingsCard } from "../../components/SettingsSections";
import { StorageRetentionCard } from "../../components/StorageRetentionCard";
import { UninstallCard } from "../../components/UninstallCard";
import { useAccounts } from "./AccountsPane";
import { Pane, PaneSection } from "./parts";

export function AdvancedPane({ anchor }: { anchor?: string }): JSX.Element {
  const accounts = useAccounts();
  return (
    <Pane id="advanced" title="Advanced" anchor={anchor}>
      <PaneSection
        id="cleanup"
        title="Cleanup"
        footer="What happens to processes left behind. Nothing is stopped by itself unless you turn that on."
        card
      >
        <CleanupSettingsCard accounts={accounts} />
      </PaneSection>
      <PaneSection id="data" title="Data: storage & retention" footer="How long AI Juggler keeps its own records on this Mac." card>
        <StorageRetentionCard />
      </PaneSection>
      <PaneSection id="backup" title="Data: backup & restore" footer="A file of AI Juggler's settings and history, to keep or to restore on another Mac." card>
        <BackupRestoreCard />
      </PaneSection>
      <PaneSection id="diagnostics" title="Diagnostics" footer="A scrubbed zip you look through and attach to a report yourself." card>
        <DiagnosticsCard />
      </PaneSection>
      <PaneSection
        id="account"
        title="Figurehead & Cloud"
        footer="Optional. Everything in AI Juggler works without it; each Cloud feature says what it sends."
        card
      >
        <FigureheadCloudCard />
      </PaneSection>
      <PaneSection id="uninstall" title="Before you uninstall" footer="What AI Juggler can take out of your tools' setups first, and what's left for you to remove." card>
        <UninstallCard />
      </PaneSection>
    </Pane>
  );
}
