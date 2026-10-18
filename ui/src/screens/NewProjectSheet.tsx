import { useEffect, useMemo, useRef, useState } from "react";
import { Button, Card, Chip } from "../components/primitives";
import { IconProjects, IconTerminal } from "../components/icons";
import { funnelStep } from "../lib/funnel";
import { ExpertSetup } from "../components/ExpertSetup";
import { OnboardingWizard } from "./OnboardingWizard";
import {
  launchRecipesList,
  launchSession,
  pickDirectory,
  type LaunchRecipeInfo,
  type ProjectInfo,
  type AccountInfo,
  type QuotaDisplayData,
  errorText,
  signedIn,
} from "../lib/bridge";

/**
 * AJ-E305 flow (b): a new project in ≤ 3 clicks and zero typed
 * paths. Step 1 — where (recent dirs as tiles, a native folder
 * picker, or a drop). Step 2 — who: the RECOMMENDED account
 * preselected (last-used on this dir, else most headroom, else most
 * recent), one [Change] away from the full list.
 *
 * No Account for the Provider detours INLINE into the add-account
 * flow (d) and returns to the pending launch on completion — the
 * directory never has to be re-picked.
 */

/** Headroom rank for the preselect rule: the engine's forecast
 * state, ordered from most to least headroom. */
const HEADROOM_RANK: Record<string, number> = {
  "wont-exhaust": 0,
  learning: 1,
  exhausts: 2,
  "no-headroom": 3,
};

/**
 * The preselect rule (Req.6), pure so tests pin it directly:
 * 1. last-used on this dir (most recent use first);
 * 2. else the account with the most headroom (engine forecast);
 * 3. else the first recipe (backend order = most recent).
 */
export function pickRecipe(
  recipes: LaunchRecipeInfo[],
  project: ProjectInfo | null,
  quotas: QuotaDisplayData[],
): LaunchRecipeInfo | null {
  if (recipes.length === 0) return null;
  const lastUsed = [...(project?.accounts_used ?? [])].reverse();
  for (const label of lastUsed) {
    const hit = recipes.find((recipe) => recipe.account_label === label);
    if (hit !== undefined) return hit;
  }
  const headroom = (recipe: LaunchRecipeInfo): number => {
    const found = quotas.find((entry) => entry.account === recipe.account_id);
    return found !== undefined ? (HEADROOM_RANK[found.forecast.state] ?? 4) : 4;
  };
  return [...recipes].sort((a, b) => headroom(a) - headroom(b))[0] ?? null;
}

