/**
 * AJ-E209 — the badged, explainable switch destinations (the
 * handover section's ranked rows; the RankedRows pattern from E611
 * with the E2 badge layer on top). Every row carries its ordered-pair
 * badge ("F1 · Beta") plus validated/unvalidated TEXT (never
 * colour-only, WCAG 2.2); cross-Provider rows always show the badge
 * AND the Drop list (D-11: "beta" is never mistaken for whole-file
 * fidelity). Top-2 factors in words + confidence + TTE; "Why this
 * order?" expands every input incl. switch cost. Hard-filtered rows
 * stay visible greyed with their reason. Suggest-only — the click
 * (the host's switch flow) is the user's. A11y: a labelled list of
 * buttons, ↑/↓ roving focus, no nested interactives.
 */
import { useRef, useState } from "react";
import type { SwitchTargetData } from "../lib/bridge";

/** The badge chip: solid when validated, hatched while beta. The text
 * label (the badge string itself + the validated line) carries the
 * state — the hatching is decoration only, legible in greyscale. */
function FidelityBadge({ badge, validated }: { badge: string; validated: boolean }) {
  return (
    <span
      data-testid="fidelity-badge"
      data-validated={validated ? "true" : "false"}
      className={
        "inline-flex items-center rounded border px-1.5 py-px text-[10px] " +
        (validated ? "border-accent/60 text-accent-hi" : "border-warning/60 text-warning")
      }
      style={
        validated
          ? undefined
          : {
              // Hatched fill marks "beta, not yet validated" — the
              // words say it too; this is never the only signal.
              backgroundImage:
                "repeating-linear-gradient(45deg, transparent 0 3px, rgba(217,119,6,0.18) 3px 5px)",
            }
      }
    >
      {badge}
      {!validated && " · Not yet validated"}
    </span>
  );
}

export function SwitchTargetRows({
  targets,
  onPick,
}: {
  targets: SwitchTargetData[];
  onPick: (target: SwitchTargetData) => void;
}): JSX.Element {
  const [whyOpen, setWhyOpen] = useState(false);
  const listRef = useRef<HTMLUListElement>(null);
  const onKeyDown = (event: React.KeyboardEvent<HTMLUListElement>) => {
    const delta = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (delta === 0) return;
    const buttons = Array.from(
      listRef.current?.querySelectorAll<HTMLButtonElement>("button[data-row]:not(:disabled)") ?? [],
    );
    if (buttons.length === 0) return;
    event.preventDefault();
    const index = buttons.findIndex((button) => button === document.activeElement);
    buttons[(index + delta + buttons.length) % buttons.length].focus();
  };
  return (
    <div className="flex flex-col gap-1.5" data-testid="switch-target-rows">
      <div className="flex items-center justify-between">
        <p className="text-[11px] uppercase tracking-wider text-muted">Ranked for you</p>
        <button
          type="button"
          aria-expanded={whyOpen}
          className="text-[11px] underline focus:outline-none focus:ring-2 focus:ring-accent"
          onClick={() => setWhyOpen((open) => !open)}
        >
          Why this order?
        </button>
      </div>
      <ul
        ref={listRef}
        onKeyDown={onKeyDown}
        aria-label="Switch destinations, ranked"
        className="flex flex-col gap-1.5"
      >
        {targets.map((target) => {
          const filtered = target.filtered_reason !== undefined && target.filtered_reason !== null;
          return (
            <li key={target.recipe_id}>
              <button
                type="button"
                data-row
                disabled={filtered}
                aria-pressed={target.suggested}
                onClick={() => onPick(target)}
                className={`flex w-full flex-col gap-1 rounded-lg border px-3 py-2 text-left text-xs transition-colors focus:outline-none focus:ring-2 focus:ring-accent ${
                  filtered
                    ? "cursor-not-allowed border-hairline"
                    : "border-hairline hover:border-accent/40 hover:bg-surface"
                }`}
              >
                <span className="flex flex-wrap items-center gap-2">
                  <span className={`font-medium ${filtered ? "text-muted" : ""}`}>
                    {target.account_label}
                  </span>
                  <FidelityBadge badge={target.badge} validated={target.validated} />
                  {target.suggested && !filtered && (
                    <span className="rounded border border-accent/50 px-1 text-[10px] text-accent">
                      Suggested
                    </span>
                  )}
                  {target.tte_min !== undefined && !filtered && (
                    <span className="text-[10px] text-muted">
                      exhausts ≈{Math.round(target.tte_min)} min
                    </span>
                  )}
                  <span className="ml-auto text-[10px] text-muted">{target.confidence}</span>
                </span>
                {target.cross_provider && target.drops.length > 0 && (
                  <span className="text-[10px] text-warning" data-testid="target-drop-list">
                    Doesn't carry: {target.drops.join(", ")}
                  </span>
                )}
                {filtered ? (
                  <span className="text-[10px] text-muted">{target.filtered_reason}</span>
                ) : (
                  <span className="text-[10px] text-muted">
                    {target.top_factors.join(" · ")}
                  </span>
                )}
                {whyOpen && !filtered && (
                  <span className="text-[10px] text-muted">
                    {target.factors.map(([words]) => words).join(" · ")}
                  </span>
                )}
              </button>
            </li>
          );
        })}
      </ul>
      <p className="text-[10px] text-faint">
        Badges are per provider pair and beta-validated by real fixtures — never assumed.
      </p>
    </div>
  );
}
