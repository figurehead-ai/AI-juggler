/**
 * AJ-E1547 (PAT-OSP-3) — our sheet before a system prompt: "macOS will
 * now ask…", why, a picture of the dialog, which button to click and what
 * happens if the user declines; Continue and Not now. After Continue it
 * waits while the system's dialog is up, then says the outcome: allowed,
 * or not with where to change it later.
 *
 * Rich mode (DESIGN-SYSTEM §4.2, DLG-1): the one dialog frame, sticky
 * header and footer, the body the only part that scrolls. Not now is the
 * calm default (focus starts there; Esc means Not now); Continue is the
 * one primary.
 *
 * `OsPromptHost` (mounted once in the app) shows it when the backend's
 * gate asks (`os-prompt://ask`) and answers with `os_prompt_answer`.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { DialogFrame, DialogOverlay } from "./DialogFrame";
import { keepTabInside, useFocusReturn } from "./form";
import { Button } from "./primitives";
import { OsPromptPicture } from "./OsPromptPicture";
import { SYSTEM_NAME } from "../lib/machine-words";
import { onOsPromptAsk, onOsPromptOutcome, osPromptAnswer, type OsPromptInfo, type OsPromptOutcomeInfo } from "../lib/ipc/os-prompts";

/** Where the sheet is: asking, waiting on the system's dialog, or done. */
export type SheetPhase = { kind: "ask" } | { kind: "waiting" } | { kind: "done"; outcome: OsPromptOutcomeInfo };

/** The sheet's heading for each phase. */
export function sheetTitle(prompt: OsPromptInfo, phase: SheetPhase): string {
  if (phase.kind === "done") return phase.outcome.status === "allowed" ? `${prompt.title}: allowed` : prompt.title;
  if (phase.kind === "waiting") return `Answer ${SYSTEM_NAME}'s dialog`;
  return `${SYSTEM_NAME} will now ask`;
}

export function OsPromptSheet({
  prompt,
  phase,
  onContinue,
  onNotNow,
  onDone,
}: {
  prompt: OsPromptInfo;
  phase: SheetPhase;
  onContinue: () => void;
  onNotNow: () => void;
  onDone: () => void;
}): JSX.Element {
  const boxRef = useRef<HTMLDivElement | null>(null);
  const calmRef = useRef<HTMLButtonElement | null>(null);
  useFocusReturn();
  useEffect(() => {
    // The calm choice has focus in each phase (Not now, then Done).
    if (phase.kind !== "waiting") calmRef.current?.focus();
  }, [phase.kind]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (phase.kind === "ask") onNotNow();
      if (phase.kind === "done") onDone();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [phase.kind, onNotNow, onDone]);
  const footer =
    phase.kind === "ask" ? (
      <>
        <Button ref={calmRef} size="sm" variant="secondary" onClick={onNotNow} data-testid="os-prompt-not-now">
          Not now
        </Button>
        <Button size="sm" variant="primary" onClick={onContinue} data-testid="os-prompt-continue">
          Continue
        </Button>
      </>
    ) : phase.kind === "done" ? (
      <Button ref={calmRef} size="sm" variant="primary" onClick={onDone} data-testid="os-prompt-done">
        Done
      </Button>
    ) : (
      <output className="text-xs text-muted">Waiting for your answer in {SYSTEM_NAME}'s dialog…</output>
    );
  return (
    <DialogOverlay ref={boxRef}>
      <DialogFrame
        size="M"
        className="sheen rounded-card border border-hairline bg-surface shadow-card"
        onKeyDown={(event) => keepTabInside(boxRef.current, event)}
        // eslint-disable-next-line jsx-a11y/prefer-tag-over-role -- the one dialog frame (DLG-1) is a div inside its overlay; a native <dialog> would leave the frame's sticky parts and focus trap
        role="dialog"
        aria-modal="true"
        aria-labelledby="os-prompt-title"
        aria-describedby="os-prompt-why"
        data-testid="os-prompt-sheet"
        data-phase={phase.kind}
        headerClassName="px-5 pt-5 pb-3"
        header={
          <>
            <h2 id="os-prompt-title" className="type-section">
              {sheetTitle(prompt, phase)}
            </h2>
            <p className="mt-1 text-xs text-muted">{prompt.title}</p>
          </>
        }
        bodyClassName="flex flex-col gap-3 px-5 pb-4"
        footer={footer}
      >
        {phase.kind === "done" ? (
          <p className="text-sm leading-relaxed text-ink" data-testid="os-prompt-outcome" data-status={phase.outcome.status}>
            {phase.outcome.words}
          </p>
        ) : (
          <>
            <p id="os-prompt-why" className="text-xs leading-relaxed text-ink-2">
              {prompt.why}
            </p>
            {prompt.dialog && <OsPromptPicture dialog={prompt.dialog} />}
            <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-xs">
              <dt className="font-medium text-ink">What to click</dt>
              <dd className="text-ink-2" data-testid="os-prompt-click">{prompt.click}</dd>
              <dt className="font-medium text-ink">If you don't</dt>
              <dd className="text-ink-2">{prompt.if_declined}</dd>
              {prompt.change_later && (
                <>
                  <dt className="font-medium text-ink">Later</dt>
                  <dd className="text-ink-2">Change it any time in {prompt.change_later}.</dd>
                </>
              )}
            </dl>
          </>
        )}
      </DialogFrame>
    </DialogOverlay>
  );
}

/** The one host: the gate's sheet, wherever the app is. */
export function OsPromptHost(): JSX.Element | null {
  const [open, setOpen] = useState<{ prompt: OsPromptInfo; phase: SheetPhase } | null>(null);
  useEffect(() => {
    const ask = onOsPromptAsk((prompt) => setOpen({ prompt, phase: { kind: "ask" } }));
    const told = onOsPromptOutcome((outcome) =>
      setOpen((now) => (now && now.prompt.id === outcome.id ? { ...now, phase: { kind: "done", outcome } } : now)),
    );
    return () => {
      void ask.then((stop) => stop());
      void told.then((stop) => stop());
    };
  }, []);
  if (!open) return null;
  const answer = (go: boolean) => {
    void osPromptAnswer(open.prompt.id, go);
    setOpen(go ? { ...open, phase: { kind: "waiting" } } : null);
  };
  return <OsPromptSheet prompt={open.prompt} phase={open.phase} onContinue={() => answer(true)} onNotNow={() => answer(false)} onDone={() => setOpen(null)} />;
}
