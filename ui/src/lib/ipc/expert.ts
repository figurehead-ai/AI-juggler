/**
 * Expert setup: the environment a launch recipe adds.
 */
import { invoke, liveBackend as hasBackend } from "./transport";
import type { ExpertSetupInfo } from "../../types/generated/ExpertSetupInfo";

/** AJ-E309 Req.5: the Expert-setup facts for a Provider (no account
 * yet) or one Account (+ optionally one of its Launch settings). */
export async function expertSetupInfo(
  provider: string,
  accountId: string | null,
  recipeId: string | null = null,
): Promise<ExpertSetupInfo> {
  if (!hasBackend) {
    const seam = (window as unknown as { __AIJ_EXPERT__?: Partial<ExpertSetupInfo> })
      .__AIJ_EXPERT__;
    const root = `/Users/you/Library/Application Support/AI Juggler/accounts/${provider}`;
    const dir = accountId === null ? root : `${root}/${accountId}`;
    return {
      provider,
      account_id: accountId,
      state_dir: dir,
      exists: accountId !== null,
      shell_snippet: accountId === null ? null : `CLAUDE_CONFIG_DIR='${dir}' claude`,
      protected_env_keys: ["CLAUDE_CONFIG_DIR", "AI_JUGGLER_CLAUDE_BIN"],
      note:
        accountId === null
          ? "Each new app gets its own isolated folder in here — the shell line appears once the app exists."
          : "This account keeps all of its state here — isolated from your other accounts and from your own install.",
      ...seam,
    };
  }
  return invoke<ExpertSetupInfo>("expert_setup_info", { provider, accountId, recipeId });
}

/** AJ-E309 Req.5: [Reveal in Finder] — the backend resolves the
 * managed folder itself (never a caller-supplied path). */
export async function expertReveal(provider: string, accountId: string | null): Promise<void> {
  if (!hasBackend) {
    return;
  }
  return invoke<void>("expert_reveal", { provider, accountId });
}

/** AJ-E309 Req.5: one Launch settings' environment overrides. */
export async function launchRecipeEnvGet(recipeId: string): Promise<Record<string, string>> {
  if (!hasBackend) {
    const store = (window as unknown as { __AIJ_RECIPE_ENV__?: Record<string, Record<string, string>> })
      .__AIJ_RECIPE_ENV__;
    return { ...(store?.[recipeId] ?? {}) };
  }
  return invoke<Record<string, string>>("launch_recipe_env_get", { recipeId });
}

/** AJ-E309 Req.5: replaces one Launch settings' environment overrides
 * (validated server-side: isolation and credential-shaped keys are
 * refused with a coded `validation` error). */
export async function launchRecipeEnvSet(
  recipeId: string,
  env: Record<string, string>,
): Promise<Record<string, string>> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_RECIPE_ENV__?: Record<string, Record<string, string>>;
      __AIJ_RECIPE_ENV_REJECT__?: string;
    };
    if (seams.__AIJ_RECIPE_ENV_REJECT__) {
      throw { code: "validation", retryable: false, user_message: seams.__AIJ_RECIPE_ENV_REJECT__ };
    }
    seams.__AIJ_RECIPE_ENV__ = { ...(seams.__AIJ_RECIPE_ENV__ ?? {}), [recipeId]: { ...env } };
    return { ...env };
  }
  return invoke<Record<string, string>>("launch_recipe_env_set", { recipeId, env });
}
