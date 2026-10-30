/**
 * D-25 — an account hit its limit: one decision, three ways on.
 *
 * 1. **Continue now** on the best other account (the ranker's pick),
 *    with its runway in words and what a switch carries over — and, when
 *    the switch was prepared before the limit (AJ-E1513), "Ready".
 * 2. **Wait for the reset**: park the session; at the reset it is offered
 *    back — nothing restarts on its own (AJ-E1003).
 * 3. **Choose another…**: the session view, with every target ranked.
 *
 * Nothing happens until the user picks; Escape or Close leaves it as it is.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Button } from "./primitives";
import { runwayLines } from "./AccountRunway";
import {
  errorText,
  instancePark,
  quotaDisplayAll,
  switchTargets,
  type InstanceInfo,
  type QuotaDisplayData,
  type SwitchTargetData,
} from "../lib/bridge";
import { stagingReady, type StagedSwitchInfo } from "../lib/ipc/staging";
import { rankTargets, type RoomTarget } from "../lib/room-targets";
import { useSubscriptions, type SubscriptionInfo } from "../lib/subscriptions";
import { SuggestedMark } from "./SwitchTargetRows";
import { IconClose } from "./icons-controls";
import { resetsWords, whenWords } from "../lib/time-words";
import { creditsConsequence, usingCreditsWords } from "../lib/credits";

function inWords(ms: number): string {
  const minutes = Math.max(0, Math.round(ms / 60_000));
  const hours = Math.floor(minutes / 60);
  if (hours === 0) return `${minutes} min`;
  return minutes % 60 === 0 ? `${hours} h` : `${hours} h ${minutes % 60} min`;
}

function ago(ms: number): string {
  return ms < 60_000 ? "just now" : `${inWords(ms)} ago`;
}

/** The pick: the one order by room's suggestion (`lib/room`, as Home's
 * "Best for new work"), else the first other account that isn't out of
 * room (one carrying on with its credits can be: a session there goes on,
 * 0.3.2). Never the same account, a held-back one, or one out of room:
 * that would walk into another limit (the 0.2 walk, F1). */
export function pickTarget(
  targets: SwitchTargetData[],
  fromAccount: string | null,
  accounts: SubscriptionInfo[] | null = null,
  displays: QuotaDisplayData[] = [],
): RoomTarget | null {
  const ranked = rankTargets(targets, accounts, displays, fromAccount);
  return ranked.find((t) => t.suggested) ?? ranked.find((t) => t.room.kind !== "blocked" && t.room.kind !== "out") ?? null;
}

