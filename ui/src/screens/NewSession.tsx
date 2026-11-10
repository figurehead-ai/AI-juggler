import { useEffect, useMemo, useRef, useState } from "react";
import { ariaOf, keysOf } from "../lib/shortcuts";
import { useEscapeCloses } from "../lib/use-escape-closes";
import { Button, Card } from "../components/primitives";
import { IconTerminal } from "../components/icons";
import { launchSession } from "../lib/ipc/launch";
import { projectSetDefaultAccount } from "../lib/ipc/accounts";
import { terminalSpawnLogin } from "../lib/ipc/terminal-io";
import { errorText, signedIn } from "../lib/bridge";
import { openReportBug, type Failure } from "../lib/report-bug";
import { failureOf } from "../components/switch/FailedStep";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import { launchTitle } from "../lib/account-words";
import { sessionTitle } from "../components/ManageWork";
import { useFolderDrop } from "../lib/use-folder-drop";
import { PackPicker, usePack } from "../components/OpenPack";
import { RemoteSessionSheet, useWhere } from "../components/RemoteLaunch";
import { useRuntimePick } from "../components/RuntimePick";
import { appsByRoom, chooseApp, chooseFolder, nextFolder, type AppWhy, type FolderChoice } from "../lib/new-session-choice";
import type { StartedBefore } from "../lib/new-session-sheet";
import { showToast } from "../components/Toast";
import { useNewSessionData } from "./new-session/useNewSessionData";
import { folderName, StatedChoice } from "./new-session/StatedChoice";
import { MoreOptions } from "./new-session/MoreOptions";

const FOCUSABLE = "button:not(:disabled), a[href], input, summary, [tabindex]:not([tabindex='-1'])";

/** A failure's code, when the backend sent one ("adapter"). */
function codeOf(cause: unknown): string | null {
  const code = (cause ?? {}) as { code?: unknown };
  return typeof code.code === "string" ? code.code : null;
}

/**
 * New session (AJ-E3207): the usual folder and App, chosen and said in
 * one sentence with a short why, from what's kept (no wait before the
 * choice shows); ⌘↩ or the one primary button starts it. Change folder
 * and Change app are one step away; the rare options sit behind one fold
 * that says their values. Out of room never moves the choice (D-31): the
 * sheet says so and offers the App with room. A folder dropped on the
 * window becomes where it works. D-96 F: once started, a toast offers
 * "Start another: <next project>", which opens it again there, keeping the
 * App the user chose; the usual-choice rule runs again for that folder, so
 * room is checked again.
 */
