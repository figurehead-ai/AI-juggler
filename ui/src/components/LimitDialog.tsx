/**
 * D-25 — an account hit its limit: one decision, three ways on.
 *
 * 1. **Continue now** on the best other account (the ranker's pick),
 *    with its runway in words and what a switch carries over.
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

function clock(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
}

function inWords(ms: number): string {
  const minutes = Math.max(0, Math.round(ms / 60_000));
  const hours = Math.floor(minutes / 60);
  if (hours === 0) return `${minutes} min`;
  return minutes % 60 === 0 ? `${hours} h` : `${hours} h ${minutes % 60} min`;
}

/** The pick: the ranker's suggestion on another account, else its best. */
export function pickTarget(targets: SwitchTargetData[], fromAccount: string | null): SwitchTargetData | null {
  const usable = targets.filter((t) => t.account_id !== fromAccount && !t.filtered_reason);
  return usable.find((t) => t.suggested) ?? usable[0] ?? null;
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
  const [carry, setCarry] = useState(false);
  const [parking, setParking] = useState<"idle" | "busy" | "done">("idle");
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const invoker = useRef<Element | null>(typeof document === "undefined" ? null : document.activeElement);

  useEffect(() => {
    let alive = true;
    switchTargets(session.id).then((rows) => alive && setTargets(rows), () => alive && setTargets([]));
    quotaDisplayAll().then((rows) => alive && setDisplays(rows), () => null);
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
  const target = targets ? pickTarget(targets, session.account_id) : null;
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
        className="sheen flex w-full max-w-lg flex-col gap-4 rounded-card border border-hairline bg-surface p-6 shadow-[0_24px_80px_-24px_rgba(0,0,0,0.8)]"
      >
        <header className="flex flex-col gap-1">
          <h2 id="limit-title" className="text-base font-semibold">
            {session.account_label} hit its limit
          </h2>
          <p id="limit-what" className="text-sm leading-relaxed text-muted">
            {resetAt
              ? resetAt > now
                ? `It resets at ${clock(resetAt)}, in ${inWords(resetAt - now)}. `
                : "Its window has reset. "
              : "Its reset time isn't known yet. "}
            The session is paused where it stopped; nothing is lost.
          </p>
        </header>

        {parking === "done" ? (
          <section className="flex flex-col gap-3 rounded-xl border border-hairline p-4" data-testid="limit-parked">
            <p className="text-sm">
              Parked.{" "}
              {resetAt ? `At ${clock(resetAt)} it's offered back to you` : "When the reset is known, it's offered back to you"}; nothing restarts on
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
                    {target.suggested && (
                      <span className="rounded-full border border-accent/40 px-1.5 text-[10.5px] font-medium text-accent-hi">Recommended</span>
                    )}
                  </div>
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
                      {target.drops.length > 0 ? `Not carried over: ${target.drops.join(", ")}.` : "Everything this pair supports carries over."}
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
                Parks the session. {resetAt ? `At ${clock(resetAt)} it's offered back to you` : "Once the reset is known it's offered back to you"};
                nothing restarts on its own.
              </p>
              <Button
                size="sm"
                variant="secondary"
                disabled={parking === "busy" || session.status !== "quota_blocked"}
                title={session.status !== "quota_blocked" ? "Only a session stopped by its limit can be parked" : undefined}
                data-autofocus={target ? undefined : true}
                onClick={() => void park()}
              >
                {parking === "busy" ? "Parking…" : resetAt ? `Park until ${clock(resetAt)}` : "Park until the reset"}
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
            <p className="text-[11px] text-faint">You can switch back to {session.account_label} after the reset.</p>
          </>
        )}
      </div>
    </div>
  );
}