export function LimitDialog({
  session,
  sessions,
  now = Date.now(),
  onContinue,
  onChooseAnother,
  onParked,
  onClose,
}: {
  session: InstanceInfo;
  sessions: InstanceInfo[];
  now?: number;
  onContinue: (target: SwitchTargetData) => void;
  onChooseAnother: () => void;
  onParked: () => void;
  onClose: () => void;
}): JSX.Element {
  const [targets, setTargets] = useState<SwitchTargetData[] | null>(null);
  const [displays, setDisplays] = useState<QuotaDisplayData[]>([]);
  const [staged, setStaged] = useState<StagedSwitchInfo | null>(null);
  const [carry, setCarry] = useState(false);
  const [parking, setParking] = useState<"idle" | "busy" | "done">("idle");
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const invoker = useRef<Element | null>(typeof document === "undefined" ? null : document.activeElement);

  useEffect(() => {
    let alive = true;
    switchTargets(session.id).then((rows) => alive && setTargets(rows), () => alive && setTargets([]));
    quotaDisplayAll().then((rows) => alive && setDisplays(rows), () => null);
    stagingReady(session.id).then((ready) => alive && setStaged(ready), () => null);
    return () => {
      alive = false;
    };
  }, [session.id]);

  const close = () => {
    onClose();
    const back = invoker.current as HTMLElement | null;
    requestAnimationFrame(() => back?.focus?.());
  };

  useEffect(() => {
    dialogRef.current?.querySelector<HTMLElement>("[data-autofocus]")?.focus();
  }, [targets]);

  const resetAt = session.quota?.resets_at_ms ?? null;
  // 0.3: an interactive tool stays open at its limit (rule `limit_hit`).
  const stillOpen = session.attention?.rule === "limit_hit";
  const accounts = useSubscriptions();
  const target = targets ? pickTarget(targets, session.account_id, accounts, displays) : null;
  const ready = target && staged?.target_account_id === target.account_id ? staged : null;
  const targetLines = target
    ? runwayLines(
        displays.find((d) => d.account === target.account_id),
        sessions.filter((s) => s.account_id === target.account_id),
      ).filter((line) => line.left !== null)
    : [];

  const park = async () => {
    setParking("busy");
    setError(null);
    try {
      await instancePark(session.id);
      setParking("done");
      onParked();
    } catch (cause) {
      setParking("idle");
      setError(errorText(cause));
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-canvas/70 p-6 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        // A click on the dialog's text keeps focus inside it.
        tabIndex={-1}
        aria-labelledby="limit-title"
        aria-describedby="limit-what"
        data-testid="limit-dialog"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.stopPropagation();
            close();
          }
          if (event.key === "Tab" && dialogRef.current) {
            const items = [...dialogRef.current.querySelectorAll<HTMLElement>("button:not([disabled])")];
            const first = items[0];
            const last = items[items.length - 1];
            if (event.shiftKey && document.activeElement === first) {
              event.preventDefault();
              last?.focus();
            } else if (!event.shiftKey && document.activeElement === last) {
              event.preventDefault();
              first?.focus();
            }
          }
        }}
        className="sheen flex w-full max-w-lg flex-col gap-4 rounded-card border border-hairline bg-surface p-6 outline-none shadow-[0_24px_80px_-24px_rgba(0,0,0,0.8)]"
      >
        <header className="flex flex-col gap-1">
          <div className="flex items-start gap-3">
            <h2 id="limit-title" className="min-w-0 flex-1 text-base font-semibold">
              {session.account_label} hit its limit
            </h2>
            {/* 0.3: a close like every other dialog's (Not now does the same). */}
            <button
              type="button"
              onClick={close}
              aria-label="Close"
              title="Close"
              data-testid="limit-close"
              className="-mr-3 -mt-2 flex size-8 shrink-0 items-center justify-center rounded-lg text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60"
            >
              <IconClose width={16} height={16} aria-hidden="true" />
            </button>
          </div>
          <p id="limit-what" className="text-sm leading-relaxed text-muted">
            {resetAt
              ? resetAt > now
                ? `It ${resetsWords(resetAt, now)}, in ${inWords(resetAt - now)}. `
                : "Its window has reset. "
              : "Its reset time isn't known yet. "}
            {stillOpen ? "The session is still open where it stopped; nothing is lost." : "The session is paused where it stopped; nothing is lost."}
          </p>
        </header>

        {parking === "done" ? (
          <section className="flex flex-col gap-3 rounded-xl border border-hairline p-4" data-testid="limit-parked">
            <p className="text-sm">
              Parked.{" "}
              {resetAt ? `At ${whenWords(resetAt, now)} it's offered back to you` : "When the reset is known, it's offered back to you"}; nothing restarts on
              its own.
            </p>
            <Button size="sm" variant="primary" data-autofocus onClick={close}>
              Done
            </Button>
          </section>
        ) : (
          <>
            <section className="flex flex-col gap-3 rounded-xl border border-accent/40 bg-accent-soft/40 p-4" data-testid="limit-continue">
              {targets === null ? (
                <p className="text-xs text-muted">Finding the best account for this…</p>
              ) : target ? (
                <>
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-sm font-medium">Continue now on {target.account_label}</span>
                    {target.suggested && <SuggestedMark />}
                  </div>
                  {ready && (
                    <p className="text-xs text-success" data-testid="limit-ready">
                      Ready to continue on {target.account_label} · prepared {ago(now - ready.prepared_at_ms)}
                    </p>
                  )}
                  {target.room.kind === "credits" && target.room.credits && (
                    <p className="text-xs text-ink" data-testid="limit-target-credits">
                      {usingCreditsWords(target.room.credits)}. {creditsConsequence(target.room.resetsAtMs, now, "It goes on")}
                    </p>
                  )}
                  {targetLines.length > 0 && (
                    <p className="text-xs text-muted" data-testid="limit-target-runway">
                      {targetLines.map((line) => `${line.label}: ${line.words}`).join(" · ")}
                    </p>
                  )}
                  <p className="text-xs leading-relaxed text-muted">
                    Takes the conversation, the branch and the project's context with it. Switching is in beta ({target.badge}):{" "}
                    <button
                      type="button"
                      aria-expanded={carry}
                      onClick={() => setCarry((open) => !open)}
                      className="text-ink underline decoration-hairline underline-offset-2 outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
                    >
                      see what carries over
                    </button>
                    .
                  </p>
                  {carry && (
                    <p className="text-xs text-muted" data-testid="limit-drops">
                      {target.drops.length > 0 ? `Doesn't carry: ${target.drops.join(", ")}.` : "Everything this pair supports carries over."}
                    </p>
                  )}
                  <Button size="sm" variant="primary" data-autofocus onClick={() => onContinue(target)}>
                    Continue on {target.account_label}
                  </Button>
                </>
              ) : (
                <p className="text-xs text-muted" data-testid="limit-no-target">
                  No other account can take this session right now. Wait for the reset, or add an account.
                </p>
              )}
            </section>

            <section className="flex flex-col gap-2 rounded-xl border border-hairline p-4" data-testid="limit-wait">
              <span className="text-sm font-medium">Wait for the reset</span>
              <p className="text-xs leading-relaxed text-muted">
                {stillOpen ? "Keeps it open where it stopped. " : "Parks the session. "}
                {resetAt ? `At ${whenWords(resetAt, now)} it's offered back to you` : "Once the reset is known it's offered back to you"};
                nothing {stillOpen ? "runs" : "restarts"} on its own.
              </p>
              {/* 0.3: said with the wait it belongs to, not under the footer. */}
              <p className="text-xs leading-relaxed text-muted" data-testid="limit-switch-back">
                You can switch back to {session.account_label} after the reset.
              </p>
              <Button
                size="sm"
                variant="secondary"
                disabled={parking === "busy" || session.status !== "quota_blocked"}
                title={session.status !== "quota_blocked" ? "Only a session stopped by its limit can be parked" : undefined}
                data-autofocus={target ? undefined : true}
                onClick={() => void park()}
              >
                {parking === "busy" ? "Parking…" : resetAt ? `Park until ${whenWords(resetAt, now)}` : "Park until the reset"}
              </Button>
              {error && <p className="text-xs text-danger">{error}</p>}
            </section>

            <footer className="flex flex-wrap items-center justify-between gap-2">
              <button
                type="button"
                onClick={onChooseAnother}
                className="text-xs text-muted underline decoration-hairline underline-offset-2 outline-none hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
              >
                Choose another account or tool…
              </button>
              <Button size="sm" variant="ghost" onClick={close}>
                Not now
              </Button>
            </footer>
          </>
        )}
      </div>
    </div>
  );
}
