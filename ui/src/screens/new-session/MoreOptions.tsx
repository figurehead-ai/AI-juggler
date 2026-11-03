/**
 * AJ-E3207 — the rare options behind one fold, its label saying what
 * they're set to now ("More options · Deep work · opus · runs as its
 * launch settings say"), so nothing hides silently: the App's launch
 * settings when it has more than one, how it runs, which Mac (only with
 * paired Macs), a saved handover to start from, this Mac's room when it's
 * fine, and the expert setup. Nothing here is remembered for next time.
 */
import type { JSX } from "react";
import { Disclosure, Picker } from "../../components/controls";
import { ExpertSetup } from "../../components/ExpertSetup";
import { MachineAdviceLine } from "../../components/MachineAdvice";
import { PackPicker, type PackChoice } from "../../components/OpenPack";
import { MachinePicker, type Where } from "../../components/RemoteLaunch";
import type { RuntimeChoice } from "../../components/RuntimePick";
import type { AccountInfo, LaunchRecipeInfo } from "../../lib/bridge";
import type { MachineAdviceInfo } from "../../lib/machine";
import { useSuggested } from "../../lib/suggested";

/** How it runs, as the fold's label says it. */
export function runtimeWords(runtime: RuntimeChoice): string {
  if (runtime === "terminal") return "runs in its terminal";
  if (runtime === "driven") return "runs without a terminal";
  return "runs as its launch settings say";
}

/** The fold's label: its current values, the ones worth saying. */
export function moreOptionsWords({ recipe, settingsCount, runtime, machine }: { recipe: LaunchRecipeInfo | null; settingsCount: number; runtime: RuntimeChoice; machine: string | null }): string {
  const parts = [settingsCount > 1 ? recipe?.name : null, recipe?.model, runtimeWords(runtime), machine];
  return parts.filter(Boolean).join(" · ");
}

export function MoreOptions({
  recipe,
  settings,
  onSettings,
  runtime,
  runtimeElement,
  where,
  pack,
  advice,
  sessionName,
  folders,
  onManageLaunchSettings,
}: {
  recipe: LaunchRecipeInfo | null;
  /** The App's launch settings (one or more). */
  settings: readonly LaunchRecipeInfo[];
  onSettings: (recipeId: string) => void;
  runtime: RuntimeChoice;
  runtimeElement: JSX.Element;
  where: Where;
  pack: PackChoice;
  advice: MachineAdviceInfo | undefined;
  sessionName: (id: string) => string | null;
  folders: readonly AccountInfo[];
  onManageLaunchSettings?: () => void;
}): JSX.Element {
  const machine = where.machines.length > 0 ? "on this computer" : null;
  const suggested = useSuggested("launch_settings", settings.map((row) => row.id));
  return (
    <Disclosure
      testId="new-session-more"
      className="mt-4"
      summary={
        <span className="flex min-w-0 flex-wrap items-baseline gap-x-1.5">
          <span className="font-medium text-ink">More options</span>
          <span className="text-muted" data-testid="new-session-more-values">
            · {moreOptionsWords({ recipe, settingsCount: settings.length, runtime, machine })}
          </span>
        </span>
      }
    >
      <div className="mt-3 flex flex-col gap-3 border-l border-hairline pl-4">
        {settings.length > 1 && recipe && (
          <div className="flex items-center gap-2 text-xs text-muted" data-testid="new-session-settings">
            <span aria-hidden="true">Launch settings</span>
            <Picker
              ariaLabel="Launch settings"
              groups={suggested}
              size="sm"
              value={recipe.id}
              onChange={onSettings}
              options={settings.map((row) => ({ value: row.id, label: row.name, hint: row.model ?? undefined }))}
            />
          </div>
        )}
        {runtimeElement}
        <MachinePicker where={where} />
        {!pack.opened && <PackPicker pack={pack} />}
        {advice?.verdict === "ok" && <MachineAdviceLine advice={advice} nameOf={sessionName} />}
        {recipe && (
          <ExpertSetup
            provider={folders.find((entry) => entry.id === recipe.account_id)?.provider ?? recipe.provider}
            accounts={[{ id: recipe.account_id, label: recipe.account_label }]}
            surface="launch"
            onManageLaunchSettings={onManageLaunchSettings}
          />
        )}
      </div>
    </Disclosure>
  );
}
