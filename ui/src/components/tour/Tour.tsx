/**
 * AJ-E1546 — the guided tour on the real UI. Bot is the guide: each coach
 * mark carries Bot's mark and its scripted words (`lib/tour`), points at
 * its element by id with a ring around it, and shows the step's clip in
 * the card instead when the element isn't on screen (an empty state, a
 * page not open). Back, Next, Skip and a progress bar; Esc ends it, ← and
 * → step; Tab stays inside (a modal dialog); Reduce Motion has no motion
 * (the app's rule). "Ask Bot about this" shows only while Bot's AI is
 * ready, and Bot's own stop says how to set it up while none is (D-144).
 * Each step shown is stored as seen (per id).
 */
import { useCallback, useEffect, useRef, useState, type JSX } from "react";
import { createPortal } from "react-dom";
import { Button } from "../primitives";
import { Clip } from "../Clip";
import { keepTabInside, useFocusReturn } from "../form";
import { IconBot } from "../bot/bot-icons";
import { markDone, type TourStep } from "../../lib/tour";
import { useAiJob } from "../../lib/ai-availability";
import { closeBot, openBot, startDraft } from "../../lib/bot-store";

/** The element a step points at, when it is on screen. */
export function targetOf(step: TourStep): HTMLElement | null {
  const element = document.querySelector<HTMLElement>(step.target);
  if (!element) return null;
  const box = element.getBoundingClientRect();
  const shown = box.width > 0 && box.height > 0 && box.bottom > 0 && box.right > 0 && box.top < window.innerHeight && box.left < window.innerWidth;
  return shown ? element : null;
}

type Box = { top: number; left: number; width: number; height: number };

const CARD = 360;
const GAP = 12;
const EDGE = 16;

/** Where the card goes, without measuring it: beside a tall element (Bot's
 * panel), under one in the upper part of the window, else over it; always
 * inside the window. `top`/`bottom` and `left` in px; `middle`: centred
 * on the window's height. */
export function placeCard(target: Box, view: { width: number; height: number }): { top?: number; bottom?: number; left: number; middle?: boolean } {
  const clamp = (left: number) => Math.min(Math.max(EDGE, left), view.width - CARD - EDGE);
  if (target.height > view.height * 0.5) {
    const before = target.left - GAP - CARD;
    return { top: view.height / 2, middle: true, left: clamp(before >= EDGE ? before : target.left + target.width + GAP) };
  }
  const left = clamp(target.left + target.width / 2 - CARD / 2);
  if (target.top + target.height < view.height * 0.55) return { top: target.top + target.height + GAP, left };
  return { bottom: view.height - target.top + GAP, left };
}

