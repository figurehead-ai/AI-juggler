/**
 * AJ-E3207 — the usual choice, made and said: one sentence ("Start in
 * web-shop on Claude Code · Work — the last you used here"; from the
 * project's history alone, "— used here before"; picked by room over an
 * App that's almost out, "— Work is almost out (5% left)", D-96 E), then the
 * folder and the App's Account each on a line with its Change link. A
 * repo whose own Account it opened on (the only one that ever ran there,
 * D-103.7) offers the binding in one click: "Always start client-a on
 * Client A". Room is said once, for the Account; unknown room says
 * nothing. Below, only
 * what's true now and changes the decision: out of room or low (with the
 * App that has room, offered, never taken), not signed in, a tight
 * verdict, this Mac short of memory.
 *
 * F-rc3f-1 (the 0.4.6 rc3f walk: picking another folder moved the App to
 * another client's Account, unseen): one line shows the App and its
 * Account, the App's mark and name first, "Suggested" when New session
 * chose it. A folder change that moves a suggestion says so on the card
 * ("App changed to … (was …): the only account used here. Undo"); one
 * that carries the person's pick into a folder whose own App is another
 * names that one and offers it. Both, and the App and Account themselves
 * once the person acts, are said in a polite live region.
 *
 * AJ-E2312 item 5 (N5 and P7 on 0.4.7's nightly: the sheet moved ~50 px
 * when its warning left, ~20 px when Suggested left the App's line):
 * the card holds still while the sheet is open, as 0.4.7 held the sheet
 * at the top. A warning the person answered by moving off its App stays
 * answered in its place ("App changed to … (was …): Work is out of room
 * until 8:12 PM. Undo"), and the card never gets shorter (its tallest
 * height so far is its least), so nothing under it moves up under the
 * pointer; it only grows, downward.
 */
import { useLayoutEffect, useRef, type JSX, type ReactNode, type RefObject } from "react";
import { IconProjects } from "../../components/icons";
import { IconStar } from "../../components/icons-actions";
import { AccountAvatar } from "../../components/Avatar";
import { CliAppMark } from "../../components/PlaceMarks";
import { SuggestedMark } from "../../components/SwitchTargetRows";
import { runwayOf } from "../../components/AccountRunway";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { homeRelative } from "../../lib/project-words";
import { agoWords } from "../../lib/time-words";
import { suggestedMarkOn, type AppChoice, type appsByRoom } from "../../lib/new-session-choice";
import { folderWhyWords, workedAt, type FolderChoice } from "../../lib/new-session-folder";
import type { NewSessionData } from "./useNewSessionData";
import { AppPicker, FolderPicker } from "./ChoicePickers";
import { LINK, Notes, reasonOf, Told } from "./ChoiceNotes";
import type { AppPick } from "./useAppPick";

/** The folder's name as said: its project's, else its last part. */
export function folderName(folder: FolderChoice): string {
  return folder.project?.name ?? (folder.root.split("/").filter(Boolean).pop() || folder.root);
}

