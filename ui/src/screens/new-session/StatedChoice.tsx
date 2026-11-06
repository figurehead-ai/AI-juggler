/**
 * AJ-E3207 — the usual choice, made and said: one sentence ("Start in
 * web-shop on Claude Code · Work — the last you used here"), then the
 * folder and the App's Account each on a line with its Change link. Room
 * is said once, for the Account; unknown room says nothing. Below, only
 * what's true now and changes the decision: out of room or low (with the
 * App that has room, offered, never taken), not signed in, a tight
 * verdict, this Mac short of memory.
 */
import type { JSX, ReactNode } from "react";
import { Button } from "../../components/primitives";
import { IconProjects } from "../../components/icons";
import { AccountAvatar } from "../../components/Avatar";
import { runwayOf } from "../../components/AccountRunway";
import { MachineAdviceLine } from "../../components/MachineAdvice";
import type { ProjectInfo } from "../../lib/bridge";
import { homeRelative } from "../../lib/project-words";
import { agoWords } from "../../lib/time-words";
import { lowRoomWords, outOfRoomWords } from "../../lib/room";
import { creditsConsequence } from "../../lib/credits";
import { appWhyWords, workedAt, type AppChoice, type appsByRoom, type FolderChoice } from "../../lib/new-session-choice";
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
  onSignIn,
  onAddApp,
  sessionName,
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
  onSignIn: (folderId: string, appName: string) => void;
  onAddApp?: () => void;
  sessionName: (id: string) => string | null;
}): JSX.Element {
  const appName = app ? data.appNameOf(app.recipe) : null;
  const why = app ? appWhyWords(app.why) : null;
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
        {why && <span className="text-ink-2"> — {why}</span>}
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
            0.4 design review, 71); no Account yet is a neutral ring. */}
        <Line
          icon={<AccountAvatar account={{ folder: app?.recipe.account_id ?? null }} label={(app && data.accountNameOf(app.recipe.account_id)) ?? "No account yet"} size={13} tone={app && data.accountNameOf(app.recipe.account_id) ? "colour" : "neutral"} />}
          change={ranked.length > 0 ? <AppPicker data={data} ranked={ranked} choice={app} project={folder?.project ?? null} onPick={onApp} now={now} /> : null}
        >
          {app ? <AccountWords app={app} data={data} /> : <NoApp loading={data.recipes === null} onAddApp={onAddApp} />}
        </Line>
      </div>
      {app && appName && <Notes app={app} appName={appName} data={data} now={now} onApp={onApp} onSignIn={onSignIn} sessionName={sessionName} />}
    </section>
  );
}

function Line({ icon, term, change, children }: { icon: ReactNode; term?: string; change: ReactNode; children: ReactNode }): JSX.Element {
  return (
    <div className="flex min-h-7 items-center gap-2 text-xs">
      <span aria-hidden="true" className="shrink-0 text-muted">
        {icon}
      </span>
      {/* One line that cuts at its end, never under the link. */}
      <span className="min-w-0 flex-1 truncate">
        {term && <span className="sr-only">{term}: </span>}
        {children}
      </span>
      {change && <span className="shrink-0">{change}</span>}
    </div>
  );
}

function FolderWhen({ folder, now }: { folder: FolderChoice; now: number }): JSX.Element | null {
  const at = folder.project ? workedAt(folder.project) : 0;
  const words = folder.why === "app-folder" ? "its launch settings' folder" : at > 0 ? `worked here ${agoWords(at, now)}` : null;
  return words ? <span className="text-muted"> · {words}</span> : null;
}

/** The Account's name and its room, said once; unknown room is silent. */
function AccountWords({ app, data }: { app: AppChoice; data: NewSessionData }): JSX.Element {
  const id = app.recipe.account_id;
  const name = data.accountNameOf(id);
  const run = runwayOf(data.displayOf(id), data.sessionsOf(id));
  const room = data.roomKnown && (run.left !== null || run.credits) && app.kind !== "out" ? run.words : null;
  return (
    <>
      <span className="text-muted">Account </span>
      <span className="font-medium text-ink">{name ?? "not known yet"}</span>
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

/** "Use Claude Code · Personal (80% left)": the App with room, offered. */
function OfferButton({ other, left, onUse }: { other: string; left: number | null; onUse: () => void }): JSX.Element {
  return (
    <button
      type="button"
      onClick={onUse}
      className="rounded font-medium text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="launch-use-roomier"
    >
      Use {other}
      {left !== null ? ` (${left}% left)` : ""}
    </button>
  );
}