export function NewSessionModal({
  projects: given,
  accounts,
  sessions = [],
  recipes: kept,
  initialDirectory, initialAccount,
  onClose,
  onLaunched,
  openLoginTerminal,
  onManageLaunchSettings,
  onProjectsChanged,
  after = null,
  onStartAnother,
}: {
  projects: ProjectInfo[];
  /** The config folders (each CLI App's own). */
  accounts: AccountInfo[];
  /** The sessions the app already has: names for "Closing … frees about …". */
  sessions?: InstanceInfo[];
  /** The launch settings the app already holds: the choice in the first frame. */
  recipes?: LaunchRecipeInfo[];
  initialDirectory: string | null;
  /** The App (its folder) it opens on: the door that opened it chose it. */
  initialAccount?: string | null;
  onClose: () => void;
  onLaunched: (sessionId: string, title: string) => void;
  /** AJ-E306 Req.8: the Sign-in-first detour opens the vendor login
   * terminal beside this sheet — the sheet stays and waits. */
  openLoginTerminal?: (id: string, title: string) => void;
  /** AJ-E309 Req.5: the Expert hatch's [Manage Launch settings →]. */
  onManageLaunchSettings?: () => void;
  /** A project's default App was set or cleared here: reload the projects. */
  onProjectsChanged?: () => void;
  /** Opened from Start another: the starts just before (D-96 F). */
  after?: StartedBefore | null;
  /** Opens it again on the next project, after a start; without it,
   * nothing is offered. */
  onStartAnother?: (at: { dir: string; after: StartedBefore }) => void;
}) {
  const [directory, setDirectory] = useState<string | null>(initialDirectory);
  const [chosenApp, setChosenApp] = useState<string | null>(initialAccount ?? null);
  const [chosenSettings, setChosenSettings] = useState<string | null>(null);
  // "Always use for web" set or cleared here, until the projects reload.
  const [defaults, setDefaults] = useState<Record<string, string | null>>({});
  const projects = useMemo(() => given.map((row) => (row.id in defaults ? { ...row, default_account_id: defaults[row.id] } : row)), [given, defaults]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // AJ-E2911: a launch that failed can be reported with what it showed.
  const [failure, setFailure] = useState<Failure | null>(null);
  const { dragging, zone } = useFolderDrop(setDirectory); // wf10: a dropped folder
  const pack = usePack(setDirectory); // AJ-E1511: started from a saved handover
  const where = useWhere(); // AJ-E1515: or on a paired Mac
  const runtimePick = useRuntimePick(); // AJ-E2304: this session's runtime
  const sheetRef = useRef<HTMLDivElement | null>(null);
  const startRef = useRef<HTMLButtonElement | null>(null);
  const restoreFocusRef = useRef<HTMLElement | null>(null);
  const now = Date.now();

  // AJ-E306 a11y: the sheet traps focus and returns it on close (Esc cancels: the overlay).
  useEffect(() => {
    restoreFocusRef.current = document.activeElement as HTMLElement | null;
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;
      const focusables = sheetRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE);
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
    // AJ-E804: opening the sheet moves focus into it (WCAG 2.4.3): onto
    // Start when it can, so Return starts the usual.
    const frame = requestAnimationFrame(() => {
      if (sheetRef.current?.contains(document.activeElement)) return;
      (startRef.current && !startRef.current.disabled ? startRef.current : sheetRef.current?.querySelector<HTMLElement>(FOCUSABLE))?.focus();
    });
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("keydown", onKey);
      restoreFocusRef.current?.focus?.();
    };
  }, []);

  const asked = useMemo(() => chooseFolder(projects, directory), [projects, directory]);
  const data = useNewSessionData({ folders: accounts, sessions, kept, projectId: asked?.project?.id });
  const recipes = useMemo(() => data.recipes ?? [], [data.recipes]);
  const ranked = useMemo(() => appsByRoom({ recipes, folders: accounts, roomOf: data.roomOf }), [recipes, accounts, data]);
  // The pick never jumps under the cursor: once said with the room known,
  // it stays for this folder until the user changes it (a fresh read only
  // updates its room and notes).
  const steady = useRef<{ folder: string | null; app: string; said: { why: AppWhy; skipped: LaunchRecipeInfo | null; passedOver: LaunchRecipeInfo | null } } | null>(null);
  const keptApp = after?.app ?? null;
  const app = useMemo(() => {
    const inputs = { recipes, project: asked?.project ?? null, folders: accounts, sessions, kept: keptApp, roomOf: data.roomOf, accountOf: data.accountKeyOf };
    const held = steady.current;
    if (chosenApp === null && held && held.folder === (asked?.root ?? null)) {
      const kept = chooseApp({ ...inputs, chosen: held.app });
      if (kept?.recipe.account_id === held.app) return { ...kept, ...held.said };
    }
    const fresh = chooseApp({ ...inputs, chosen: chosenApp });
    if (chosenApp === null && fresh && data.roomKnown) steady.current = { folder: asked?.root ?? null, app: fresh.recipe.account_id, said: { why: fresh.why, skipped: fresh.skipped, passedOver: fresh.passedOver ?? null } };
    return fresh;
  }, [recipes, asked, accounts, chosenApp, keptApp, sessions, data]);
  const settings = app ? recipes.filter((row) => row.account_id === app.recipe.account_id) : [];
  const recipe = settings.find((row) => row.id === chosenSettings) ?? app?.recipe ?? null;
  // No folder known: the App's own, from its launch settings.
  const folder: FolderChoice | null = asked ?? (recipe?.default_cwd ? { root: recipe.default_cwd, project: projects.find((row) => row.root === recipe.default_cwd) ?? null, why: "app-folder" } : null);
  const ready = recipe !== null && signedIn(accounts.find((row) => row.id === recipe.account_id)?.login_state ?? "logged_in");
  const sessionNames = useMemo(() => new Map(sessions.map((row) => [row.id, sessionTitle(row)])), [sessions]);
  const sessionName = (id: string) => sessionNames.get(id) ?? null;
  const appName = app ? data.appNameOf(app.recipe) : null;

  // "Always use for web" (or No default), from Change app: the project's
  // default App, which New session picks first ("your default here").
  const setDefault = async (id: string | null) => {
    const project = folder?.project;
    if (!project) return;
    setError(null);
    try {
      await projectSetDefaultAccount(project.id, id);
    } catch (cause) {
      return setError(errorText(cause));
    }
    setDefaults((held) => ({ ...held, [project.id]: id }));
    // Said again from the default; a cleared one keeps the App chosen in
    // the sheet, if any.
    steady.current = null;
    if (id !== null) setChosenApp(null);
    onProjectsChanged?.();
  };

  useEscapeCloses(onClose);

  // D-96 F: "Start another: shop", the next project worked in (none
  // started in this run), keeping the App the user chose (in the sheet, or
  // kept from the start before); nothing when no project is left.
  const offerAnother = (cwd: string) => {
    if (!onStartAnother || !folder) return;
    const roots = [...(after?.roots ?? []), cwd];
    const next = nextFolder(projects, roots);
    if (!next) return;
    const name = folderName(next);
    const carry = chosenApp ?? (app?.why === "kept" ? app.recipe.account_id : null);
    // The page changed under the sheet: the keyboard goes to the offer,
    // not back to the door that opened it (Return takes it, ⌘↩ starts).
    restoreFocusRef.current = null;
    showToast({
      id: "start-another",
      testId: "start-another",
      tone: "success",
      message: `Started in ${folderName(folder)}`,
      detail: appName ? `on ${appName}` : undefined,
      seconds: 10,
      focusWhenFree: true,
      actionBelow: true,
      action: { label: `Start another: ${name}`, run: () => onStartAnother({ dir: next.root, after: { roots, name: folderName(folder), app: carry } }) },
    });
  };

  const launch = async () => {
    if (!recipe || !ready || busy) return;
    setBusy(true);
    setError(null);
    setFailure(null);
    const cwd = folder?.root ?? null;
    try {
      if (!pack.opened && !(await runtimePick.confirm(recipe.id, cwd))) return setBusy(false); // Lab 2: the folder's trust
      const result = pack.opened ? await pack.start(recipe.id, cwd) : await launchSession(recipe.id, cwd, runtimePick.runtime);
      if (!result) {
        setError("Preview mode — no backend connected.");
        setBusy(false);
        return;
      }
      onLaunched(result.session_id, launchTitle(recipe));
      if (!pack.opened && cwd) offerAnother(cwd);
    } catch (cause) {
      const words = errorText(cause);
      setError(words);
      setFailure({ from: "launch", words, detail: failureOf(cause).detail, code: codeOf(cause), atMs: Date.now() });
      setBusy(false);
    }
  };

  // ⌘↩ starts the stated choice, wherever focus is in the sheet.
  const launchRef = useRef(launch);
  launchRef.current = launch;
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
        event.preventDefault();
        void launchRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (where.machine) return <RemoteSessionSheet where={where} onClose={onClose} />;
  const startWords = folder ? `Start in ${folderName(folder)}` : appName ? `Start on ${appName}` : "Start";
  return (
    <div
      ref={sheetRef}
      role="dialog"
      aria-modal="true"
      aria-label="New session"
      data-testid="new-session-sheet"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
    >
      <Card className="flex max-h-[86vh] w-full max-w-xl flex-col overflow-hidden p-0">
        <div className="flex items-center gap-3 border-b border-hairline/60 px-6 py-4">
          <span className="text-accent-hi">
            <IconTerminal width={16} height={16} />
          </span>
          <h3 className="text-sm font-semibold">New session</h3>
        </div>

        <div
          className={"min-h-0 flex-1 overflow-y-auto px-6 py-5 " + (dragging ? "bg-accent-soft/40 outline-dashed outline-1 -outline-offset-8 outline-accent/60" : "")}
          data-testid="new-session-drop"
          {...zone}
        >
          {pack.opened && (
            <div className="mb-4">
              <PackPicker pack={pack} />
            </div>
          )}
          <StatedChoice
            folder={folder}
            app={app}
            data={data}
            ranked={ranked}
            projects={projects}
            handover={pack.opened !== null}
            now={now}
            onFolder={(root) => {
              setDirectory(root);
              setChosenSettings(null);
            }}
            onApp={(id) => {
              setChosenApp(id);
              setChosenSettings(null);
            }}
            onAlways={(id) => void setDefault(id)}
            onSignIn={async (id, name) => {
              const terminal = await terminalSpawnLogin(id);
              openLoginTerminal?.(terminal.id, `Sign in · ${name}`);
            }}
            onAddApp={onManageLaunchSettings}
            sessionName={sessionName}
            after={after?.name ?? null}
          />
          {/* Its own place: More options is folded by default. */}
          {runtimePick.trustElement}
          <MoreOptions
            recipe={recipe}
            settings={settings}
            onSettings={setChosenSettings}
            runtime={runtimePick.runtime}
            runtimeElement={runtimePick.element}
            where={where}
            pack={pack}
            advice={recipe ? data.adviceOf(recipe.id) : undefined}
            sessionName={sessionName}
            folders={accounts}
            onManageLaunchSettings={onManageLaunchSettings}
          />
          {error && (
            <div role="alert" className="mt-3 flex flex-wrap items-baseline gap-x-3 gap-y-1 text-xs text-danger" data-testid="launch-error">
              <span>{error}</span>
              {failure && (
                <button
                  type="button"
                  onClick={() => openReportBug(failure)}
                  className="rounded text-ink underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/60"
                  data-testid="launch-report-bug"
                >
                  Report a bug…
                </button>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-hairline/60 bg-surface-2 px-6 py-4">
          <Button size="sm" variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button ref={startRef} size="sm" variant="primary" disabled={busy || !ready} onClick={() => void launch()} aria-keyshortcuts={ariaOf("page-action")} data-testid="launch-now">
            {busy ? "Starting…" : startWords}
            {pack.opened && !busy && " with handover"}
            <kbd aria-hidden="true" className="ml-2 font-sans text-label">
              {keysOf("page-action")[0]}
            </kbd>
          </Button>
        </div>
      </Card>
    </div>
  );
}
