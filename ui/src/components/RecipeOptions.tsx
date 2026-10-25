/**
 * One launch setting's options, behind its "Options" disclosure: whether
 * its crashed sessions are restarted (AJ-E1509; its own choice, or as in
 * Settings › Sessions), and its environment overrides.
 */
import { useEffect, useState, type JSX } from "react";
import { EnvOverlayEditor } from "./EnvOverlayEditor";
import { errorText, expertSetupInfo, type LaunchRecipeInfo } from "../lib/bridge";
import { sessionsRestartChoiceSet } from "../lib/ipc/sessions";

type Choice = "settings" | "restart" | "stay";

const choiceOf = (restart: boolean | null): Choice => (restart === null ? "settings" : restart ? "restart" : "stay");

/** AJ-E1509: what happens when one of its sessions crashes. */
function RestartChoice({ recipe }: { recipe: LaunchRecipeInfo }): JSX.Element {
  const [choice, setChoice] = useState<Choice>(choiceOf(recipe.restart_on_crash));
  const [error, setError] = useState<string | null>(null);
  const id = `recipe-restart-${recipe.id}`;
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs" data-testid="recipe-restart">
      <label htmlFor={id} className="font-medium text-ink">
        If a session crashes
      </label>
      <select
        id={id}
        value={choice}
        onChange={(event) => {
          const next = event.target.value as Choice;
          const before = choice;
          setChoice(next);
          setError(null);
          void sessionsRestartChoiceSet(recipe.id, next === "settings" ? null : next === "restart").catch((cause) => {
            setChoice(before);
            setError(errorText(cause));
          });
        }}
        className="rounded-lg border border-hairline bg-surface-2 px-2 py-1 text-xs text-ink"
      >
        <option value="settings">As in Settings</option>
        <option value="restart">Restart it</option>
        <option value="stay">Leave it ended</option>
      </select>
      {error && (
        <span className="text-label text-danger" role="alert">
          {error}
        </span>
      )}
    </div>
  );
}

/** The env-override editor for one Launch settings, with the
 * Provider's protected (isolation) keys fetched from the backend —
 * never a hard-coded Provider list in the UI. */
function RecipeEnvEditor({ provider, accountId, recipeId }: { provider: string; accountId: string; recipeId: string }) {
  const [protectedKeys, setProtectedKeys] = useState<string[] | null>(null);
  useEffect(() => {
    let alive = true;
    void expertSetupInfo(provider, accountId)
      .then((info) => {
        if (alive) setProtectedKeys(info.protected_env_keys);
      })
      .catch(() => {
        // The backend still refuses protected keys on Save.
        if (alive) setProtectedKeys([]);
      });
    return () => {
      alive = false;
    };
  }, [provider, accountId]);
  if (protectedKeys === null) {
    return <p className="text-label text-muted">Loading…</p>;
  }
  return <EnvOverlayEditor recipeId={recipeId} protectedKeys={protectedKeys} />;
}

export function RecipeOptions({ provider, accountId, recipe }: { provider: string; accountId: string; recipe: LaunchRecipeInfo }): JSX.Element {
  return (
    <div className="flex flex-col gap-3">
      <RestartChoice recipe={recipe} />
      <RecipeEnvEditor provider={provider} accountId={accountId} recipeId={recipe.id} />
    </div>
  );
}
