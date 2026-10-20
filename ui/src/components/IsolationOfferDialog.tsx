/**
 * AJ-E1002 — launching a second Instance into a project another one is
 * already working in. The safe choice is the default: its own git
 * worktree on a `juggler/<id>` branch, at a path shown (and editable)
 * before anything is created. Sharing the checkout is the explicit
 * opt-out and says what it risks. A non-git project says isolation is
 * unavailable and offers no fake alternative. One dialog serves every
 * launch entry point (the bridge's `launchSession` raises it).
 */
import { useEffect, useRef, useState, type JSX } from "react";
import {
  ISOLATION_OFFER_EVENT,
  type IsolationDecision,
  type IsolationOfferRequest,
} from "../lib/bridge";

export function IsolationOfferDialog(): JSX.Element | null {
  const [request, setRequest] = useState<IsolationOfferRequest | null>(null);
  const [choice, setChoice] = useState<"worktree" | "shared">("worktree");
  const [path, setPath] = useState("");
  const dialogRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const onRequest = (event: Event) => {
      const detail = (event as CustomEvent<IsolationOfferRequest & { claimed?: boolean }>).detail;
      detail.claimed = true;
      setChoice(detail.offer.available ? "worktree" : "shared");
      setPath(detail.offer.proposed_path ?? "");
      setRequest(detail);
    };
    window.addEventListener(ISOLATION_OFFER_EVENT, onRequest);
    return () => window.removeEventListener(ISOLATION_OFFER_EVENT, onRequest);
  }, []);

  const decide = (decision: IsolationDecision) => {
    request?.resolve(decision);
    setRequest(null);
  };

  useEffect(() => {
    if (!request) return;
    const node = dialogRef.current;
    node?.querySelector<HTMLElement>("input:checked, button")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        decide({ kind: "cancel" });
      }
      if (event.key === "Tab" && node) {
        const focusables = Array.from(node.querySelectorAll<HTMLElement>("input:not(:disabled), button:not(:disabled)"));
        if (focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
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
  const { offer } = request;
  const others = offer.in_use_by.join(", ");
  const go = () =>
    choice === "worktree" && offer.proposed_branch
      ? decide({ kind: "worktree", path: path.trim(), branch: offer.proposed_branch })
      : decide({ kind: "shared" });
  const ready = choice === "shared" || path.trim().length > 0;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm">
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="isolation-title"
        data-testid="isolation-offer"
        className="flex w-full max-w-lg flex-col gap-3 rounded-xl border border-hairline bg-surface p-5 shadow-card"
      >
        <h3 id="isolation-title" className="text-sm font-semibold">
          {others} {offer.in_use_by.length === 1 ? "is" : "are"} already working in {offer.project_name}
        </h3>
        {!offer.available && (
          <p className="text-[11px] text-ink" data-testid="isolation-unavailable">
            {offer.unavailable_reason}. Both sessions would edit the same files.
          </p>
        )}
        <div role="radiogroup" aria-labelledby="isolation-title" className="flex flex-col gap-1.5">
          <label
            className={
              "flex items-start gap-2 rounded-lg border px-2.5 py-2 text-[11px] " +
              (choice === "worktree" ? "border-accent/60 bg-accent-soft" : "border-hairline") +
              (offer.available ? "" : " opacity-70")
            }
          >
            <input
              type="radio"
              name="isolation"
              checked={choice === "worktree"}
              disabled={!offer.available}
              onChange={() => setChoice("worktree")}
              aria-describedby="isolation-worktree-desc"
              data-testid="isolation-worktree"
            />
            <span className="flex min-w-0 flex-1 flex-col gap-1">
              <span className="font-medium text-ink">Its own worktree (recommended)</span>
              <span id="isolation-worktree-desc" className="text-muted">
                {offer.available
                  ? `A separate checkout on a new branch ${offer.proposed_branch}${offer.base_branch ? ` from ${offer.base_branch}` : ""} — neither session can overwrite the other's files.`
                  : "Not possible here."}
              </span>
              {offer.available && choice === "worktree" && (
                <label className="flex flex-col gap-0.5 text-muted">
                  Created at
                  <input
                    type="text"
                    value={path}
                    onChange={(event) => setPath(event.target.value)}
                    className="rounded border border-hairline bg-canvas px-2 py-1 font-mono text-[11px] text-ink"
                    data-testid="isolation-path"
                    aria-label="Worktree folder"
                  />
                </label>
              )}
            </span>
          </label>
          <label
            className={
              "flex items-start gap-2 rounded-lg border px-2.5 py-2 text-[11px] " +
              (choice === "shared" ? "border-accent/60 bg-accent-soft" : "border-hairline")
            }
          >
            <input
              type="radio"
              name="isolation"
              checked={choice === "shared"}
              onChange={() => setChoice("shared")}
              aria-describedby="isolation-shared-desc"
              data-testid="isolation-shared"
            />
            <span>
              <span className="block font-medium text-ink">Share the checkout</span>
              <span id="isolation-shared-desc" className="text-muted">
                Both sessions edit the same files — one can silently overwrite the other's work.
              </span>
            </span>
          </label>
        </div>
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={() => decide({ kind: "cancel" })}
            className="rounded-lg border border-hairline px-3 py-1.5 text-xs text-ink"
            data-testid="isolation-cancel"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={!ready}
            onClick={go}
            className={
              "rounded-lg border px-3 py-1.5 text-xs " +
              (ready ? "border-accent/40 bg-accent-soft text-accent-hi" : "border-hairline text-muted")
            }
            data-testid="isolation-go"
          >
            {choice === "worktree" ? "Create the worktree and launch" : "Launch in the shared checkout"}
          </button>
        </div>
      </div>
    </div>
  );
}
