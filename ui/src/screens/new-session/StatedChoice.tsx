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
 */
import type { JSX, ReactNode } from "react";
import { Button } from "../../components/primitives";
import { IconProjects } from "../../components/icons";
import { IconStar } from "../../components/icons-actions";
import { AccountAvatar } from "../../components/Avatar";
import { runwayOf } from "../../components/AccountRunway";
import { MachineAdviceLine } from "../../components/MachineAdvice";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { homeRelative } from "../../lib/project-words";
import { agoWords } from "../../lib/time-words";
import { lowRoomWords, outOfRoomWords } from "../../lib/room";
import { creditsConsequence } from "../../lib/credits";
import { roomMarkOf } from "../../lib/room-mark";
import { appWhyWords, folderWhyWords, passedOverWords, skippedWords, workedAt, type AppChoice, type appsByRoom, type FolderChoice } from "../../lib/new-session-choice";
import type { NewSessionData } from "./useNewSessionData";
import { AppPicker, FolderPicker } from "./ChoicePickers";

/** The folder's name as said: its project's, else its last part. */
export function folderName(folder: FolderChoice): string {
  return folder.project?.name ?? (folder.root.split("/").filter(Boolean).pop() || folder.root);
}

export function StatedChoice({
  folder,
  app,
  data,
  ranked,
  projects,
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
  data: NewSessionData;
  ranked: ReturnType<typeof appsByRoom>;
  projects: readonly ProjectInfo[];
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
  return (
    <section aria-labelledby="new-session-sentence" data-testid="launch-preselected" className="rounded-xl border border-accent/30 bg-accent-soft px-4 py-3.5">
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
        <Line icon={<IconProjects width={13} height={13} />} term="Folder" change={<FolderPicker projects={projects} root={folder?.root ?? null} onPick={onFolder} now={now} />}>
          {folder ? (
            <>
              <span className="font-mono text-label text-ink">{homeRelative(folder.root)}</span>
              <FolderWhen folder={folder} now={now} />
            </>
          ) : (
            <span className="text-ink-2">None yet: it starts in a temporary folder</span>
          )}
        </Line>
        {/* The Account's own dot on its line, not a terminal's mark (the
            0.4 design review, 71); no Account yet, or an App not signed in
            (no Account in use: the rc14 walk read its colour as a healthy
            state), is a neutral ring. */}
        <Line
          icon={
            app?.kind === "blocked" ? (
              <AccountAvatar account={{ folder: null }} label="Not signed in" size={13} tone="neutral" />
            ) : (
              <AccountAvatar account={{ folder: app?.recipe.account_id ?? null }} label={(app && data.accountNameOf(app.recipe.account_id)) ?? "No account yet"} size={13} tone={app && data.accountNameOf(app.recipe.account_id) ? "colour" : "neutral"} />
            )
          }
          change={ranked.length > 0 ? <AppPicker data={data} ranked={ranked} choice={app} project={folder?.project ?? null} onPick={onApp} onAlways={onAlways} now={now} /> : null}
        >
          {app ? <AccountWords app={app} data={data} /> : <NoApp loading={data.recipes === null} onAddApp={onAddApp} />}
        </Line>
        <BindingOffer app={app} folder={folder} data={data} ranked={ranked} onAlways={onAlways} />
      </div>
      {app && appName && <Notes app={app} appName={appName} data={data} now={now} onApp={onApp} onSignIn={onSignIn} sessionName={sessionName} />}
    </section>
  );
}

const LINK = "rounded font-medium text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60";

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

/** D-96 E: room picked over the App it would have opened on, as its
 * Account stands now ("Work is almost out (5% left)"); `null` once that
 * one has room again (the why is then the most room's). */
function reasonOf(app: AppChoice, data: NewSessionData, now: number, after: string | null | undefined): string | null {
  // The rc27 walk: the App history pointed to can't start; this one can.
  if (app.passedOver) return passedOverWords(data.appNameOf(app.passedOver));
  const skipped = app.skipped;
  if (!skipped) return appWhyWords(app.why, after);
  const id = skipped.account_id;
  // An estimate says "about", as its Low/Out mark does.
  const about = roomMarkOf(data.displayOf(id), data.sessionsOf(id), now)?.about ?? false;
  const skip = skippedWords(data.accountNameOf(id) ?? data.appNameOf(skipped), data.roomOf(id), now, about);
  return skip ?? appWhyWords(app.why, after);
}

function Line({ icon, term, change, children }: { icon: ReactNode; term?: string; change: ReactNode; children: ReactNode }): JSX.Element {
  return (
    <div className="flex min-h-7 items-center gap-2 text-xs">
      <span aria-hidden="true" className="shrink-0 text-muted">
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

function Notes({
  app,
  appName,
  data,
  now,
  onApp,
  onSignIn,
  sessionName,
}: {
  app: AppChoice;
  appName: string;
  data: NewSessionData;
  now: number;
  onApp: (folderId: string) => void;
  onSignIn: (folderId: string, appName: string) => void;
  sessionName: (id: string) => string | null;
}): JSX.Element | null {
  const id = app.recipe.account_id;
  const room = data.roomOf(id);
  const verdict = app.kind === "room" || app.kind === "unknown" ? data.verdictOf(id) : undefined;
  const advice = data.adviceOf(app.recipe.id);
  const offer = app.roomier ? (
    <OfferButton other={data.appNameOf(app.roomier)} left={data.roomOf(app.roomier.account_id).left} onUse={() => onApp(app.roomier!.account_id)} />
  ) : null;
  const lines: JSX.Element[] = [];
  if (app.kind === "out")
    lines.push(
      <Note key="out" testId="launch-out-of-room" tone="warning">
        {data.accountNameOf(id) ?? "Its account"} is {lowerFirst(outOfRoomWords(room.resetsAtMs, now))}: a session there would stop at its limit at once. {offer}
      </Note>,
    );
  if (app.kind === "low" && room.left !== null)
    lines.push(
      <Note key="low" testId="launch-low" tone="warning">
        {lowRoomWords(appName, room.left)} {offer}
      </Note>,
    );
  if (app.kind === "blocked")
    lines.push(
      <Note key="signin" testId="launch-signed-out" tone="warning">
        {appName} isn&apos;t signed in.{" "}
        <Button size="sm" variant="secondary" className="ml-1" onClick={() => onSignIn(id, appName)} data-testid="sign-in-first">
          Sign in first
        </Button>{" "}
        {offer}
      </Note>,
    );
  if (app.kind === "credits")
    lines.push(
      <Note key="credits" testId="launch-credits">
        {creditsConsequence(room.resetsAtMs, now)}
      </Note>,
    );
  if (verdict === "tight") lines.push(<Note key="tight" testId="launch-verdict">Tight — this session may use most of the room left</Note>);
  if (verdict === "no") lines.push(<Note key="no" testId="launch-verdict">Probably not enough room left — you can still start</Note>);
  if (advice && advice.verdict !== "ok")
    lines.push(
      <div key="mac" className="mt-2">
        <MachineAdviceLine advice={advice} nameOf={sessionName} />
      </div>,
    );
  return lines.length > 0 ? <div className="mt-2 border-t border-accent/20 pt-1">{lines}</div> : null;
}

function Note({ children, testId, tone = "plain" }: { children: ReactNode; testId: string; tone?: "plain" | "warning" }): JSX.Element {
  return (
    <p className={"mt-1.5 text-xs " + (tone === "warning" ? "text-ink" : "text-ink-2")} data-testid={testId}>
      {tone === "warning" && (
        <span aria-hidden="true" className="mr-1.5 text-warning">
          ▲
        </span>
      )}
      {children}
    </p>
  );
}

function lowerFirst(words: string): string {
  return words.charAt(0).toLowerCase() + words.slice(1);
}

/** "Use Claude Code · Personal (80% left)": the App with room, offered;
 * "(its room isn't measured)" when that's all that's known. */
function OfferButton({ other, left, onUse }: { other: string; left: number | null; onUse: () => void }): JSX.Element {
  return (
    <button type="button" onClick={onUse} className={LINK} data-testid="launch-use-roomier">
      Use {other}
      {left !== null ? ` (${left}% left)` : " (its room isn't measured)"}
    </button>
  );
}