export function StatedChoice({
  folder,
  app,
  suggestion = null,
  told,
  acted = false,
  data,
  ranked,
  projects,
  recent,
  handover,
  now,
  onFolder,
  onApp,
  onAlways,
  onSignIn,
  onAddApp,
  sessionName,
  after,
}: {
  folder: FolderChoice | null;
  app: AppChoice | null;
  /** What New session would suggest for this folder when the person
   * picked another (`null`: `app` is the suggestion): first in Change app. */
  suggestion?: AppChoice | null;
  /** What a folder change did to the App (`useAppPick`): the App a
   * suggestion moved from, and the folder's own App, named and offered
   * when the person's pick was carried here. */
  told?: AppPick["told"];
  /** The person did something that may move the App: from then on the
   * live region says the App and its Account. */
  acted?: boolean;
  data: NewSessionData;
  ranked: ReturnType<typeof appsByRoom>;
  projects: readonly ProjectInfo[];
  /** Recent's order, as the sheet holds it (`steadyRecent`). */
  recent?: readonly string[];
  /** Started from a saved handover. */
  handover: boolean;
  now: number;
  onFolder: (root: string) => void;
  onApp: (folderId: string) => void;
  /** Sets or clears (`null`) the folder's project's default App. */
  onAlways?: (folderId: string | null) => void;
  onSignIn: (folderId: string, appName: string) => void;
  onAddApp?: () => void;
  sessionName: (id: string) => string | null;
  /** The folder of the start just before (Start another): a kept App's why. */
  after?: string | null;
}): JSX.Element {
  const appName = app ? data.appNameOf(app.recipe) : null;
  const why = app ? whyOf(app, data, now, after) : null;
  const suggested = suggestion ?? app;
  const { change, ask, left } = told?.(app, suggested) ?? { change: null, ask: null, left: null };
  const card = useHeldHeight<HTMLElement>();
  // SK-6: "Suggested" only on an App that can run now: on the one shown,
  // or, when it can't, on the one its note offers.
  const markOn = suggestedMarkOn(app, ranked);
  return (
    <section ref={card} aria-labelledby="new-session-sentence" data-testid="launch-preselected" className="rounded-xl border border-accent/30 bg-accent-soft px-4 py-3.5">
      <p id="new-session-sentence" data-testid="new-session-sentence" className="text-body leading-snug text-ink">
        {folder ? "Start in " : "Start a session"}
        {folder && <strong className="font-semibold">{folderName(folder)}</strong>}
        {appName && (
          <>
            {" on "}
            <strong className="font-semibold">{appName}</strong>
          </>
        )}
        {handover && " with the handover"}
        {why && (
          <span className="text-ink-2">
            {" — "}
            <Unbroken words={why} />
          </span>
        )}
      </p>
      <div className="mt-3 flex flex-col gap-1">
        <Line icon={<IconProjects width={13} height={13} />} term="Folder" change={<FolderPicker projects={projects} recent={recent} root={folder?.root ?? null} onPick={onFolder} now={now} />}>
          {folder ? (
            <>
              <span className="font-mono text-label text-ink">{homeRelative(folder.root)}</span>
              <FolderWhen folder={folder} now={now} />
            </>
          ) : (
            <span className="text-ink-2">None yet: it starts in a temporary folder</span>
          )}
        </Line>
        {/* The App and what a folder change did to it, together. */}
        <div>
          <AppLine
            app={app}
            suggested={app !== null && markOn === app.recipe.account_id}
            data={data}
            change={ranked.length > 0 ? <AppPicker data={data} ranked={ranked} choice={app} suggested={suggested} project={folder?.project ?? null} onPick={onApp} onAlways={onAlways} now={now} /> : null}
            onAddApp={onAddApp}
          />
          <Told app={app} change={change} ask={ask} acted={acted} place={folder ? folderName(folder) : null} data={data} now={now} after={after} onApp={onApp} />
        </div>
        <BindingOffer app={app} folder={folder} data={data} ranked={ranked} onAlways={onAlways} />
      </div>
      {app && appName && <Notes app={app} appName={appName} left={left} offerSuggested={markOn !== null && markOn === app.roomier?.account_id} data={data} now={now} onApp={onApp} onSignIn={onSignIn} sessionName={sessionName} />}
    </section>
  );
}

/** The card's least height is its tallest so far (AJ-E2312 item 5): a
 * line that goes (a warning answered, Suggested gone) never moves up
 * what's under it while the sheet is open. Measured after each drawing
 * and whenever its size changes; held in the element's own style, so it
 * draws nothing again. */
function useHeldHeight<T extends HTMLElement>(): RefObject<T | null> {
  const ref = useRef<T | null>(null);
  const held = useRef(0);
  useLayoutEffect(() => {
    const box = ref.current;
    const hold = () => {
      const height = box?.getBoundingClientRect().height ?? 0;
      if (!box || height <= held.current) return;
      held.current = height;
      box.style.minHeight = `${height}px`;
    };
    hold();
    const watch = box && typeof ResizeObserver !== "undefined" ? new ResizeObserver(hold) : null;
    if (box) watch?.observe(box);
    return () => watch?.disconnect();
  });
  return ref;
}

/** D-103.7: a repo with no binding offers it in one click, "Always start
 * client-a on Client A": on its own Account (the only one that ever ran
 * there), and, where several ran there (the R2 re-walk's trap: it opened
 * on the agency's), on whichever App is chosen now, following Change
 * app. It names the App instead when its Account has others here (the
 * binding is the App's, as Change app's "Always use for" is). */
function BindingOffer({
  app,
  folder,
  data,
  ranked,
  onAlways,
}: {
  app: AppChoice | null;
  folder: FolderChoice | null;
  data: NewSessionData;
  ranked: ReturnType<typeof appsByRoom>;
  onAlways?: (folderId: string | null) => void;
}): JSX.Element | null {
  const project = folder?.project;
  if (!app || !project || !onAlways || project.default_account_id) return null;
  if (app.why !== "own-account" && data.ranHere.length < 2) return null;
  const key = data.accountKeyOf(app.recipe.account_id);
  const alone = ranked.filter((row) => data.accountKeyOf(row.recipe.account_id) === key).length <= 1;
  const name = (alone ? data.accountNameOf(app.recipe.account_id) : null) ?? data.appNameOf(app.recipe);
  return (
    <Line icon={<IconStar width={13} height={13} />} change={null}>
      <button type="button" onClick={() => onAlways(app.recipe.account_id)} className={LINK} data-testid="launch-always">
        Always start {project.name} on {name}
      </button>
    </Line>
  );
}

/** The why's last two words kept together, so a narrow sheet never
 * leaves "left)" or "shop" alone on a line ("(5% / left)", "web- / shop"
 * at 800×600). */
function Unbroken({ words }: { words: string }): JSX.Element {
  const cut = words.lastIndexOf(" ", words.lastIndexOf(" ") - 1);
  if (cut <= 0) return <>{words}</>;
  return (
    <>
      {words.slice(0, cut + 1)}
      <span className="whitespace-nowrap">{words.slice(cut + 1)}</span>
    </>
  );
}

