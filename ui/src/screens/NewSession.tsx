import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { homeRelative } from "../lib/project-words";
import { Button, Card, Chip } from "../components/primitives";
import { IconProjects, IconTerminal } from "../components/icons";
import { RunwayWords } from "../components/AccountRunway";
import { RankedRows } from "../components/RankedRows";
import { pickRecipe } from "./NewProjectSheet";
import { ExpertSetup } from "../components/ExpertSetup";
import { MachineAdviceLine, MachineChip } from "../components/MachineAdvice";
import { machineAdvice, type MachineAdviceInfo } from "../lib/machine";
import { hasLiveBackend, launchSession, pickDirectory, terminalSpawnLogin, quotaDisplayAll, rankingRows, errorText, signedIn } from "../lib/bridge";
import type { RankedRowData, QuotaDisplayData, AccountInfo, LaunchRecipeInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";
import { launchSettingsName, launchTitle } from "../lib/account-words";
import { sessionTitle } from "../components/ManageWork";
import { useTwinToggle } from "../lib/use-twin-toggle";
import { useFolderDrop } from "../lib/use-folder-drop";
import { PackPicker, usePack } from "../components/OpenPack";
import { MachinePicker, RemoteSessionSheet, useWhere } from "../components/RemoteLaunch";

/**
 * New session — zero typing: recent projects (real history), a native
 * folder picker, or drag a folder onto the window; then an account whose
 * sign-in state is visible before you commit.
 */
export function NewSessionModal({
  projects,
  accounts,
  sessions = [],
  initialDirectory,
  onClose,
  onLaunched,
  openLoginTerminal,
  onManageLaunchSettings,
}: {
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  /** The sessions the app already has: names for "Closing … frees about …". */
  sessions?: InstanceInfo[];
  initialDirectory: string | null;
  onClose: () => void;
  onLaunched: (sessionId: string, title: string) => void;
  /** AJ-E306 Req.8: the Sign-in-first detour opens the vendor login
   * terminal beside this sheet — the sheet stays and waits. */
  openLoginTerminal?: (id: string, title: string) => void;
  /** AJ-E309 Req.5: the Expert hatch's [Manage Launch settings →]. */
  onManageLaunchSettings?: () => void;
}) {
  const [directory, setDirectory] = useState<string | null>(initialDirectory);
  const [profiles, setProfiles] = useState<LaunchRecipeInfo[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { dragging, zone } = useFolderDrop(setDirectory); // wf10: a dropped folder
  const pack = usePack(setDirectory); // AJ-E1511: started from a context pack
  const where = useWhere(); // AJ-E1515: or on a paired Mac
  // AJ-E306 flow (e): Launch settings collapsed under Customise ▸ —
  // the recommended "Main" rides preselected; ⌘↩ launches it.
  const customise = useTwinToggle();
  const customiseOpen = customise.open;
  const sheetRef = useRef<HTMLDivElement | null>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);

  // AJ-E306 a11y: the sheet traps focus and returns it on close (Esc
  // cancels cleanly — the overlay already handles it).
  useEffect(() => {
    restoreFocusRef.current = document.activeElement as HTMLElement | null;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;
      const focusables = sheetRef.current?.querySelectorAll<HTMLElement>(
        "button:not(:disabled), a[href], input, [tabindex]:not([tabindex='-1'])",
      );
      if (focusables === undefined || focusables.length === 0) return;
      const first = focusables[0]!;
      const last = focusables[focusables.length - 1]!;
      // AJ-E804: focus that is not inside the sheet is pulled in.
      if (!sheetRef.current?.contains(document.activeElement)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
      } else if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    // AJ-E804: opening the sheet moves focus into it (WCAG 2.4.3).
    const frame = requestAnimationFrame(() => {
      if (!sheetRef.current?.contains(document.activeElement)) {
        sheetRef.current
          ?.querySelector<HTMLElement>("button:not(:disabled), a[href], input, [tabindex]:not([tabindex='-1'])")
          ?.focus();
      }
    });
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("keydown", onKey);
      restoreFocusRef.current?.focus?.();
    };
  }, []);

  useEffect(() => {
    void import("../lib/bridge").then((bridge) =>
      bridge.launchRecipesList().then(setProfiles),
    );
  }, []);

  // E14 (AJ-E1407): "Has this Mac got room?" per launch setting, and the
  // running sessions' names for "Closing … frees about …".
  // AJ-E1535: the names come from the sessions the app already has (one
  // read fewer on every open), named as everywhere else.
  const [advice, setAdvice] = useState<Map<string, MachineAdviceInfo>>(new Map());
  const sessionNames = useMemo(() => new Map(sessions.map((row) => [row.id, sessionTitle(row)])), [sessions]);
  const sessionsOf = (accountId: string) => sessions.filter((row) => row.account_id === accountId);
  useEffect(() => {
    let alive = true;
    void machineAdvice().then(
      (rows) => alive && setAdvice(new Map(rows.map((row) => [row.recipe_id, row]))),
      () => null,
    );
    return () => {
      alive = false;
    };
  }, []);

  const loginByAccount = useMemo(() => {
    const map = new Map<string, string>();
    for (const account of accounts) map.set(account.id, account.login_state);
    return map;
  }, [accounts]);

  // AJ-E611: the explainable ranking (suggest-only — clicking a row
  // highlights that account's recipes; the launch stays the user's
  // own click on a recipe).
  const [ranked, setRanked] = useState<RankedRowData[]>([]);
  const [pickedAccount, setPickedAccount] = useState<string | undefined>(undefined);
  useEffect(() => {
    let alive = true;
    rankingRows().then((rows) => {
      if (alive) setRanked(rows);
    });
    return () => {
      alive = false;
    };
  }, []);
  // AJ-E610: the account's two-clock display for the row chip (one
  // query for all rows; typed states render as copy, never 0%).
  const [quotaByAccount, setQuotaByAccount] = useState<Record<string, QuotaDisplayData>>({});
  useEffect(() => {
    let alive = true;
    quotaDisplayAll().then((all) => {
      if (!alive) return;
      const map: Record<string, QuotaDisplayData> = {};
      for (const entry of all) map[entry.account] = entry;
      setQuotaByAccount(map);
    });
    return () => {
      alive = false;
    };
  }, []);

  // AJ-E306 flow (e): the admission verdict per account (SPEC §5.2)
  // rides the ranked rows — Ok/Tight/No inline, unknown as TEXT.
  const verdictByAccount = useMemo(() => {
    const map = new Map<string, { admission?: "ok" | "tight" | "no"; unknown: boolean }>();
    for (const row of ranked) {
      map.set(row.account, { admission: row.admission ?? undefined, unknown: row.headroom_unknown });
    }
    return map;
  }, [ranked]);

  const project = useMemo(
    () => projects.find((row) => row.root === directory) ?? null,
    [projects, directory],
  );
  const preselected = useMemo(
    () =>
      profiles !== null ? pickRecipe(profiles, project, Object.values(quotaByAccount), accounts) : null,
    [profiles, project, quotaByAccount, accounts],
  );

  const recents = useMemo(
    () =>
      projects.filter((project) => !project.missing) // a moved one waits for Relocate…
        .sort(
          (a, b) => (b.last_activity_ms ?? 0) - (a.last_activity_ms ?? 0),
        )
        .slice(0, 6),
    [projects],
  );

  // ⌘↩ launches the preselected Launch settings (flow (e) HIG).
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === "Enter" && !busy && preselected) {
        event.preventDefault();
        void launch(preselected);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [busy, preselected, directory]);

  const launch = async (recipe: LaunchRecipeInfo) => {
    setBusy(true);
    setError(null);
    try {
      const result = pack.opened ? await pack.start(recipe.id, directory) : await launchSession(recipe.id, directory);
      if (!result) {
        setError("Preview mode — no backend connected.");
        setBusy(false);
        return;
      }
      onLaunched(result.session_id, launchTitle(recipe));
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };

  if (where.machine) return <RemoteSessionSheet where={where} onClose={onClose} />;
  return (
    <div
      ref={sheetRef}
      role="dialog"
      aria-modal="true"
      aria-label="New session"
      data-testid="new-session-sheet"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
      onKeyDown={(event) => event.key === "Escape" && onClose()}
    >
      <Card className="flex max-h-[86vh] w-full max-w-xl flex-col overflow-hidden p-0">
        <div className="flex items-center gap-3 border-b border-hairline/60 px-6 py-4">
          <span className="text-accent-hi">
            <IconTerminal width={16} height={16} />
          </span>
          <h3 className="text-sm font-semibold">New session</h3>
          <div className="ml-auto flex items-center gap-2">
            {directory && (
              <Chip tone="accent">
                <span className="max-w-64 truncate font-mono text-label">
                  {directory.split("/").pop()}
                </span>
              </Chip>
            )}
          </div>
        </div>

        <div
          className="min-h-0 flex-1 overflow-y-auto px-6 py-5"
          data-testid="new-session-drop"
          {...zone}
        >
          <MachinePicker where={where} />
          <PackPicker pack={pack} />
          {/* Step 1 — where */}
          <p className="text-section font-semibold text-ink">
            Where do you want to work?
          </p>
          <div className="mt-2 grid grid-cols-2 gap-2">
            {recents.map((project) => (
              <button
                key={project.id}
                type="button"
                onClick={() => setDirectory(project.root)}
                className={
                  "flex min-w-0 flex-col gap-1 rounded-xl border px-3 py-2.5 text-left transition-colors " +
                  (directory === project.root
                    ? "border-accent/40 bg-accent-soft"
                    : "border-hairline hover:border-hairline/80 hover:bg-surface")
                }
              >
                <span className="flex items-center gap-1.5">
                  <IconProjects width={12} height={12} />
                  <span className="truncate text-xs font-medium">
                    {project.name}
                  </span>
                  {project.active_instance && (
                    <span className="dot-live ml-auto size-1.5 rounded-full bg-live" />
                  )}
                </span>
                <span className="truncate text-label text-muted">
                  {project.history_sessions > 0 ? `${project.history_sessions} prior session${project.history_sessions === 1 ? "" : "s"} · ${project.accounts_used.join(", ")}` : homeRelative(project.root)}
                </span>
              </button>
            ))}
            <button
              type="button"
              onClick={async () => {
                const picked = await pickDirectory();
                if (picked) setDirectory(picked);
              }}
              className={
                "flex flex-col items-center justify-center gap-1 rounded-xl border border-dashed px-3 py-2.5 text-center transition-colors " +
                (dragging
                  ? "border-accent/60 bg-accent-soft text-accent-hi"
                  : "border-hairline text-muted hover:border-accent/40 hover:text-ink")
              }
            >
              <IconProjects width={14} height={14} />
              <span className="text-xs font-medium">Choose folder…</span>
              <span className="text-label text-muted">or drop it anywhere</span>
            </button>
          </div>

          {/* Step 2 — who (AJ-E306 flow (e)): the recommended Launch
              settings preselected; the full list under Customise ▸. */}
          <p className="mt-5 text-section font-semibold text-ink">
            {ranked.length > 0 && (
              <RankedRows
                rows={ranked}
                activeAccount={pickedAccount}
                onPick={(account) => setPickedAccount(account)}
              />
            )}
            Which account?
          </p>
          <div className="mt-2 flex flex-col gap-1.5" data-testid="launch-sheet-accounts">
            {profiles !== null && profiles.length > 0 && preselected !== null && !customiseOpen && (
              <div
                className="flex flex-col gap-1.5 rounded-xl border border-accent/30 bg-accent-soft px-3 py-2.5"
                data-testid="launch-preselected"
              >
                <div className="flex flex-wrap items-center gap-2 text-xs">
                  <IconTerminal width={13} height={13} />
                  <span className="font-medium">{preselected.account_label}</span>
                  <RunwayWords display={quotaByAccount[preselected.account_id]} sessions={sessionsOf(preselected.account_id)} />
                  <span className="text-muted">·</span>
                  <span className="text-muted">Launch settings: {preselected.name}</span>
                  {preselected.model && (
                    <span className="font-mono text-label text-muted">{preselected.model}</span>
                  )}
                  <VerdictChip verdict={verdictByAccount.get(preselected.account_id)} />
                  {/* AJ-E1501: why this one, when the user picked it. */}
                  {(project?.default_account_id === preselected.account_id ||
                    accounts.find((account) => account.is_default)?.id === preselected.account_id) && (
                    <span className="ml-auto text-label text-muted" data-testid="launch-default">
                      {project?.default_account_id === preselected.account_id ? "this project's default" : "your default"}
                    </span>
                  )}
                </div>
                <MachineAdviceLine advice={advice.get(preselected.id)} nameOf={(id) => sessionNames.get(id) ?? null} />
                <div className="flex flex-wrap gap-2">
                  <Button
                    size="sm"
                    variant="primary"
                    disabled={busy || !signedIn(loginByAccount.get(preselected.account_id))}
                    onClick={() => void launch(preselected)}
                    data-testid="launch-now"
                  >
                    Launch →
                  </Button>
                  <Button
                    ref={customise.showRef}
                    size="sm"
                    variant="secondary"
                    onClick={() => customise.toggle(true)}
                    aria-expanded={customiseOpen}
                  >
                    Customise ▸
                  </Button>
                  {!signedIn(loginByAccount.get(preselected.account_id)) && (
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={async () => {
                        const terminal = await terminalSpawnLogin(preselected.account_id);
                        openLoginTerminal?.(terminal.id, `Sign in · ${preselected.account_label}`);
                      }}
                    >
                      Sign in first
                    </Button>
                  )}
                </div>
              </div>
            )}
            {customiseOpen && (
              <button
                ref={customise.hideRef}
                type="button"
                onClick={() => customise.toggle(false)}
                className="self-start text-label text-muted underline-offset-2 hover:text-ink hover:underline"
                aria-expanded={customiseOpen}
              >
                Customise ▾ (hide Launch settings)
              </button>
            )}
            {(customiseOpen || preselected === null) &&
              (profiles ?? []).map((recipe) => {
                const login = loginByAccount.get(recipe.account_id) ?? "logged_out";
                const ready = signedIn(login);
                return (
                  <div
                    key={recipe.id}
                    className={
                      "flex flex-wrap items-center gap-2 rounded-xl border px-3 py-2.5 text-left text-xs " +
                      (ready ? "border-hairline" : "border-hairline/50") // AJ-E804: no opacity — text keeps 4.5:1
                    }
                  >
                    <IconTerminal width={13} height={13} />
                    <span className="font-medium">{recipe.account_label}</span>
                    <RunwayWords display={quotaByAccount[recipe.account_id]} sessions={sessionsOf(recipe.account_id)} />
                    <span className="text-muted">·</span>
                    <span className="text-muted">Launch settings: {launchSettingsName(recipe.name)}</span>
                    <VerdictChip verdict={verdictByAccount.get(recipe.account_id)} />
                    <MachineChip advice={advice.get(recipe.id)} />
                    <span className="flex-1" />
                    {ready ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        disabled={busy}
                        onClick={() => void launch(recipe)}
                      >
                        Launch →
                      </Button>
                    ) : (
                      // Req.8: disabled with its reason + the one-tap
                      // detour that RETURNS (the sheet never closes).
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={async () => {
                          const terminal = await terminalSpawnLogin(recipe.account_id);
                          openLoginTerminal?.(terminal.id, `Sign in · ${recipe.account_label}`);
                        }}
                        data-testid="sign-in-first"
                      >
                        Sign in first
                      </Button>
                    )}
                  </div>
                );
              })}
            {profiles !== null && profiles.length === 0 && (
              <p className="py-3 text-center text-xs text-muted">
                No launch settings yet — add an account first (Accounts, top right).
              </p>
            )}
            {profiles === null && (
              <p className="py-3 text-center text-xs text-muted">Loading…</p>
            )}
          </div>

          {/* AJ-E309 Req.5: the expert hatch for the account this
              sheet would launch (folder, shell line, env overrides). */}
          {preselected !== null && (
            <div className="mt-3">
              <ExpertSetup
                provider={
                  accounts.find((entry) => entry.id === preselected.account_id)?.provider ??
                  preselected.provider
                }
                accounts={[{ id: preselected.account_id, label: preselected.account_label }]}
                surface="launch"
                onManageLaunchSettings={onManageLaunchSettings}
              />
            </div>
          )}

          {error && <p className="mt-3 text-xs text-danger">{error}</p>}
        </div>

        <div className="flex items-center justify-between border-t border-hairline/60 px-6 py-4">
          {/* AJ-E1535: the footer speaks only when a folder is picked. */}
          <span className="text-label text-muted">{directory ? `Launching in ${directory.split("/").pop()}` : null}</span>
          <Button size="sm" variant="secondary" onClick={onClose}>
            Close
          </Button>
        </div>
      </Card>
    </div>
  );
}

void hasLiveBackend;

/**
 * AJ-E306 Req.7/8: the admission verdict as words — never a bare
 * colour, never a fabricated percentage. Tight/No explain and NEVER
 * block the launch click; unknown headroom says so in text.
 */
function VerdictChip({
  verdict,
}: {
  verdict?: { admission?: "ok" | "tight" | "no"; unknown: boolean };
}): JSX.Element | null {
  // No verdict: the runway line beside it already says what is known
  // (QA, P26b: "0% Estimate … Room left unknown" was two claims in a row).
  if (verdict === undefined) return null;
  if (verdict.admission === "tight") {
    return (
      <Chip tone="neutral" >
        Tight — this session may use most of the room left
      </Chip>
    );
  }
  if (verdict.admission === "no") {
    return (
      <Chip tone="neutral">Probably not enough room left — you can still start</Chip>
    );
  }
  if (verdict.admission === "ok") {
    return <Chip tone="neutral">Enough room left</Chip>;
  }
  return null;
}

