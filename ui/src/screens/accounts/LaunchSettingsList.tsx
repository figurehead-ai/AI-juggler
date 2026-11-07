/**
 * A CLI App's launch settings: each set with its model, folder and
 * options, Launch, Delete (asks first, refused while one of its sessions
 * runs, AJ-E2907) and a new set. On the Accounts screen's profile row
 * (opened from its menu) and, since 0.4.1, as a CLI App page's own
 * "Launch settings" tab (AJ-E3204 R2).
 */
import { useState, type JSX } from "react";
import { Button, Chip } from "../../components/primitives";
import { ActionIcon } from "../../components/ActionIcon";
import { IconTerminal } from "../../components/icons";
import { ExpertSetup } from "../../components/ExpertSetup";
import { RecipeOptions } from "../../components/RecipeOptions";
import { modelName } from "../../components/charts/colors";
import { launchSettingsName } from "../../lib/account-words";
import { signedIn } from "../../lib/bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";
import { DeleteLaunchSettingsModal } from "./DeleteLaunchSettings";

export interface LaunchSettingsProps {
  /** The App's folder (a CLI App's sign-in). */
  account: AccountInfo;
  /** Its launch settings. */
  recipes: LaunchRecipeInfo[];
  /** AJ-E2907 R3: AI Juggler's live sessions per set (Delete waits). */
  liveByRecipe?: Record<string, number>;
  onLaunch: (recipe: LaunchRecipeInfo) => void;
  onDeleteRecipe: (id: string) => Promise<void>;
  onNewRecipe: () => void;
  /** Its "Done", where it opens in place (the profile row). */
  onClose?: () => void;
  /** Which set's options are open, when the caller keeps it. */
  envFor?: string | null;
  setEnvFor?: (id: string | null) => void;
}

/** A folder, short: home as ~, the middle folded ("~/…/studio-main"); the full path is on hover. */
export function shortPath(path: string): string {
  const home = path.replace(/^\/Users\/[^/]+/, "~");
  const parts = home.split("/").filter(Boolean);
  if (parts.length <= 3) return home;
  return `${parts[0] === "~" ? "~" : `/${parts[0]}`}/…/${parts[parts.length - 1]}`;
}

/** AJ-E2907 R3: why Delete is off while sessions started with them run
 * (deep/11 §10, D-10's words), or null. */
export function liveWords(live: number): string | null {
  if (live === 0) return null;
  return `${live === 1 ? "1 live session uses" : `${live} live sessions use`} these launch settings — stop ${live === 1 ? "it" : "them"} first.`;
}

/** A profile's launch settings: each with its model, folder and options,
 * Launch, and a new set. */
export function LaunchSettingsList(props: LaunchSettingsProps): JSX.Element {
  // D-60: deleting asks first (one click used to delete them and their sessions).
  const [deleting, setDeleting] = useState<LaunchRecipeInfo | null>(null);
  const [ownEnvFor, setOwnEnvFor] = useState<string | null>(null);
  const { account, recipes } = props;
  const envFor = props.setEnvFor ? (props.envFor ?? null) : ownEnvFor;
  const setEnvFor = props.setEnvFor ?? setOwnEnvFor;
  return (
    <div className="mx-5 mb-3 divide-y divide-hairline rounded-xl border border-hairline bg-surface-2/40" data-testid="profile-launch-settings">
      <div className="flex items-center justify-between px-4 py-2">
        <span className="text-meta font-semibold text-ink">Launch settings <span className="font-normal text-muted">· used when this app starts</span></span>
        <span className="flex items-center gap-1">
          <button type="button" className="rounded-md px-2 py-1 text-label text-muted hover:bg-surface hover:text-ink" onClick={props.onNewRecipe}>
            ＋ New settings set
          </button>
          {props.onClose && (
            <button type="button" className="rounded-md px-2 py-1 text-label text-muted hover:bg-surface hover:text-ink" onClick={props.onClose} title="Close launch settings">
              Done
            </button>
          )}
        </span>
      </div>
      {recipes.length === 0 ? (
        <div className="px-4 py-3 text-meta text-muted">No launch settings yet — the default applies when you start.</div>
      ) : (
        recipes.map((recipe) => (
          <div key={recipe.id} className="group flex flex-wrap items-center gap-3 px-4 py-2.5">
            <span className="text-body">{launchSettingsName(recipe.name)}</span>
            {/* The model as people say it ("Opus"; the 0.4 review, 71). */}
            <Chip tone="neutral">{recipe.model ? modelName(recipe.model) : "default model"}</Chip>
            {recipe.default_cwd && <span className="truncate font-mono text-label text-muted" title={recipe.default_cwd}>{shortPath(recipe.default_cwd)}</span>}
            <button
              type="button"
              aria-expanded={envFor === recipe.id}
              onClick={() => setEnvFor(envFor === recipe.id ? null : recipe.id)}
              className="rounded-md px-2 py-1 text-label text-muted hover:bg-surface hover:text-ink"
              data-testid="recipe-env-toggle"
            >
              Options {envFor === recipe.id ? "▾" : "▸"}
            </button>
            <div className="ml-auto flex items-center gap-1">
              <Button
                size="sm"
                variant="ghost"
                title={liveWords(props.liveByRecipe?.[recipe.id] ?? 0) ?? "Delete launch settings"}
                aria-label={`Delete ${launchSettingsName(recipe.name)}`}
                disabled={(props.liveByRecipe?.[recipe.id] ?? 0) > 0}
                onClick={() => setDeleting(recipe)}
                data-testid="recipe-delete"
              >
                <ActionIcon name="delete" size={13} />
              </Button>
              {/* One filled button per view (principle 9): a list's own
                  actions are secondary beside the page's New session. */}
              <Button size="sm" variant="secondary" disabled={!signedIn(account.login_state)} onClick={() => props.onLaunch(recipe)}>
                <IconTerminal width={13} height={13} />
                Launch
              </Button>
            </div>
            {envFor === recipe.id && (
              <div className="basis-full pt-1">
                <RecipeOptions provider={account.provider} accountId={account.id} recipe={recipe} />
              </div>
            )}
          </div>
        ))
      )}
      <div className="px-4 py-3">
        <ExpertSetup provider={account.provider} accounts={[{ id: account.id, label: account.label }]} surface="accounts" showMetricsExport />
      </div>
      {deleting && <DeleteLaunchSettingsModal recipe={deleting} onClose={() => setDeleting(null)} onDelete={props.onDeleteRecipe} />}
    </div>
  );
}