/** The why, and, where several Accounts ran in the repo and none is
 * bound, how many (D-103.7, the R2 re-walk: "— the last you used here; 3
 * accounts worked here"), so a hurried Return isn't taken for the
 * repo's own. */
function whyOf(app: AppChoice, data: NewSessionData, now: number, after: string | null | undefined): string | null {
  const said = reasonOf(app, data, now, after);
  const several = data.ranHere.length;
  return said && several > 1 && app.why !== "project-default" ? `${said}; ${several} accounts worked here` : said;
}

function Line({ icon, term, change, children }: { icon: ReactNode; term?: string; change: ReactNode; children: ReactNode }): JSX.Element {
  return (
    <div className="flex min-h-7 items-center gap-2 text-xs">
      {/* One icon column, as wide as the App's mark, so every line's
          words start together. */}
      <span aria-hidden="true" className="flex w-4 shrink-0 justify-center text-muted">
        {icon}
      </span>
      {/* AJ-E3306 (rc26, 800×600): the line wraps, a long path inside
          itself, rather than cut "worked here 17 …"; never under the link. */}
      <span className="min-w-0 flex-1 [overflow-wrap:anywhere]">
        {term && <span className="sr-only">{term}: </span>}
        {children}
      </span>
      {change && <span className="shrink-0">{change}</span>}
    </div>
  );
}

/** F-rc3f-1: the one place that shows the App and its Account. The App's
 * mark (its icon and its colour's dot, as the rail draws it) and its name
 * lead, with "Suggested" when New session chose it rather than the
 * person and it can run now (SK-6); its Account and room under it, in words (D-69: beside an App
 * an Account wears no colour); Change app on the App's line. */
function AppLine({ app, suggested, data, change, onAddApp }: { app: AppChoice | null; suggested: boolean; data: NewSessionData; change: ReactNode; onAddApp?: () => void }): JSX.Element {
  if (!app)
    return (
      <Line icon={<AccountAvatar account={{ folder: null }} label="No account yet" size={13} tone="neutral" />} term="App" change={change}>
        <NoApp loading={data.recipes === null} onAddApp={onAddApp} />
      </Line>
    );
  const { recipe } = app;
  return (
    <div className="flex items-start gap-2 text-xs" data-testid="launch-app-line">
      <span aria-hidden="true" className="flex h-7 w-4 shrink-0 items-center justify-center">
        <CliAppMark tool={recipe.provider} folderId={recipe.account_id} folder={recipe.account_label} size={16} ring="var(--color-surface)" />
      </span>
      <span className="min-w-0 flex-1 [overflow-wrap:anywhere]">
        <span className="flex min-h-7 flex-wrap items-center gap-x-2 gap-y-0.5">
          <span className="font-semibold text-ink" data-testid="launch-app">
            <span className="sr-only">App: </span>
            {data.appNameOf(recipe)}
          </span>
          {suggested && <SuggestedMark />}
        </span>
        <span className="-mt-1 block pb-1">
          <AccountWords app={app} data={data} />
        </span>
      </span>
      {change && <span className="shrink-0">{change}</span>}
    </div>
  );
}

function FolderWhen({ folder, now }: { folder: FolderChoice; now: number }): JSX.Element | null {
  const at = folder.project ? workedAt(folder.project) : 0;
  const words = at > 0 && folder.why !== "app-folder" ? `worked here ${agoWords(at, now)}` : folderWhyWords(folder.why);
  return words ? <span className="text-muted"> · {words}</span> : null;
}

/** The Account's name and its room, said once; unknown room is silent.
 * An App that isn't signed in says only that (rc7 walk: "78% left this
 * week" beside an App that couldn't start; rc8: its account's email). */
function AccountWords({ app, data }: { app: AppChoice; data: NewSessionData }): JSX.Element {
  // rc8 walk: an App that isn't signed in still named its account (an
  // email). Not signed in, no account is in use: it says only that, as
  // the switch's picker does.
  if (app.kind === "blocked")
    return (
      <>
        <span className="text-muted">Account </span>
        <span className="font-medium text-ink" data-testid="launch-account">
          not signed in
        </span>
      </>
    );
  const id = app.recipe.account_id;
  const name = data.accountNameOf(id);
  const run = runwayOf(data.displayOf(id), data.sessionsOf(id));
  const room = data.roomKnown && (run.left !== null || run.credits) && app.kind !== "out" ? run.words : null;
  return (
    <>
      <span className="text-muted">Account </span>
      <span className="font-medium text-ink" data-testid="launch-account">
        {name ?? "not known yet"}
      </span>
      {room && (
        <span className="text-ink-2" data-testid="launch-runway">
          {" "}
          · {room}
        </span>
      )}
    </>
  );
}

function NoApp({ loading, onAddApp }: { loading: boolean; onAddApp?: () => void }): JSX.Element {
  if (loading) return <span className="text-muted">Reading your apps…</span>;
  return (
    <span className="text-ink-2">
      No app to start it with yet.{" "}
      {onAddApp && (
        <button type="button" onClick={onAddApp} className="rounded text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60">
          Add one in Accounts
        </button>
      )}
    </span>
  );
}