export function NewProjectSheet({
  projects,
  accounts,
  initialDirectory,
  onClose,
  onLaunched,
  openLoginTerminal,
  onAccountsChanged,
  onManageLaunchSettings,
}: {
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  initialDirectory: string | null;
  onClose: () => void;
  onLaunched: (sessionId: string, title: string) => void;
  openLoginTerminal: (id: string, title: string) => void;
  onAccountsChanged: () => void;
  /** AJ-E309 Req.5: the Expert hatch's [Manage Launch settings →]. */
  onManageLaunchSettings?: () => void;
}) {
  const [directory, setDirectory] = useState<string | null>(initialDirectory);
  const [recipes, setRecipes] = useState<LaunchRecipeInfo[] | null>(null);
  const [quotas, setQuotas] = useState<QuotaDisplayData[]>([]);
  const [showAll, setShowAll] = useState(false);
  const [detour, setDetour] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const tilesRef = useRef<HTMLDivElement | null>(null);

  const reload = () => {
    void launchRecipesList().then(setRecipes);
  };
  useEffect(() => {
    funnelStep("b-new-project", "open");
    reload();
    void import("../lib/bridge").then((bridge) =>
      bridge.quotaDisplayAll().then(setQuotas),
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const project = useMemo(
    () => projects.find((row) => row.root === directory) ?? null,
    [projects, directory],
  );
  const loginByAccount = useMemo(() => {
    const map = new Map<string, string>();
    for (const account of accounts) map.set(account.id, account.login_state);
    return map;
  }, [accounts]);
  const quotaByAccount = useMemo(() => {
    const map = new Map<string, QuotaDisplayData>();
    for (const entry of quotas) map.set(entry.account, entry);
    return map;
  }, [quotas]);

  const recommended = useMemo(
    () => (recipes !== null ? pickRecipe(recipes, project, quotas) : null),
    [recipes, project, quotas],
  );

  const chooseDirectory = (dir: string) => {
    funnelStep("b-new-project", "pick-dir");
    setDirectory(dir);
    setShowAll(false);
  };

  const start = async (recipe: LaunchRecipeInfo) => {
    setBusy(true);
    setError(null);
    try {
      funnelStep("b-new-project", "launch");
      const result = await launchSession(recipe.id, directory);
      if (!result) {
        setError("Preview mode — no backend connected.");
        setBusy(false);
        return;
      }
      onLaunched(result.session_id, `${recipe.account_label} · ${recipe.name}`);
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };

  // Tiles are arrow-key navigable (Req: WCAG 2.1 visible focus) —
  // arrows move focus within the tile row, Enter/Space activate (the
  // browser's native button behaviour).
  const onTilesKey = (event: React.KeyboardEvent) => {
    const tiles = Array.from(
      tilesRef.current?.querySelectorAll<HTMLButtonElement>("button[data-tile]") ?? [],
    );
    if (tiles.length === 0) return;
    const index = tiles.findIndex((tile) => tile === document.activeElement);
    if (index === -1) return;
    const forward = event.key === "ArrowRight" || event.key === "ArrowDown";
    const back = event.key === "ArrowLeft" || event.key === "ArrowUp";
    if (!forward && !back) return;
    event.preventDefault();
    const next = forward ? (index + 1) % tiles.length : (index - 1 + tiles.length) % tiles.length;
    tiles[next]?.focus();
  };

  const recents = useMemo(
    () =>
      [...projects]
        .sort((a, b) => (b.last_activity_ms ?? 0) - (a.last_activity_ms ?? 0))
        .slice(0, 6),
    [projects],
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <Card className="flex max-h-[86vh] w-full max-w-xl flex-col overflow-hidden p-0">
        <div className="flex items-center gap-3 border-b border-hairline/60 px-6 py-4">
          <span className="text-accent-hi">
            <IconProjects width={16} height={16} />
          </span>
          <h3 className="text-sm font-semibold">New project</h3>
          <div className="ml-auto flex items-center gap-2">
            {directory && (
              <Chip tone="accent">
                <span className="max-w-64 truncate font-mono text-[10px]">
                  {directory.split("/").pop()}
                </span>
              </Chip>
            )}
          </div>
        </div>

        <div
          className="min-h-0 flex-1 overflow-y-auto px-6 py-5"
          onDragOver={(event) => {
            event.preventDefault();
            setDragging(true);
          }}
          onDragLeave={() => setDragging(false)}
          onDrop={(event) => {
            event.preventDefault();
            setDragging(false);
            // Webview drops carry names only; the native Tauri drop
            // path (App-level) opens this sheet pre-filled.
          }}
        >
          {/* Step 1 — where */}
          <p className="text-[11px] uppercase tracking-wider text-muted">
            Step 1 — where do you want to work?
          </p>
          <div
            ref={tilesRef}
            onKeyDown={onTilesKey}
            className="mt-2 grid grid-cols-2 gap-2"
            data-testid="project-tiles"
          >
            {recents.map((row) => (
              <button
                key={row.id}
                type="button"
                data-tile
                aria-label={`Work in ${row.name}${row.accounts_used.length > 0 ? ` — last used with ${row.accounts_used[row.accounts_used.length - 1]}` : ""}`}
                onClick={() => chooseDirectory(row.root)}
                className={
                  "flex min-w-0 flex-col gap-1 rounded-xl border px-3 py-2.5 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
                  (directory === row.root
                    ? "border-accent/40 bg-accent-soft"
                    : "border-hairline hover:border-hairline/80 hover:bg-surface")
                }
              >
                <span className="flex items-center gap-1.5">
                  <IconProjects width={12} height={12} />
                  <span className="truncate text-xs font-medium">{row.name}</span>
                </span>
                <span className="truncate text-[10px] text-muted">
                  {row.accounts_used.length > 0
                    ? `last used with ${row.accounts_used[row.accounts_used.length - 1]}`
                    : row.root}
                </span>
              </button>
            ))}
            <button
              type="button"
              data-tile
              onClick={async () => {
                const picked = await pickDirectory();
                if (picked) chooseDirectory(picked);
              }}
              className={
                "flex flex-col items-center justify-center gap-1 rounded-xl border border-dashed px-3 py-2.5 text-center outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
                (dragging
                  ? "border-accent/60 bg-accent-soft text-accent-hi"
                  : "border-hairline text-muted hover:border-accent/40 hover:text-ink")
              }
            >
              <IconProjects width={14} height={14} />
              <span className="text-xs font-medium">Choose folder…</span>
              <span className="text-[10px] text-muted">or drop it anywhere</span>
            </button>
          </div>

          {/* Step 2 — who (appears once a directory is chosen) */}
          {directory !== null && (
            <>
              <p className="mt-5 text-[11px] uppercase tracking-wider text-muted">
                Step 2 — which account?
              </p>
              <div className="mt-2 flex flex-col gap-1.5" data-testid="project-account">
                {recipes === null && <p className="py-3 text-xs text-muted">Loading…</p>}
                {recipes !== null && recipes.length === 0 && !detour && (
                  <div
                    className="flex flex-col items-start gap-2 rounded-xl border border-hairline px-4 py-3"
                    data-testid="no-account-detour"
                  >
                    <p className="text-xs text-muted">
                      No account set up yet — add one and your project launch picks up right
                      where it left off.
                    </p>
                    <Button size="sm" variant="primary" onClick={() => setDetour(true)}>
                      Add an account
                    </Button>
                  </div>
                )}
                {recipes !== null && recipes.length > 0 && recommended !== null && !showAll && (
                  <>
                    <div className="flex items-center gap-2 rounded-xl border border-accent/30 bg-accent-soft px-3 py-2.5 text-xs">
                      <IconTerminal width={13} height={13} />
                      <span className="font-medium">{recommended.account_label}</span>
                      <Chip tone="neutral">
                        {quotaByAccount.get(recommended.account_id)?.forecast.badge ?? "quota unknown"}
                      </Chip>
                      <span className="text-muted">{recommended.name}</span>
                      <span className="ml-auto text-[10px] text-muted">
                        {project !== null && project.accounts_used.includes(recommended.account_label)
                          ? "last used here"
                          : "recommended"}
                      </span>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <Button
                        size="sm"
                        variant="primary"
                        disabled={busy || !signedIn(loginByAccount.get(recommended.account_id))}
                        onClick={() => void start(recommended)}
                        data-testid="start-session"
                      >
                        Start session →
                      </Button>
                      <Button size="sm" variant="secondary" onClick={() => setShowAll(true)}>
                        Change
                      </Button>
                    </div>
                    {!signedIn(loginByAccount.get(recommended.account_id)) && (
                      <p className="text-[11px] text-muted">
                        This account needs sign-in (Ready home) — pick another or sign in first.
                      </p>
                    )}
                  </>
                )}
                {recipes !== null && showAll && (
                  <div className="flex flex-col gap-1.5" data-testid="project-account-list">
                    {recipes.map((recipe) => {
                      const ready = signedIn(loginByAccount.get(recipe.account_id));
                      return (
                        <button
                          key={recipe.id}
                          type="button"
                          disabled={busy || !ready}
                          onClick={() => void start(recipe)}
                          className={
                            "flex items-center gap-2 rounded-xl border px-3 py-2.5 text-left text-xs outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 " +
                            (ready
                              ? "border-hairline hover:border-accent/40 hover:bg-surface"
                              : "border-hairline/50 opacity-50")
                          }
                        >
                          <IconTerminal width={13} height={13} />
                          <span className="font-medium">{recipe.account_label}</span>
                          <Chip tone="neutral">
                            {quotaByAccount.get(recipe.account_id)?.forecast.badge ?? "quota unknown"}
                          </Chip>
                          <span className="text-muted">·</span>
                          <span className="text-muted">{recipe.name}</span>
                          {!ready && <Chip tone="neutral">needs sign-in (Ready home)</Chip>}
                        </button>
                      );
                    })}
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => setShowAll(false)}
                      className="self-start"
                    >
                      Back to recommendation
                    </Button>
                  </div>
                )}
              </div>
              {/* AJ-E309 Req.5: the expert hatch for the recommended
                  account (folder, shell line, env overrides). */}
              {recommended !== null && (
                <div className="mt-3">
                  <ExpertSetup
                    provider={recommended.provider}
                    accounts={[{ id: recommended.account_id, label: recommended.account_label }]}
                    surface="new-project"
                    onManageLaunchSettings={onManageLaunchSettings}
                  />
                </div>
              )}
            </>
          )}

          {error && <p className="mt-3 text-xs text-danger">{error}</p>}
        </div>

        <div className="flex items-center justify-between border-t border-hairline/60 px-6 py-4">
          <span className="text-[11px] text-muted">
            {directory
              ? `Starting in ${directory.split("/").pop()}`
              : "Pick a folder to continue"}
          </span>
          <Button size="sm" variant="secondary" onClick={onClose}>
            Close
          </Button>
        </div>
      </Card>

      {/* The inline flow (d) detour — completing it returns to the
          pending project launch (Req.7): same directory, refreshed
          accounts. */}
      {detour && (
        <OnboardingWizard
          provider="claude-code"
          onClose={() => setDetour(false)}
          onAccountCreated={() => {
            onAccountsChanged();
            reload();
            void import("../lib/bridge").then((bridge) =>
              bridge.quotaDisplayAll().then(setQuotas),
            );
          }}
          openLoginTerminal={openLoginTerminal}
        />
      )}
    </div>
  );
}
