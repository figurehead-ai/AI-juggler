/**
 * AJ-E3204 R2: a CLI App's page has its launch settings as a tab of its
 * own: the sets it starts with, each with its model, folder and options,
 * Launch (which opens the new session's page), Delete (asks first, waits
 * for its live sessions) and a new set. The same list the Accounts
 * screen's profile row shows (`LaunchSettingsList`).
 */
import { useCallback, useEffect, useState, type JSX } from "react";
import { LaunchSettingsList } from "../../screens/accounts/LaunchSettingsList";
import { liveByRecipe } from "../../screens/accounts/AccountCard";
import { NewProfileModal } from "../../screens/Profiles";
import { launchRecipesList, launchSession } from "../../lib/ipc/launch";
import { deleteProfile } from "../../lib/ipc/manage";
import { errorText } from "../../lib/bridge";
import { showToast } from "../../components/Toast";
import { useActionHost } from "../../components/ItemMenu";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";

export function AppLaunchSettings({ folder, sessions }: { folder: AccountInfo; sessions: InstanceInfo[] }): JSX.Element {
  const host = useActionHost();
  const [recipes, setRecipes] = useState<LaunchRecipeInfo[] | null>(null);
  const [adding, setAdding] = useState(false);
  const read = useCallback(() => {
    launchRecipesList().then(
      (rows) => setRecipes(rows.filter((row) => row.account_id === folder.id)),
      (cause: unknown) => (setRecipes([]), showToast({ id: "launch-settings", tone: "error", message: `Couldn't read its launch settings: ${errorText(cause)}` })),
    );
  }, [folder.id]);
  useEffect(read, [read]);
  if (recipes === null) {
    return (
      <p className="p-4 text-meta text-ink-2" data-testid="app-launch-settings-loading">
        Reading its launch settings…
      </p>
    );
  }
  return (
    <div className="-mx-5" data-testid="app-launch-settings">
      <LaunchSettingsList
        account={folder}
        recipes={recipes}
        liveByRecipe={liveByRecipe(sessions.filter((session) => session.account_id === folder.id))}
        onLaunch={(recipe) => {
          launchSession(recipe.id, null).then(
            (result) => {
              if (result) host.openSessionById?.(result.session_id);
              else showToast({ id: "launch-settings", message: "Preview mode: no backend connected — nothing launched." });
            },
            (cause: unknown) => showToast({ id: "launch-settings", tone: "error", message: `Couldn't launch it: ${errorText(cause)}` }),
          );
        }}
        onDeleteRecipe={async (id) => {
          await deleteProfile(id);
          read();
        }}
        onNewRecipe={() => setAdding(true)}
      />
      {adding && <NewProfileModal account={folder} onClose={() => setAdding(false)} onCreated={() => (setAdding(false), read())} />}
    </div>
  );
}
