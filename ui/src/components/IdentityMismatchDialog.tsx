/**
 * AJ-E1001 — the wrong-account guard's pause. Shown when a launch's
 * Account is signed in as someone other than it was first seen with:
 * expected vs detected in words (never colour alone), and only three
 * ways forward — switch to the Launch setting that IS that identity,
 * check again (after fixing the sign-in), or cancel. There is no
 * "continue anyway". One dialog serves every launch entry point (the
 * bridge's `launchSession` raises it). Keyboard: focus starts on the
 * first action, Tab stays inside, Esc cancels.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import {
  IDENTITY_MISMATCH_EVENT,
  type IdentityDecision,
  type IdentityMismatchRequest,
} from "../lib/bridge";
import { launchSettingsName } from "../lib/account-words";

export function IdentityMismatchDialog(): JSX.Element | null {
  const [request, setRequest] = useState<IdentityMismatchRequest | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const onRequest = (event: Event) => {
      const detail = (event as CustomEvent<IdentityMismatchRequest & { claimed?: boolean }>).detail;
      detail.claimed = true;
      setRequest(detail);
    };
    window.addEventListener(IDENTITY_MISMATCH_EVENT, onRequest);
    return () => window.removeEventListener(IDENTITY_MISMATCH_EVENT, onRequest);
  }, []);

  const decide = (decision: IdentityDecision) => {
    request?.resolve(decision);
    setRequest(null);
  };

  useEffect(() => {
    if (!request) return;
    const node = dialogRef.current;
    node?.querySelector<HTMLElement>("button")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        decide({ kind: "cancel" });
      }
      if (event.key === "Tab" && node) {
        const buttons = Array.from(node.querySelectorAll<HTMLElement>("button"));
        if (buttons.length === 0) return;
        const first = buttons[0];
        const last = buttons[buttons.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [request]);

  if (!request) return null;
  const { check } = request;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="identity-title"
        aria-describedby="identity-body"
        data-testid="identity-mismatch"
        className="flex w-full max-w-md flex-col gap-3 rounded-xl border border-warning/50 bg-surface p-5 shadow-card"
      >
        <h3 id="identity-title" className="text-sm font-semibold">
          <span aria-hidden="true">⚠ </span>Wrong account for «{check.account_label}»
        </h3>
        <div id="identity-body" className="flex flex-col gap-1.5 text-label text-ink">
          <p>Nothing has launched. This account's {check.provider} sign-in changed:</p>
          <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 rounded-lg border border-hairline p-2.5">
            <dt className="text-muted">Expected</dt>
            <dd data-testid="identity-expected">{check.expected ?? "—"}</dd>
            <dt className="text-muted">Signed in as</dt>
            <dd data-testid="identity-detected" className="font-medium">
              {check.detected ?? "—"}
            </dd>
          </dl>
          <p className="text-ink-2">
            Work started now would be billed to the account signed in. Pick the right one, or fix the
            sign-in and check again.
          </p>
        </div>
        <div className="flex flex-col gap-1.5">
          {check.alternatives.map((alternative) => (
            <button
              key={alternative.recipe_id}
              type="button"
              onClick={() => decide({ kind: "switch", recipeId: alternative.recipe_id })}
              className="rounded-lg border border-accent/40 bg-accent-soft px-3 py-1.5 text-left text-xs text-accent-hi"
              data-testid="identity-switch"
            >
              Use «{alternative.account_label}» ({launchSettingsName(alternative.recipe_name)}) — it is signed in as{" "}
              {check.detected}
            </button>
          ))}
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => decide({ kind: "recheck" })}
              className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
              data-testid="identity-recheck"
            >
              Check again
            </button>
            <button
              type="button"
              onClick={() => decide({ kind: "cancel" })}
              className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
              data-testid="identity-cancel"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
