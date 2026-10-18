import { useEffect, useId, useState } from "react";
import { Button } from "./primitives";
import { EnvOverlayEditor } from "./EnvOverlayEditor";
import {
  errorText,
  expertReveal,
  expertSetupInfo,
  funnelExport,
  launchRecipesList,
  type ExpertSetupInfo,
  type LaunchRecipeInfo,
} from "../lib/bridge";
import { funnelEvent } from "../lib/funnel";

/**
 * AJ-E309 Req.5: the [Expert setup…] hatch every flow exposes —
 * collapsed by default so the guided path stays simple, and complete
 * when opened: the raw state folder (read-only + Reveal in Finder),
 * the copyable shell line that reproduces the setup, the environment
 * overrides of the account's Launch settings, a way to manage Launch
 * settings, [Import from folder…] where the flow can import, and the
 * local onboarding-metrics export.
 *
 * Every path is resolved by the backend from AI Juggler's own state;
 * this component never sends a path anywhere.
 */
export function ExpertSetup({
  provider,
  accounts = [],
  surface,
  onImportFolder,
  onManageLaunchSettings,
  showMetricsExport = false,
}: {
  provider: string;
  /** The Account(s) this flow is about; empty before one exists. With
   * several (a set accept), the panel offers a picker. */
  accounts?: { id: string; label: string }[];
  /** Where the hatch lives (the `expert_opened{surface}` field). */
  surface: "wizard" | "launch" | "new-project" | "accounts";
  onImportFolder?: () => void;
  onManageLaunchSettings?: () => void;
  showMetricsExport?: boolean;
}): JSX.Element {
  const panelId = useId();
  const [open, setOpen] = useState(false);
  const [announced, setAnnounced] = useState(false);
  const [accountId, setAccountId] = useState<string | null>(accounts[0]?.id ?? null);
  const [recipes, setRecipes] = useState<LaunchRecipeInfo[]>([]);
  const [recipeId, setRecipeId] = useState<string | null>(null);
  const [info, setInfo] = useState<ExpertSetupInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [exported, setExported] = useState<string | null>(null);
  const [envRevision, setEnvRevision] = useState(0);

  // Keep the selected Account valid as the list changes.
  const accountKey = accounts.map((account) => account.id).join("|");
  useEffect(() => {
    if (accountId !== null && accounts.some((account) => account.id === accountId)) return;
    setAccountId(accounts[0]?.id ?? null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [accountKey]);

  // The Account's Launch settings (the env editor's targets).
  useEffect(() => {
    if (!open || accountId === null) {
      setRecipes([]);
      setRecipeId(null);
      return;
    }
    let alive = true;
    void launchRecipesList()
      .then((rows) => {
        if (!alive) return;
        const mine = rows.filter((row) => row.account_id === accountId);
        setRecipes(mine);
        setRecipeId((current) =>
          current !== null && mine.some((row) => row.id === current) ? current : (mine[0]?.id ?? null),
        );
      })
      .catch(() => {
        if (alive) setRecipes([]);
      });
    return () => {
      alive = false;
    };
  }, [open, accountId]);

  useEffect(() => {
    if (!open) return;
    let alive = true;
    setError(null);
    void expertSetupInfo(provider, accountId, accountId !== null ? recipeId : null)
      .then((next) => {
        if (alive) setInfo(next);
      })
      .catch((cause) => {
        if (alive) setError(errorText(cause));
      });
    return () => {
      alive = false;
    };
  }, [open, provider, accountId, recipeId, envRevision]);

  const toggle = () => {
    setOpen((current) => !current);
    if (!announced) {
      setAnnounced(true);
      funnelEvent("expert_opened", { surface });
    }
  };

  const exportMetrics = async () => {
    const markdown = await funnelExport().catch(() => null);
    if (markdown === null) return;
    const blob = new Blob([markdown], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `ai-juggler-onboarding-funnel-${new Date().toISOString().slice(0, 10)}.md`;
    anchor.click();
    URL.revokeObjectURL(url);
    setExported("Saved the local report — it never leaves this Mac unless you share it.");
  };

  const recipe = recipes.find((entry) => entry.id === recipeId) ?? null;

  return (
    <div className="flex flex-col" data-testid="expert-setup">
      <button
        type="button"
        aria-expanded={open}
        aria-controls={panelId}
        onClick={toggle}
        className="self-start rounded text-[11px] text-muted underline-offset-2 outline-none hover:text-ink hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
        data-testid="expert-hatch"
      >
        Expert setup… {open ? "▾" : "▸"}
      </button>
      {open && (
        <div
          id={panelId}
          className="mt-2 flex flex-col gap-3 rounded-xl border border-hairline bg-canvas px-4 py-3 text-[11px]"
          data-testid="expert-panel"
        >
          {error && (
            <p className="text-danger" role="alert">
              {error}
            </p>
          )}
          {accounts.length > 1 && (
            <label className="flex items-center gap-2 text-muted">
              Account
              <select
                value={accountId ?? ""}
                onChange={(event) => setAccountId(event.target.value)}
                className="rounded-lg border border-hairline bg-surface px-2 py-1 text-[11px] text-ink"
                data-testid="expert-account"
              >
                {accounts.map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.label}
                  </option>
                ))}
              </select>
            </label>
          )}
          {info === null && error === null && <p className="text-muted">Loading…</p>}
          {info !== null && (
            <>
              <div className="flex flex-col gap-1.5">
                <label className="flex flex-col gap-1">
                  <span className="font-medium text-ink">State folder (read-only)</span>
                  <input
                    readOnly
                    value={info.state_dir}
                    onFocus={(event) => event.currentTarget.select()}
                    className="w-full rounded-lg border border-hairline bg-surface px-2 py-1 font-mono text-[11px] text-ink outline-none focus:border-accent/50"
                    data-testid="expert-state-dir"
                  />
                </label>
                <p className="leading-relaxed text-muted">
                  {info.note}
                  {!info.exists && " It doesn't exist yet."}
                </p>
                <Button
                  size="sm"
                  variant="secondary"
                  className="self-start"
                  onClick={() =>
                    void expertReveal(provider, accountId).catch((cause) =>
                      setError(errorText(cause)),
                    )
                  }
                  data-testid="expert-reveal"
                >
                  Reveal in Finder
                </Button>
              </div>

              <div className="flex flex-col gap-1.5">
                <span className="font-medium text-ink">Reproduce in your shell</span>
                {info.shell_snippet !== null ? (
                  <>
                    <code
                      className="block break-all rounded-lg border border-hairline bg-surface px-2.5 py-2 font-mono text-[11px] leading-relaxed text-ink"
                      data-testid="expert-snippet"
                    >
                      {info.shell_snippet}
                    </code>
                    <Button
                      size="sm"
                      variant="secondary"
                      className="self-start"
                      onClick={async () => {
                        await navigator.clipboard?.writeText(info.shell_snippet ?? "").catch(() => null);
                        setCopied(true);
                        window.setTimeout(() => setCopied(false), 1500);
                      }}
                      data-testid="expert-copy-snippet"
                    >
                      {copied ? "Copied" : "Copy shell line"}
                    </Button>
                  </>
                ) : (
                  <p className="leading-relaxed text-muted" data-testid="expert-snippet-pending">
                    The shell line appears once the account exists — it points the vendor's CLI at
                    that account's own folder.
                  </p>
                )}
              </div>

              <div className="flex flex-col gap-1.5">
                <span className="font-medium text-ink">Environment overrides</span>
                {accountId !== null && recipe !== null ? (
                  <>
                    {recipes.length > 1 && (
                      <label className="flex items-center gap-2 text-muted">
                        Launch settings
                        <select
                          value={recipe.id}
                          onChange={(event) => setRecipeId(event.target.value)}
                          className="rounded-lg border border-hairline bg-surface px-2 py-1 text-[11px] text-ink"
                        >
                          {recipes.map((entry) => (
                            <option key={entry.id} value={entry.id}>
                              {entry.name === "Main" ? "Default" : entry.name}
                            </option>
                          ))}
                        </select>
                      </label>
                    )}
                    <p className="leading-relaxed text-muted">
                      Extra variables set when{" "}
                      {recipe.name === "Main" ? "the default Launch settings" : `“${recipe.name}”`}{" "}
                      start an Instance.
                    </p>
                    <EnvOverlayEditor
                      recipeId={recipe.id}
                      protectedKeys={info.protected_env_keys}
                      onSaved={() => setEnvRevision((revision) => revision + 1)}
                    />
                  </>
                ) : accountId !== null ? (
                  <p className="leading-relaxed text-muted" data-testid="expert-env-pending">
                    This account has no Launch settings yet — add some in Accounts and their
                    overrides edit here.
                  </p>
                ) : (
                  <p className="leading-relaxed text-muted" data-testid="expert-env-pending">
                    Overrides belong to an account's Launch settings — edit them here once the
                    account exists, or anytime in Accounts.
                  </p>
                )}
                {onManageLaunchSettings && (
                  <Button
                    size="sm"
                    variant="secondary"
                    className="self-start"
                    onClick={onManageLaunchSettings}
                    data-testid="expert-manage-launch-settings"
                  >
                    Manage Launch settings →
                  </Button>
                )}
              </div>

              {onImportFolder && (
                <div className="flex flex-col gap-1.5">
                  <span className="font-medium text-ink">Import a specific folder</span>
                  <Button
                    size="sm"
                    variant="secondary"
                    className="self-start"
                    onClick={onImportFolder}
                    data-testid="expert-import-folder"
                  >
                    Import from folder…
                  </Button>
                </div>
              )}

              {showMetricsExport && (
                <div className="flex flex-col gap-1.5">
                  <span className="font-medium text-ink">Onboarding metrics</span>
                  <p className="leading-relaxed text-muted">
                    Counts of setup steps, stored only on this Mac — never paths, names, or
                    credentials.
                  </p>
                  <Button
                    size="sm"
                    variant="secondary"
                    className="self-start"
                    onClick={() => void exportMetrics()}
                    data-testid="expert-export-metrics"
                  >
                    Export report…
                  </Button>
                  {exported && (
                    <span className="text-muted" role="status">
                      {exported}
                    </span>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
