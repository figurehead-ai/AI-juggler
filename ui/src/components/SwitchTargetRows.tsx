/**
 * AJ-E209 — the badged, explainable switch destinations (the
 * handover section's ranked rows; the RankedRows pattern from E611
 * with the E2 badge layer on top). Every row carries its ordered-pair
 * badge ("Full story, fresh start · Beta") plus validated/unvalidated TEXT (never
 * colour-only, WCAG 2.2); cross-Provider rows always show the badge
 * AND the Drop list (D-11: "beta" is never mistaken for whole-file
 * fidelity). Top-2 factors in words + confidence + TTE; "Why this
 * order?" expands every input incl. switch cost. Hard-filtered rows
 * stay visible greyed with their reason. Suggest-only — the click
 * (the host's switch flow) is the user's. A11y: a labelled list of
 * buttons, ↑/↓ roving focus, no nested interactives.
 */
import { forecastWords } from "../lib/quotaWindow";
import { useEffect, useRef, useState, type JSX } from "react";
import { noteSwitchPick, statsEvent, type SwitchTargetData } from "../lib/bridge";
import { lowRoomWords, ROOM_ORDER_WORDS, outOfRoomConsequence, outOfRoomWords } from "../lib/room";
import type { TargetRoom } from "../lib/room-targets";
import { creditsConsequence, usingCreditsWords } from "../lib/credits";

/** "Suggested": the one with the most room (the one order by room). */
export function SuggestedMark({ className = "" }: { className?: string }): JSX.Element {
  return (
    <span className={"shrink-0 rounded border border-accent/50 px-1 text-label text-accent-hi " + className} data-testid="suggested-mark">
      Suggested
    </span>
  );
}

/** An account's room under its name, in the one runway's words; out of
 * room says until when and what picking it anyway does (the 0.2 walk, F1:
 * it was offered like any other). */
export function RoomLine({ name, room, testId = "target-room" }: { name: string; room: TargetRoom; testId?: string }): JSX.Element {
  if (room.kind === "out") {
    return (
      <span className="flex flex-col text-label text-danger" data-testid={testId} data-room="out">
        <span className="font-medium">{outOfRoomWords(room.resetsAtMs)}</span>
        <span>{outOfRoomConsequence(name, room.resetsAtMs)}</span>
      </span>
    );
  }
  // 0.3.2: its window is used up, but its credits carry a session on.
  if (room.kind === "credits" && room.credits) {
    return (
      <span className="flex flex-col text-label text-ink" data-testid={testId} data-room="credits">
        <span className="font-medium">{usingCreditsWords(room.credits)}</span>
        <span className="text-muted">{creditsConsequence(room.resetsAtMs)}</span>
      </span>
    );
  }
  if (room.kind === "low" && room.left !== null) {
    return (
      <span className="text-label text-warning" data-testid={testId} data-room="low">
        {lowRoomWords(name, room.left)}
        {room.resets ? ` ${room.resets}.` : ""}
      </span>
    );
  }
  return (
    <span className="text-label text-muted" data-testid={testId} data-room={room.kind}>
      {room.resets ?? room.words}
    </span>
  );
}

/** The badge chip: solid when validated, hatched while beta. The text
 * label (the badge string itself + the validated line) carries the
 * state — the hatching is decoration only, legible in greyscale. */
export function FidelityBadge({ badge, validated }: { badge: string; validated: boolean }) {
  return (
    <span
      data-testid="fidelity-badge"
      data-validated={validated ? "true" : "false"}
      className={
        "inline-flex items-center rounded border px-1.5 py-px text-label " +
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
      {!validated && " · Not proven yet"}
    </span>
  );
}

export function SwitchTargetRows({
  targets,
  onPick,
}: {
  /** In the one order by room, each with its room (`rankTargets`). */
  targets: Array<SwitchTargetData & { room?: TargetRoom }>;
  onPick: (target: SwitchTargetData) => void;
}): JSX.Element {
  const [whyOpen, setWhyOpen] = useState(false);
  const listRef = useRef<HTMLUListElement>(null);
  // AJ-E1103: a suggestion shown, then taken or passed over.
  const suggested = targets.find((target) => target.suggested && !target.filtered_reason);
  const suggestedProvider = suggested?.provider ?? "other";
  useEffect(() => {
    if (suggested) statsEvent("usage.suggestion", { provider: suggestedProvider, action: "shown" });
  }, [suggested?.recipe_id, suggestedProvider]); // eslint-disable-line react-hooks/exhaustive-deps
  const pick = (target: SwitchTargetData) => {
    if (suggested) {
      statsEvent("usage.suggestion", {
        provider: suggestedProvider,
        action: target.recipe_id === suggested.recipe_id ? "accepted" : "dismissed",
      });
    }
    noteSwitchPick(target.suggested);
    onPick(target);
  };
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
        aria-label="Switch destinations, ranked"
        className="flex flex-col gap-1.5"
      >
        {targets.map((target) => {
          const filtered = target.filtered_reason != null;
          return (
            <li key={target.recipe_id}>
              <button
                type="button"
                data-row
                disabled={filtered}
                aria-pressed={target.suggested}
                onClick={() => pick(target)}
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
                  {target.suggested && !filtered && <SuggestedMark />}
                  {!filtered && <span className="ml-auto text-label text-muted">{forecastWords(target.tte_min, target.confidence)}</span>}
                </span>
                {target.room && !filtered && <RoomLine name={target.account_label} room={target.room} />}
                {target.cross_provider && target.drops.length > 0 && (
                  <span className="text-label text-warning" data-testid="target-drop-list">
                    Doesn't carry: {target.drops.join(", ")}
                  </span>
                )}
                {filtered ? (
                  <span className="text-label text-muted">{target.filtered_reason}</span>
                ) : (
                  <span className="text-label text-muted">
                    {target.top_factors.join(" · ")}
                  </span>
                )}
                {whyOpen && !filtered && (
                  <span className="text-label text-muted">
                    {target.factors.map(([words]) => words).join(" · ")}
                  </span>
                )}
              </button>
            </li>
          );
        })}
      </ul>
      <p className="text-label text-muted">
        What carries is checked for each pair of tools, never assumed.
      </p>
    </div>
  );
}
