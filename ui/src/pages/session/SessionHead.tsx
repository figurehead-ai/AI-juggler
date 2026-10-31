/**
 * AJ-E2108 — a session page's header parts: its app's icon with the
 * profile's badge cut into its corner (the row anatomy's mark, larger),
 * the chips (its state, its Account menu), where it runs, and Go to it.
 * One set for every kind of session.
 */
import { useEffect, type JSX } from "react";
import { Button, type ChipTone } from "../../components/primitives";
import { ProfileBadge, markPlace } from "../../components/PlaceMarks";
import { AppIconTile } from "../../components/AppIcon";
import { IconGoTo } from "../../components/icons-actions";
import { ToolMark } from "../../components/ToolMark";
import { ago, agoWords } from "../../lib/time-words";
import type { SubscriptionInfo } from "../../lib/subscriptions";
import { AccountMenu } from "./AccountMenu";
import { appWords, type SessionModel } from "./model";

/** The app's icon in the header: the one place to swap in the app's
 * real icon (read from the Mac at run time) when it comes. Until then
 * the app's mark from the place registry, or, not open in any app now,
 * its tool's own mark (never "An app"). */
export function SessionAppIcon({ model, size = 48 }: { model: SessionModel; size?: number }): JSX.Element {
  if (model.places.length === 0 && model.sessionId === null) {
    return (
      <span className="inline-flex items-center justify-center rounded-[28%] border border-hairline bg-surface-3 text-ink" style={{ width: size, height: size }} data-testid="session-app-icon">
        <ToolMark tool={model.tool} size={Math.round(size / 2)} />
      </span>
    );
  }
  return (
    <span className="inline-flex" data-testid="session-app-icon">
      {(() => {
        const place = markPlace(model.places[0], model.sessionId !== null, model.tool, model.profileLabel);
        return <AppIconTile appId={place.appId} appPid={place.appPid} name={place.app} size={size} />;
      })()}
    </span>
  );
}

/** The header's mark: one app icon, the profile's badge (its colour and
 * monogram, the rail's) beside it, never over it. Decorative: the chips
 * name both in words. */
export function SessionMark({ model }: { model: SessionModel }): JSX.Element {
  const place = model.places[0];
  const name = place?.profile ?? place?.instance ?? model.profileLabel;
  return (
    <span aria-hidden="true" className="inline-flex shrink-0 items-end gap-1.5" data-testid="session-mark">
      <SessionAppIcon model={model} />
      {name && (
        <span data-testid="session-mark-profile" title={name} className="-mb-0.5">
          <ProfileBadge name={name} size={18} />
        </span>
      )}
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

const TONES: Record<ChipTone, string> = {
  neutral: "border-hairline bg-surface-2 text-ink",
  success: "border-success/30 bg-success-soft text-success",
  warning: "border-warning/35 bg-warning-soft text-warning",
  danger: "border-danger/30 bg-danger-soft text-danger",
  live: "border-live/30 bg-live-soft text-live",
  accent: "border-accent/30 bg-accent-soft text-accent-hi",
};

/**
 * The header's chips (0.3 polish: two, where there were four or five
 * saying the profile twice and the place three times): its state, and
 * its Account menu (the profile's badge and name; the account, profile
 * and folder inside, and the door to the profile's page). Where it runs
 * is on the facts line under them (`SessionWhere`), Go to it beside.
 */
export function SessionChips({ model, account, now }: { model: SessionModel; account: SubscriptionInfo | undefined; now: number }): JSX.Element {
  const since = model.live && model.state.since !== null ? ` · ${ago(model.state.since, now)}` : "";
  return (
    <>
      <span
        data-testid="session-state-chip"
        data-tone={model.state.tone}
        className={"inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full border px-2.5 py-0.5 text-meta font-semibold leading-5 " + TONES[model.state.tone]}
      >
        {model.live && <span aria-hidden="true" className={"size-2 rounded-full bg-current " + (model.state.working ? "motion-safe:animate-pulse" : "")} />}
        {model.state.words}
        {model.state.asks || model.state.tone === "accent" ? since : ""}
      </span>
      <AccountMenu model={model} account={account} />
    </>
  );
}

/** Where it runs, on the facts line: "Running in Terminal · viewed 2 min
 * ago", "Last active 3 h ago". */
export function SessionWhere({ model, now }: { model: SessionModel; now: number }): JSX.Element {
  const where = whereWords(model, now);
  return (
    <span className="min-w-0 truncate" title={where} data-testid="session-where">
      {where}
    </span>
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
    <Button size="md" variant="primary" disabled={why !== null} title={why ?? "Go to it (↵)"} onClick={onGo} data-testid="session-go-to">
      Go to it
      <IconGoTo width={14} height={14} aria-hidden="true" />
      <kbd aria-hidden="true" className="font-sans text-label opacity-80">
        ↵
      </kbd>
    </Button>
  );
}
