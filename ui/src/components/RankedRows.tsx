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
import { forecastWords } from "../lib/quotaWindow";
import { rankByRoom, ROOM_ORDER_WORDS } from "../lib/room";
import { useRef, useState, type JSX } from "react";
import type { RankedRowData } from "../lib/bridge";

export function RankedRows({
  rows: given,
  leftOf,
  creditsOf,
  onPick,
  activeAccount,
}: {
  rows: RankedRowData[];
  /** Each account's % left, as Home's Runway reads it: the rows go in the
   * one order by room and "Suggested" is its pick (the 0.2 walk, F17). */
  leftOf?: (account: string) => number | null;
  /** A used-up window its credits carry on past (0.3.2): ranked after the
   * low ones, before any out of room. */
  creditsOf?: (account: string) => boolean;
  onPick: (account: string) => void;
  activeAccount?: string;
}): JSX.Element {
  const rows = leftOf
    ? rankByRoom(given, (row) => leftOf(row.account), (row) => row.filtered_reason == null, (row) => creditsOf?.(row.account) ?? false).map(({ item, suggested }) => ({ ...item, suggested }))
    : given;
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
        <p className="text-section font-semibold text-ink">Ranked for you</p>
        <button
          type="button"
          aria-expanded={whyOpen}
          className="py-1 text-label underline focus:outline-none focus:ring-2 focus:ring-accent"
          onClick={() => setWhyOpen((open) => !open)}
        >
          Why this order?
        </button>
      </div>
      {whyOpen && (
        <p className="text-label text-muted" data-testid="why-this-order">
          {ROOM_ORDER_WORDS}
        </p>
      )}
      <ul
        ref={listRef}
        onKeyDown={onKeyDown}
        aria-label="Suggested accounts"
        className="flex flex-col gap-1.5"
      >
        {rows.map((row) => {
          const filtered = row.filtered_reason != null;
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
                    {row.account_label}
                  </span>
                  {row.suggested && (
                    <span className="rounded border border-accent/50 px-1 text-label text-accent">
                      Suggested
                    </span>
                  )}
                  {!filtered && <span className="ml-auto text-label text-muted">{forecastWords(row.tte_min, row.confidence)}</span>}
                </span>
                {filtered ? (
                  <span className="text-label text-muted">{row.filtered_reason}</span>
                ) : (
                  <span className="text-label text-muted">
                    {row.top_factors.join(" · ")}
                  </span>
                )}
                {whyOpen && !filtered && (
                  <span className="text-label text-muted">
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
