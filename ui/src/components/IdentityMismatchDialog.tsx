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
import { useEffect, useMemo, useRef, useState, type JSX } from "react";
import { IDENTITY_MISMATCH_EVENT, type IdentityDecision, type IdentityMismatchRequest } from "../lib/ipc/launch";
import { launchSettingsName } from "../lib/account-words";
import { useCliRecipes, useTargetApp } from "../lib/apps";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import { PRIMARY_FILL } from "./primitives";

const NO_RECIPES: readonly LaunchRecipeInfo[] = [];

export function IdentityMismatchDialog({ recipes = NO_RECIPES }: { recipes?: readonly LaunchRecipeInfo[] }): JSX.Element | null {
  const [request, setRequest] = useState<IdentityMismatchRequest | null>(null);
  const dialogRef = useRef<HTMLDivElement | null>(null);
  // The 0.4.6 RC walks A8 and K4: each way out is a CLI start, so a CLI
  // App's launch settings only (`cliRecipes`; the alternatives name a
  // recipe, its folder is the app's launch settings'), each named by its
  // App (`targetApp`: your name for it, never "Claude Code CLI · Work Max").
  const appOf = useTargetApp();
  const folders = useMemo(
    () =>
      (request?.check.alternatives ?? []).map((alternative) => {
        const recipe = recipes.find((row) => row.id === alternative.recipe_id);
        return { ...alternative, account_id: recipe?.account_id ?? "", tool: recipe?.provider ?? request?.check.provider ?? null };
      }),
    [request, recipes],
  );
  const alternatives = useCliRecipes(folders) ?? [];

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
        const first = buttons[0]!;
        const last = buttons[buttons.length - 1]!;
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
          <span aria-hidden="true">⚠ </span>Wrong account for {appOf({ folder: check.account_id, tool: check.provider, label: check.account_label }).name}
        </h3>
        <div id="identity-body" className="flex flex-col gap-1.5 text-label text-ink">
          <p>Nothing has launched. This app's sign-in changed:</p>
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
          {alternatives.map((alternative, index) => (
            <button
              key={alternative.recipe_id}
              type="button"
              onClick={() => decide({ kind: "switch", recipeId: alternative.recipe_id })}
              // 0.3.2: the first way out is the dialog's primary.
              className={"flex flex-col gap-0.5 rounded-lg border px-3 py-1.5 text-left text-xs " + (index === 0 ? PRIMARY_FILL : "border-accent/40 bg-accent-soft text-accent-hi")}
              data-testid="identity-switch"
            >
              <span className="font-medium">
                Use {appOf({ folder: alternative.account_id, tool: alternative.tool, label: alternative.account_label }).name} ({launchSettingsName(alternative.recipe_name)})
              </span>
              {/* The address whole, never broken across lines. */}
              <span>
                Signed in as <span className="whitespace-nowrap">{check.detected}</span>
              </span>
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
