/**
 * AJ-E611 — the explainable destination ranking, one component for
 * two hosts (NewSession now; the handover wizard reuses it when the
 * switch epic lands). Rows explain themselves in place: top-2 factors
 * in words + confidence badge + TTE; the top survivor is badged
 * "Suggested"; hard-filtered rows stay visible GREYED with their
 * reason. "Why this order?" expands all five inputs. Suggest-only —
 * the click (the host's own launch) is the only action. A11y: a
 * labelled LIST of buttons (no nested interactives inside listbox
 * options) with ↑/↓ roving focus between rows.
 */
import { useRef, useState } from "react";
import type { RankedRowData } from "../lib/bridge";

export function RankedRows({
  rows,
  onPick,
  activeAccount,
}: {
  rows: RankedRowData[];
  onPick: (account: string) => void;
  activeAccount?: string;
}): JSX.Element {
  const [whyOpen, setWhyOpen] = useState(false);
  const listRef = useRef<HTMLUListElement>(null);
  const onKeyDown = (event: React.KeyboardEvent<HTMLUListElement>) => {
    const delta = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (delta === 0) return;
    const buttons = Array.from(
      listRef.current?.querySelectorAll<HTMLButtonElement>("button[data-row]") ?? [],
    );
    if (buttons.length === 0) return;
    event.preventDefault();
    const index = buttons.findIndex((button) => button === document.activeElement);
    buttons[(index + delta + buttons.length) % buttons.length].focus();
  };
  return (
    <div className="flex flex-col gap-1.5" data-testid="ranked-rows">
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
        aria-label="Suggested accounts"
        className="flex flex-col gap-1.5"
      >
        {rows.map((row) => {
          const filtered = row.filtered_reason !== undefined;
          return (
            <li key={row.account}>
              <button
                type="button"
                data-row
                aria-pressed={row.account === activeAccount}
                onClick={() => onPick(row.account)}
                className={`flex w-full flex-col gap-0.5 rounded-lg border px-3 py-2 text-left text-xs transition-colors focus:outline-none focus:ring-2 focus:ring-accent ${
                  filtered
                    ? "cursor-not-allowed border-hairline"
                    : "border-hairline hover:border-accent/40 hover:bg-surface"
                }`}
              >
                <span className="flex items-center gap-2">
                  <span className={`font-medium ${filtered ? "text-muted" : ""}`}>
                    {row.account}
                  </span>
                  {row.suggested && (
                    <span className="rounded border border-accent/50 px-1 text-[10px] text-accent">
                      Suggested
                    </span>
                  )}
                  {row.tte_min !== undefined && (
                    <span className="text-[10px] text-muted">
                      exhausts ≈{Math.round(row.tte_min)} min
                    </span>
                  )}
                  <span className="ml-auto text-[10px] text-muted">{row.confidence}</span>
                </span>
                {filtered ? (
                  <span className="text-[10px] text-muted">{row.filtered_reason}</span>
                ) : (
                  <span className="text-[10px] text-muted">
                    {row.top_factors.join(" · ")}
                  </span>
                )}
                {whyOpen && !filtered && (
                  <span className="text-[10px] text-muted">
                    {row.factors.map(([words]) => words).join(" · ")}
                  </span>
                )}
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
