/**
 * AJ-E2110 — Settings › Switching: how a context pack treats the keys
 * and tokens it finds when work moves to another account (D-14), and
 * (D-127) whether switches still in beta are listed.
 */
import type { JSX } from "react";
import { PackSecretsCard } from "../../components/PackSecretsCard";
import { BetaSwitchesSection } from "./BetaSwitchesSection";
import { useAccounts } from "./AccountsPane";
import { Pane, PaneSection } from "./parts";

export function SwitchingPane({ anchor }: { anchor?: string }): JSX.Element {
  const accounts = useAccounts();
  const providers = [...new Set(accounts.map((account) => account.provider))];
  return (
    <Pane id="switching" title="Switching" anchor={anchor}>
      <PaneSection
        id="packs"
        title="Handovers and keys"
        footer="A handover carries your work to another app. Keys and passwords in it are handled as chosen here."
        card
      >
        <PackSecretsCard providers={providers} />
      </PaneSection>
      <BetaSwitchesSection />
    </Pane>
  );
}
