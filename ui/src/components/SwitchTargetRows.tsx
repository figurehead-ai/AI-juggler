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
import { RoomChip } from "./RoomChip";
import { runwayDetail } from "./AccountRunway";
import { roomMarkFrom } from "../lib/room-mark";
import { forecastWords } from "../lib/quotaWindow";
import { useEffect, useRef, useState, type JSX } from "react";
import { noteSwitchPick } from "../lib/ipc/switchExecute";
import { statsEvent } from "../lib/ipc/usage-stats";
import type { SwitchTargetData } from "../lib/ipc/switch-targets";
import { lowRoomWords, ROOM_ORDER_WORDS, outOfRoomConsequence, outOfRoomWords } from "../lib/room";
import { DESKTOP_ONLY, roomless, type TargetRoom } from "../lib/room-targets";
import { creditsConsequence, usingCreditsWords } from "../lib/credits";
import { useTargetApp } from "../lib/apps";
import { afterColon, BadgeLine, BadgeTag, RoomTag, SuggestedTag, TargetButton, TargetLogo } from "./switch/TargetCard";

/** "Suggested": the one with the most room (the one order by room). One
 * look everywhere (TAG-5): the accent tag. */
export function SuggestedMark({ className = "" }: { className?: string }): JSX.Element {
  return (
    <span className={"inline-flex shrink-0 " + className}>
      <SuggestedTag />
    </span>
  );
}

/** An account's room under its name, in the one runway's words; out of
 * room says until when and what picking it anyway does (the 0.2 walk, F1:
 * it was offered like any other). */
export function RoomLine({ name, room, testId = "target-room", chip = true }: { name: string; room: TargetRoom; testId?: string; chip?: boolean }): JSX.Element {
  // 0.4.1: out or low, the one mark every screen shows, whatever this
  // target is ranked (on the source's own account it can't take it).
  const mark = chip && room.kind !== "credits" && room.left !== null ? roomMarkFrom(room.left, room.resetsAtMs, false, undefined, room.age ?? null) : null;
  const line = roomWords(name, room, testId);
  if (!mark) return line;
  return (
    <span className="flex flex-col items-start gap-1">
      <RoomChip mark={mark} testId="switch-target-room" />
      {line}
    </span>
  );
}

function roomWords(name: string, room: TargetRoom, testId: string): JSX.Element {
  if (room.kind === "out") {
    return (
      <span className="flex flex-col text-meta text-ink-2" data-testid={testId} data-room="out">
        <span className="font-medium text-ink">{outOfRoomWords(room.resetsAtMs)}</span>
        <span>{outOfRoomConsequence(name, room.resetsAtMs)}</span>
      </span>
    );
  }
  // 0.3.2: its window is used up, but its credits carry a session on.
  if (room.kind === "credits" && room.credits) {
    return (
      <span className="flex flex-col text-meta text-ink-2" data-testid={testId} data-room="credits">
        <span className="font-medium text-ink">{usingCreditsWords(room.credits)}</span>
        <span>{creditsConsequence(room.resetsAtMs)}</span>
      </span>
    );
  }
  if (room.kind === "low" && room.left !== null) {
    return (
      <span className="text-meta text-ink-2" data-testid={testId} data-room="low">
        {lowRoomWords(name, room.left)}
        {room.resets ? ` ${room.resets}.` : ""}
      </span>
    );
  }
  return (
    <span className="text-meta text-ink-2" data-testid={testId} data-room={room.kind}>
      {runwayDetail(room)}
    </span>
  );
}

/** The badge: how much carries, a tag (0.5.0: neutral, never an amber
 * hatch; the words say "· Beta", D-127, and never "proven"). */
export function FidelityBadge({ badge, validated }: { badge: string; validated: boolean }) {
  return <BadgeTag badge={badge} validated={validated} />;
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
  // The App each target is reached through (the 0.4.6 RC walk K3: a Claude
  // app profile's folder read "Claude Code CLI · Work Max").
  const appOf = useTargetApp();
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
    buttons[(index + delta + buttons.length) % buttons.length]!.focus();
  };
  return (
    <div className="flex flex-col gap-3" data-testid="switch-target-rows">
      <div className="flex items-baseline justify-between gap-3">
        <p className="type-section">Ranked for you</p>
        <button type="button" aria-expanded={whyOpen} className="rounded text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent" onClick={() => setWhyOpen((open) => !open)}>
          Why this order?
        </button>
      </div>
      {whyOpen && (
        <p className="text-meta text-ink-2" data-testid="why-this-order">
          {ROOM_ORDER_WORDS}
        </p>
      )}
      <ul ref={listRef} onKeyDown={onKeyDown} aria-label="Switch destinations, ranked" className="flex flex-col gap-2.5">
        {targets.map((target) => {
          const filtered = target.filtered_reason != null;
          const app = appOf({ folder: target.account_id, tool: target.provider, label: target.account_label });
          const forecast = !filtered && !roomless(target) ? forecastWords(target.tte_min, target.confidence) : null;
          const factors = [...target.top_factors, ...(target.first_run ? ["its first-run setup is done as it switches"] : [])].join(" · ");
          return (
            <li key={target.recipe_id} className="flex flex-col gap-1">
              <TargetButton
                data-row
                disabled={filtered}
                aria-pressed={target.suggested}
                onClick={() => pick(target)}
                suggested={target.suggested && !filtered}
                logo={<TargetLogo tool={target.provider ?? null} kind={app.kind} place={app.mark} folderId={target.account_id} label={target.account_label} />}
                name={app.name}
                sub={filtered ? target.filtered_reason : `On ${target.room?.account ?? target.account_label}${forecast ? ` · ${forecast}` : ""}`}
                tags={
                  !filtered && (target.room || target.suggested) && (
                    <>
                      {target.room && <RoomTag room={target.room} />}
                      {target.suggested && <SuggestedTag />}
                    </>
                  )
                }
                detail={
                  <>
                    <BadgeLine badge={target.badge} validated={target.validated} />
                    {!filtered && target.room && <RoomLine name={target.room.account ?? target.account_label} room={target.room} chip={false} />}
                    {!filtered && target.signed_out && target.desktop && <span data-testid="target-sign-in">{DESKTOP_ONLY}</span>}
                    {!filtered && target.cross_provider && target.drops.length > 0 && <span data-testid="target-drop-list">Doesn't carry: {afterColon(target.drops)}</span>}
                    {/* The top two, or with "Why this order?" open, every input. */}
                    {!filtered && (whyOpen ? <span>{target.factors.map(([words]) => words).join(" · ")}</span> : factors && <span>{factors}</span>)}
                  </>
                }
              />
              {target.held_back === "set_up" && <StartThereLink accountId={target.account_id} label={target.account_label} />}
            </li>
          );
        })}
      </ul>
      <p className="text-meta text-ink-2">What carries is checked for each pair of tools, never assumed.</p>
    </div>
  );
}

/** 0.3.4: a target held back because its tool's first-run setup isn't
 * done there: New session on it does it (the switch's own check refuses
 * it until then). */
export function StartThereLink({ accountId, label, onStart }: { accountId: string; label: string; onStart?: () => void }): JSX.Element {
  return (
    <button
      type="button"
      onClick={() => {
        onStart?.();
        window.dispatchEvent(new CustomEvent("aij:new-session", { detail: { accountId } }));
      }}
      data-testid="continue-set-up"
      className="w-fit rounded px-1 text-label text-accent-hi outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
    >
      Start a session on {label} ›
    </button>
  );
}
