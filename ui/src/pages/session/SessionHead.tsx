/**
 * AJ-E2108 — a session page's header parts: its app's mark with the
 * profile's badge, the chips (state, where it runs, profile, account,
 * folder), and Go to it. One set for every kind of session.
 */
import { useEffect, type JSX } from "react";
import { Button, Chip } from "../../components/primitives";
import { PlaceMark, ProfileBadge, markPlace } from "../../components/PlaceMarks";
import { profileColor } from "../../lib/profile-marks";
import { IconGoTo } from "../../components/icons-actions";
import { shortPath } from "../../screens/AccountManage";
import { ago, agoWords } from "../../lib/time-words";
import { accountLine, accountName, type SubscriptionInfo } from "../../lib/subscriptions";
import { appWords, type SessionModel } from "./model";

/** The profile's name as the rail's badge reads it (its place's, else
 * its folder's): one colour and monogram everywhere. */
function profileName(model: SessionModel): string {
  const place = model.places[0];
  return place?.profile ?? place?.instance ?? model.profileLabel;
}

/** The app's mark, 52 px, with the profile's badge on its corner: the
 * same mark as the session's rail row and Home's (AJ-E2104). */
export function SessionMark({ model }: { model: SessionModel }): JSX.Element {
  const place = markPlace(model.places[0], model.sessionId !== null);
  return (
    <span className="relative inline-flex" data-testid="session-mark">
      <PlaceMark place={place} profile={model.profileLabel} size={52} />
    </span>
  );
}

/** Where it runs, in words: "Running in the Claude app · viewed 2 min
 * ago", never "in front" (D-37: the app's own record, said as such). */
export function whereWords(model: SessionModel, now: number): string {
  if (!model.live) {
    return model.state.since !== null ? `Last active ${agoWords(model.state.since, now)}` : "Not open now";
  }
  const place = model.places[0];
  const app = appWords(place, model.tool);
  const where = app ? `Running in ${app}` : model.sessionId ? "Running in AI Juggler" : "Running";
  const more = model.places.length > 1 ? ` (and ${model.places.length - 1} more)` : "";
  return model.viewedMs !== null ? `${where}${more} · viewed ${agoWords(model.viewedMs, now)}` : `${where}${more}`;
}

export function SessionChips({ model, account, now }: { model: SessionModel; account: SubscriptionInfo | undefined; now: number }): JSX.Element {
  const since = model.live && model.state.since !== null ? ` · ${ago(model.state.since, now)}` : "";
  const line = account ? accountLine(account) : null;
  return (
    <>
      <span data-testid="session-state-chip" data-tone={model.state.tone}>
        <Chip tone={model.state.tone} dot={model.live}>
          {model.state.words}
          {model.state.asks || model.state.tone === "accent" ? since : ""}
        </Chip>
      </span>
      <span data-testid="session-where-chip">
        <Chip tone="neutral">{whereWords(model, now)}</Chip>
      </span>
      <span
        data-testid="session-profile-chip"
        className="inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[11px] font-medium text-ink"
        style={{ borderColor: profileColor(profileName(model)) }}
      >
        <ProfileBadge name={profileName(model)} size={14} />
        Profile: {model.profileLabel}
      </span>
      {account && account.state === "signed_in" && (
        <span data-testid="session-account-chip">
          <Chip tone="neutral">
            Account: {accountName(account)}
            {line ? ` · ${line}` : ""}
          </Chip>
        </span>
      )}
      {model.cwd && (
        <span data-testid="session-folder-chip" title={model.cwd}>
          <Chip tone="neutral">
            <span className="font-mono">{shortPath(model.cwd)}</span>
          </Chip>
        </span>
      )}
    </>
  );
}

/** "Go to it ↵": the page's primary while it runs somewhere. Enter on the
 * page (not in a field or a terminal) does the same. */
export function GoToButton({ onGo, why }: { onGo: () => void; why: string | null }): JSX.Element {
  useEffect(() => {
    if (why !== null) return;
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const here = target === document.body || target?.closest?.("[data-testid='page-header']") !== null;
      if (event.key !== "Enter" || event.metaKey || event.altKey || event.ctrlKey || event.shiftKey || !here) return;
      if (target?.closest?.("button, a, input, textarea, select, [contenteditable='true']")) return;
      event.preventDefault();
      onGo();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onGo, why]);
  return (
    <Button size="sm" variant="primary" disabled={why !== null} title={why ?? "Go to it (↵)"} onClick={onGo} data-testid="session-go-to">
      Go to it
      <IconGoTo width={13} height={13} aria-hidden="true" />
      <kbd aria-hidden="true" className="font-sans text-[10px] opacity-80">
        ↵
      </kbd>
    </Button>
  );
}
