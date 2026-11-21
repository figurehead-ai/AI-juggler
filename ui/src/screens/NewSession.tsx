import { useEffect, useMemo, useRef, useState } from "react";
import { ariaOf, keysOf } from "../lib/shortcuts";
import { useEscapeCloses } from "../lib/use-escape-closes";
import { Button } from "../components/primitives";
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
import { appsByRoom, chooseApp, type AppWhy } from "../lib/new-session-choice";
import { chooseFolder, nextFolder, recentFolders, steadyRecent, type FolderChoice } from "../lib/new-session-folder";
import { standingOffer, standOffer, withdrawOffer, type StartedBefore } from "../lib/new-session-sheet";
import { showToast } from "../components/Toast";
import { useNewSessionData } from "./new-session/useNewSessionData";
import { folderName, StatedChoice } from "./new-session/StatedChoice";
import { MoreOptions } from "./new-session/MoreOptions";
import { useAppPick } from "./new-session/useAppPick";
import { DialogFrame, DialogOverlay } from "../components/DialogFrame";

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
 * room is checked again. F-rc3f-1: a folder change never moves the App
 * unseen; the person's pick stays, a suggestion's move is said with Undo
 * (`useAppPick`).
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
  openLoginTerminal?: (id: string, title: string, options?: { account?: string }) => void;
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
  // F-rc3f-1: the App the person picked, and what a folder change did.
  const choice = useAppPick(initialAccount ?? null);
  const chosenApp = choice.chosen;
  const [chosenSettings, setChosenSettings] = useState<string | null>(null);
  // "Always use for web" set or cleared here, until the projects reload.
  const [defaults, setDefaults] = useState<Record<string, string | null>>({});
  const projects = useMemo(() => given.map((row) => (row.id in defaults ? { ...row, default_account_id: defaults[row.id] } : row)), [given, defaults]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // AJ-E2911: a launch that failed can be reported with what it showed.
  const [failure, setFailure] = useState<Failure | null>(null);
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

  // AJ-E2311 item 2: Recent (and the folder it opens on) by the starts
  // made here first; held in the order it first showed while the sheet is
  // open, so the tools' histories moving behind it never reshuffle it.
  const recentNow = useMemo(() => recentFolders(projects, sessions), [projects, sessions]);
  const [heldRecent, holdRecent] = useState<string[] | null>(null);
  const recent = useMemo(() => steadyRecent(heldRecent, recentNow, projects), [heldRecent, recentNow, projects]);
  if (recent.join("|") !== (heldRecent ?? []).join("|")) holdRecent(recent);
  const asked = useMemo(() => chooseFolder(projects, directory, recent), [projects, directory, recent]);
  const data = useNewSessionData({ folders: accounts, sessions, kept, project: asked?.project ?? null });
  const recipes = useMemo(() => data.recipes ?? [], [data.recipes]);
  const ranked = useMemo(() => appsByRoom({ recipes, folders: accounts, roomOf: data.roomOf }), [recipes, accounts, data]);
  // The pick never jumps under the cursor: once said with the room and the
  // Accounts that ran here known, it stays for this folder until the user
  // changes it (a fresh read only updates its room and notes).
  const steady = useRef<{ folder: string | null; app: string; said: { why: AppWhy; skipped: LaunchRecipeInfo | null; passedOver: LaunchRecipeInfo | null } } | null>(null);
  const keptApp = after?.app ?? null;
  const inputs = useMemo(
    () => ({ recipes, project: asked?.project ?? null, folders: accounts, sessions, kept: keptApp, roomOf: data.roomOf, accountOf: data.accountKeyOf, ranHere: data.ranHere }),
    [recipes, asked, accounts, sessions, keptApp, data],
  );
  const app = useMemo(() => {
    const held = steady.current;
    if (chosenApp === null && held && held.folder === (asked?.root ?? null)) {
      const kept = chooseApp({ ...inputs, chosen: held.app });
      if (kept?.recipe.account_id === held.app) return { ...kept, ...held.said };
    }
    const fresh = chooseApp({ ...inputs, chosen: chosenApp });
    if (chosenApp === null && fresh && data.roomKnown && data.ranHereKnown) steady.current = { folder: asked?.root ?? null, app: fresh.recipe.account_id, said: { why: fresh.why, skipped: fresh.skipped, passedOver: fresh.passedOver ?? null } };
    return fresh;
  }, [inputs, asked, chosenApp, data]);
  // What New session would suggest here when the person picked another:
  // first in Change app, and named when their pick was carried here.
  const suggestion = useMemo(() => (chosenApp === null ? null : chooseApp(inputs)), [chosenApp, inputs]);
  const settings = app ? recipes.filter((row) => row.account_id === app.recipe.account_id) : [];
  const recipe = settings.find((row) => row.id === chosenSettings) ?? app?.recipe ?? null;
  // No folder known: the App's own, from its launch settings.
  const folder: FolderChoice | null = asked ?? (recipe?.default_cwd ? { root: recipe.default_cwd, project: projects.find((row) => row.root === recipe.default_cwd) ?? null, why: "app-folder" } : null);
  const ready = recipe !== null && signedIn(accounts.find((row) => row.id === recipe.account_id)?.login_state ?? "logged_in");
  const sessionNames = useMemo(() => new Map(sessions.map((row) => [row.id, sessionTitle(row)])), [sessions]);
  const sessionName = (id: string) => sessionNames.get(id) ?? null;
  const appName = app ? data.appNameOf(app.recipe) : null;
  // The person's pick; the App shown picked again (F-rc3g-5) keeps the
  // launch settings chosen for it.
  const pickApp = (id: string) => {
    choice.pick(id);
    if (id !== app?.recipe.account_id) setChosenSettings(null);
  };
  // Change folder, a dropped folder, a saved handover's: one rule for the App.
  const changeFolder = (root: string) => {
    choice.moving(root === folder?.root ? null : app);
    setDirectory(root);
    setChosenSettings(null);
  };
  const { dragging, zone } = useFolderDrop(changeFolder); // wf10: a dropped folder
  const pack = usePack(changeFolder); // AJ-E1511: started from a saved handover

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
    // Said again from the default, and the person's if the folder changes
    // next; a cleared one keeps the App chosen in the sheet, if any.
    steady.current = null;
    choice.bind(id);
    onProjectsChanged?.();
  };

  useEscapeCloses(onClose);

  // D-96 F: "Start another: shop", the next project worked in (none
  // started in this run), keeping the App the user chose (in the sheet, or
  // kept from the start before); nothing when no project is left.
  const offerAnother = (cwd: string, sessionId: string | null) => {
    if (!onStartAnother || !folder) return;
    const roots = [...(after?.roots ?? []), cwd];
    const next = nextFolder(projects, roots, sessions);
    if (!next) return;
    const name = folderName(next);
    const carry = chosenApp ?? (app?.why === "kept" ? app.recipe.account_id : null);
    // The page changed under the sheet: the keyboard goes to the offer,
    // not back to the door that opened it (Return takes it, ⌘↩ starts).
    restoreFocusRef.current = null;
    const at = { dir: next.root, after: { roots, name: folderName(folder), app: carry } };
    // It stands until taken, closed, or the user works elsewhere (the
    // 0.4.1 persona walk: gone in 10 s, before it was read); meanwhile ⌘N
    // opens there too.
    const toastId = showToast({
      id: "start-another",
      testId: "start-another",
      tone: "success",
      message: `Started in ${folderName(folder)}`,
      detail: appName ? `on ${appName}` : undefined,
      waits: true,
      focusWhenFree: true,
      actionBelow: true,
      action: { label: `Start another: ${name}`, run: () => onStartAnother(at) },
      onClose: () => {
        if (standingOffer()?.toastId === toastId) withdrawOffer(true);
      },
    });
    standOffer({ ...at, sessionId, toastId });
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
      if (!pack.opened && cwd) offerAnother(cwd, result.session_id);
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
    <DialogOverlay
      ref={sheetRef}
      role="dialog"
      aria-modal="true"
      aria-label="New session"
      data-testid="new-session-sheet"
      // AJ-E2311 item 2: held at the top, not centred, so a line that
      // comes in a read later grows it downward and never moves the
      // choice under the pointer.
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/60 p-4 pt-[8vh] backdrop-blur-sm"
    >
      <DialogFrame
        className="sheen rounded-card border border-hairline bg-surface shadow-card w-full max-w-xl"
        headerClassName="flex items-center gap-3 px-6 py-4"
        header={<><IconTerminal width={16} height={16} className="text-accent-hi" /><h3 className="text-sm font-semibold">New session</h3></>}
        bodyClassName={"px-6 py-5 " + (dragging ? "bg-accent-soft/40 outline-dashed outline-1 -outline-offset-8 outline-accent/60" : "")}
        bodyTestId="new-session-drop"
        bodyProps={zone}
        footerClassName="flex items-center justify-end gap-2 bg-surface-2 px-6 py-4"
        footer={
          <>
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
          </>
        }
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
          recent={recent}
          handover={pack.opened !== null}
          now={now}
          suggestion={suggestion}
          told={choice.told}
          acted={choice.acted}
          onFolder={changeFolder}
          onApp={pickApp}
          onAlways={(id) => void setDefault(id)}
          onSignIn={async (id, name) => {
            const terminal = await terminalSpawnLogin(id);
            openLoginTerminal?.(terminal.id, `Sign in · ${name}`, { account: id });
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
      </DialogFrame>
    </DialogOverlay>
  );
}
