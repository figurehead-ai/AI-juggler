/**
 * AJ-E3207 — New session's notes under the stated choice: why it's the
 * App (`reasonOf`); only what's true now and changes the decision (out of
 * room or low, with the App that has room offered, never taken; not
 * signed in; a tight verdict; this Mac short of memory); and F-rc3f-1's,
 * what a folder change did to the App (`Told`).
 */
import type { JSX, ReactNode } from "react";
import { Button } from "../../components/primitives";
import { Alert } from "../../components/Alert";
import { MachineAdviceLine } from "../../components/MachineAdvice";
import { lowRoomWords, outOfRoomWords } from "../../lib/room";
import { creditsConsequence } from "../../lib/credits";
import { roomMarkOf } from "../../lib/room-mark";
import { appWhyWords, folderOwnWords, passedOverWords, skippedWords, type AppChoice } from "../../lib/new-session-choice";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";
import type { NewSessionData } from "./useNewSessionData";

export const LINK = "rounded font-medium text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60";

/** D-96 E: room picked over the App it would have opened on, as its
 * Account stands now ("Work is almost out (5% left)"); `null` once that
 * one has room again (the why is then the most room's). */
export function reasonOf(app: AppChoice, data: NewSessionData, now: number, after: string | null | undefined): string | null {
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

/** F-rc3f-1: what a folder change did to the App, on the card under it,
 * in the one Alert's `info` level (a fact, its action at its end): a
 * suggestion that moved ("App changed to Claude Code CLI · Lab (was
 * Claude Code CLI · aij-test): the only account used here. Undo"), or the
 * person's pick kept where the folder's own is another ("Kept …, as you
 * chose. Claude Code CLI · Lab is the only account used in team. Use
 * …"). The region is always there and polite: once the person acts, the
 * App and its Account are said when they change, and these as they come. */
export function Told({
  app,
  change,
  ask,
  acted,
  place,
  data,
  now,
  after,
  onApp,
}: {
  app: AppChoice | null;
  change: LaunchRecipeInfo | null;
  ask: AppChoice | null;
  acted: boolean;
  /** The folder's name, as said. */
  place: string | null;
  data: NewSessionData;
  now: number;
  after: string | null | undefined;
  onApp: (folderId: string) => void;
}): JSX.Element {
  const appName = app ? data.appNameOf(app.recipe) : null;
  const account = app && app.kind !== "blocked" ? data.accountNameOf(app.recipe.account_id) : null;
  const why = app && change ? reasonOf(app, data, now, after) : null;
  const own = ask && place && appName ? folderOwnWords(ask.why, place, data.appNameOf(ask.recipe)) : null;
  return (
    // Never hidden while empty: a region the page shows only once it has
    // words isn't heard when they come.
    <output aria-live="polite" data-testid="new-session-told" className="flex flex-col">
      {acted && appName && (
        <span className="sr-only" data-testid="new-session-heard">
          {account ? `Starts on ${appName}, account ${account}.` : `Starts on ${appName}.`}
        </span>
      )}
      {appName && change && (
        <Alert level="info" live={null} className="my-1" title={`App changed to ${appName}`} testId="launch-app-changed">
          {/* An App's name stays whole on its line ("Work / Max" read as two). */}
          (was <span className="whitespace-nowrap">{data.appNameOf(change)}</span>){why ? `: ${why}` : ""}.{" "}
          <button type="button" onClick={() => onApp(change.account_id)} aria-label={`Undo: back to ${data.appNameOf(change)}`} className={LINK} data-testid="launch-app-undo">
            Undo
          </button>
        </Alert>
      )}
      {appName && ask && own && (
        <Alert level="info" live={null} className="my-1" title={`Kept ${appName}, as you chose.`} testId="launch-app-kept">
          {own}.{" "}
          <button type="button" onClick={() => onApp(ask.recipe.account_id)} className={LINK} data-testid="launch-use-folder-own">
            Use {data.appNameOf(ask.recipe)}
          </button>
        </Alert>
      )}
    </output>
  );
}

export function Notes({
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