export function Tour({ steps, onEnd }: { steps: readonly TourStep[]; onEnd: () => void }): JSX.Element | null {
  const [at, setAt] = useState(0);
  const step = steps[at];
  const [box, setBox] = useState<Box | null>(null);
  const card = useRef<HTMLDialogElement | null>(null);
  const chat = useAiJob("chat");
  const botReady = chat?.state === "ready";
  // Ended, the keyboard goes back where it was (never lost on the page).
  useFocusReturn();

  const measure = useCallback(() => {
    const element = step ? targetOf(step) : null;
    const rect = element?.getBoundingClientRect();
    setBox(rect ? { top: rect.top, left: rect.left, width: rect.width, height: rect.height } : null);
  }, [step]);

  // Each step: seen (stored per id), its element found once drawn, and
  // found again as the window changes. A step that opens something (Bot's
  // panel, the first stop) opens it, finds it once it is drawn (it loads
  // on first use), and closes it after.
  useEffect(() => {
    if (!step) return undefined;
    markDone(step.id);
    if (step.opens === "bot") openBot();
    const frame = window.requestAnimationFrame(measure);
    const timers = step.opens === "bot" ? [80, 250, 600].map((ms) => window.setTimeout(measure, ms)) : [];
    window.addEventListener("resize", measure);
    return () => {
      window.cancelAnimationFrame(frame);
      timers.forEach((timer) => window.clearTimeout(timer));
      window.removeEventListener("resize", measure);
      if (step.opens === "bot") closeBot();
    };
  }, [step, measure]);

  const last = at === steps.length - 1;
  const next = useCallback(() => (last ? onEnd() : setAt((now) => now + 1)), [last, onEnd]);
  const back = useCallback(() => setAt((now) => Math.max(0, now - 1)), []);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onEnd();
      } else if (event.key === "ArrowRight" && !(event.target instanceof HTMLButtonElement)) next();
      else if (event.key === "ArrowLeft" && !(event.target instanceof HTMLButtonElement)) back();
      else keepTabInside(card.current, event);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [next, back, onEnd]);

  if (!step) return null;
  const view = { width: window.innerWidth, height: window.innerHeight };
  const place = box ? placeCard(box, view) : null;
  const titleId = `tour-title-${step.id}`;
  return createPortal(
    <div className="fixed inset-0 z-50" data-testid="tour" data-step={step.id} data-anchored={box !== null}>
      {/* The page under the tour waits; the ring shows the element. */}
      <div aria-hidden="true" className="absolute inset-0" style={box ? undefined : { background: "color-mix(in srgb, var(--color-canvas) 62%, transparent)" }} />
      {box && (
        <div
          aria-hidden="true"
          className="pointer-events-none absolute rounded-[10px] transition-all duration-150"
          style={{
            top: box.top - 6,
            left: box.left - 6,
            width: box.width + 12,
            height: box.height + 12,
            boxShadow: "0 0 0 2px var(--color-accent), 0 0 0 9999px color-mix(in srgb, var(--color-canvas) 62%, transparent)",
          }}
          data-testid="tour-ring"
        />
      )}
      <dialog
        ref={card}
        open
        aria-modal="true"
        aria-labelledby={titleId}
        className={"absolute m-0 flex max-w-[calc(100vw-32px)] flex-col gap-3 rounded-[14px] border-0 bg-surface p-5 text-ink shadow-card inset-ring-1 inset-ring-hairline " + (place ? "w-[360px] " + (place.middle ? "-translate-y-1/2" : "") : "top-1/2 left-1/2 w-[440px] -translate-x-1/2 -translate-y-1/2")}
        style={place ? { top: place.top ?? "auto", bottom: place.bottom ?? "auto", left: place.left, right: "auto" } : undefined}
        data-testid="tour-card"
      >
        <div className="flex items-center gap-2.5">
          <span aria-hidden="true" className="flex size-7 items-center justify-center rounded-lg bg-accent-soft text-accent-hi">
            <IconBot width={16} height={16} />
          </span>
          <span className="text-meta font-medium text-ink-2">Bot</span>
          <span className="flex-1" />
          <span className="text-label text-muted tabular-nums" data-testid="tour-count">
            {at + 1} of {steps.length}
          </span>
        </div>
        <progress
          aria-label="Tour progress"
          max={steps.length}
          value={at + 1}
          className="h-1 w-full appearance-none overflow-hidden rounded-full bg-surface-3 [&::-moz-progress-bar]:bg-accent-fill [&::-webkit-progress-bar]:bg-surface-3 [&::-webkit-progress-value]:rounded-full [&::-webkit-progress-value]:bg-accent-fill"
        />
        {/* A new step puts the keyboard on its title (WIZ-5's rule). */}
        <h2 key={step.id} ref={(node) => node?.focus()} id={titleId} tabIndex={-1} className="type-section outline-none">
          {step.title}
        </h2>
        <p className="text-body text-ink-2">{botReady ? step.text : (step.textNoAi ?? step.text)}</p>
        {!box && step.clip && <Clip id={step.clip} label={step.title} />}
        {botReady && (
          <button
            type="button"
            className="self-start text-meta font-medium text-accent-hi hover:underline"
            onClick={() => {
              onEnd();
              // After the tour has closed (and closed the panel it opened).
              window.setTimeout(() => {
                openBot();
                startDraft(step.ask);
              }, 0);
            }}
            data-testid="tour-ask-bot"
          >
            Ask Bot about this
          </button>
        )}
        <div className="mt-1 flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={onEnd} data-testid="tour-skip">
            {last ? "Close" : "Skip the tour"}
          </Button>
          <span className="flex-1" />
          {at > 0 && (
            <Button variant="ghost" size="sm" onClick={back} data-testid="tour-back">
              Back
            </Button>
          )}
          <Button variant="primary" size="sm" onClick={next} data-testid="tour-next" data-autofocus>
            {last ? "Done" : "Next"}
          </Button>
        </div>
      </dialog>
    </div>,
    document.body,
  );
}
