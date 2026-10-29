/**
 * One launch setting's options, behind its "Options" disclosure: whether
 * its crashed sessions are restarted (AJ-E1509; its own choice, or as in
 * Settings › Sessions), and its environment overrides.
 */
import { useEffect, useState, type JSX } from "react";
import { EnvOverlayEditor } from "./EnvOverlayEditor";
import { Picker } from "./controls";
import { errorText, expertSetupInfo, type LaunchRecipeInfo } from "../lib/bridge";
import { sessionsRestartChoiceSet } from "../lib/ipc/sessions";

type Choice = "settings" | "restart" | "stay";

const choiceOf = (restart: boolean | null): Choice => (restart === null ? "settings" : restart ? "restart" : "stay");

/** AJ-E1509: what happens when one of its sessions crashes. */
function RestartChoice({ recipe }: { recipe: LaunchRecipeInfo }): JSX.Element {
  const [choice, setChoice] = useState<Choice>(choiceOf(recipe.restart_on_crash));
  const [error, setError] = useState<string | null>(null);
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs" data-testid="recipe-restart">
      <span aria-hidden="true" className="font-medium text-ink">
        If a session crashes
      </span>
      <Picker
        ariaLabel="If a session crashes"
        size="sm"
        value={choice}
        testId="recipe-restart-choice"
        onChange={(next) => {
          const before = choice;
          setChoice(next);
          setError(null);
          void sessionsRestartChoiceSet(recipe.id, next === "settings" ? null : next === "restart").catch((cause) => {
            setChoice(before);
            setError(errorText(cause));
          });
        }}
        options={[
          { value: "settings", label: "As in Settings" },
          { value: "restart", label: "Restart it" },
          { value: "stay", label: "Leave it ended" },
        ]}
      />
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
