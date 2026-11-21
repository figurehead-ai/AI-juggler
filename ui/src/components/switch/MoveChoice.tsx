/**
 * ADR-0031 (D-93): how a Claude → Claude switch happens, Move (the
 * default when it can) or Copy, and what Move does, in plain words:
 * what happens to the original, that both profiles restart, and that
 * anything running in them stops and must be started again. Copy stays
 * as it always was (the rest of the switch screen).
 * D-127: Move is never shown when something the user can't change stands
 * in its way (its kind is `setup`): Copy is the switch then, alone. It is
 * shown waiting only for what the user can clear, saying how.
 */
import { useEffect, useRef, useState, type JSX, type ReactNode } from "react";
import { switchMoveExecute, switchMoveOffer, switchMoves, type MoveDoneInfo, type MoveOfferInfo } from "../../lib/ipc/switch-move";
import { errorText } from "../../lib/bridge";
import { FailedStep } from "./FailedStep";
import { MoveDone, MoveUndo } from "./MoveDone";
import { MoveBody, MoveChoice, MoveHeader, type How } from "./MoveScreen";
import { Button } from "../primitives";

export type { How };
export { sentence } from "./MoveScreen";

/** D-127: the offer as the switch screen shows it: none when anything
 * the user can't clear stands in Move's way. */
function shownOffer(offer: MoveOfferInfo | null): MoveOfferInfo | null {
  return offer && offer.applies && !offer.blocks.some((block) => block.kind === "setup") ? offer : null;
}

/** Move when it can, else Copy. */
function defaultHow(offer: MoveOfferInfo | null): How {
  return offer && offer.blocks.length === 0 ? "move" : "copy";
}

/** What the move is doing, once the user clicked Move. */
type MovePhase =
  | { kind: "idle" }
  | { kind: "moving" }
  | { kind: "moved"; done: MoveDoneInfo }
  // A move that stopped part way names itself in the moves list, for Undo.
  | { kind: "failed"; message: string; cause: unknown; left: string | null };

/**
 * The switch screen's Move: the offer (null where moving doesn't apply),
 * Move or Copy (Move when it can), the notes' choice, and, once the user
 * clicked Move, the move's own screens (moving, moved with its outcomes
 * and Undo, failed), which the switch dialog shows in place of its own.
 * Read again with every check of the switch (`attempt`).
 */
export function useMoveFlow(sessionId: string, targetProfileId: string, attempt: number) {
  const [offer, setOffer] = useState<MoveOfferInfo | null>(null);
  const [chosen, setHow] = useState<How | null>(null);
  const [takeNotes, setTakeNotes] = useState(false);
  const [phase, setPhase] = useState<MovePhase>({ kind: "idle" });
  const [read, setRead] = useState<string | null>(null);
  // Bumped to read the offer again: a check again of the switch, or
  // Check again on a Move that can't happen yet (busy work finished).
  const [nonce, setNonce] = useState(0);
  const key = `${sessionId}|${targetProfileId}|${attempt}`;
  if (read !== key) {
    // A new switch (or a check again): read the offer afresh.
    setRead(key);
    setOffer(null);
    setHow(null);
    setTakeNotes(false);
    setPhase({ kind: "idle" });
    setNonce((n) => n + 1);
  }
  useEffect(() => {
    let alive = true;
    if (nonce > 0)
      void switchMoveOffer(sessionId, targetProfileId)
        .catch(() => null)
        .then((found) => {
          if (alive) setOffer(shownOffer(found));
        });
    return () => {
      alive = false;
    };
  }, [sessionId, targetProfileId, nonce]);
  const how: How = chosen ?? defaultHow(offer);
  const started = useRef(false);
  const start = async () => {
    if (started.current) return;
    started.current = true;
    setPhase({ kind: "moving" });
    try {
      setPhase({ kind: "moved", done: await switchMoveExecute(sessionId, targetProfileId, takeNotes) });
    } catch (cause) {
      started.current = false;
      const left = await switchMoves()
        .then((moves) => moves.find((m) => m.state === "interrupted")?.move_id ?? null)
        .catch(() => null);
      setPhase({ kind: "failed", message: errorText(cause), cause, left });
    }
  };
  /** What the dialog shows: the move's own screens once it started, else
   * the switch's `shown` ones. */
  const frame = (
    shown: { body: JSX.Element | null; footer: JSX.Element | null },
    target: string,
    holding: boolean,
    close: () => void,
    cancelButton: JSX.Element,
  ): { body: JSX.Element | null; footer: JSX.Element | null } => {
    if (phase.kind === "moving") return { body: <Moving target={target} holding={holding} />, footer: null };
    if (phase.kind === "moved") {
      return {
        body: <MoveDone done={phase.done} target={target} />,
        footer: (
          <Button variant="primary" type="button" data-autofocus onClick={close} data-testid="switch-moved-close">
            Done
          </Button>
        ),
      };
    }
    if (phase.kind === "failed") {
      return {
        body: (
          <div className="flex flex-col gap-4">
            <FailedStep target={target} message={phase.message} cause={phase.cause} results={[]} />
            {phase.left && <MoveUndo moveId={phase.left} />}
          </div>
        ),
        footer: cancelButton,
      };
    }
    return shown;
  };
  /** Move is chosen and can be: the confirm screen is Move's. */
  const moving = offer !== null && how === "move";
  /** The header, where moving applies: "Continue on <App>", from → to,
   * and a wizard's step marks under it; else just the marks. */
  const header = (marks: ReactNode, target: string, preflight: { source_account?: string | null; source_title?: string | null; target_tool: string } | null) =>
    offer && preflight ? (
      <MoveHeader offer={offer} source={preflight.source_account ?? "this account"} target={target} title={preflight.source_title ?? null} tool={preflight.target_tool} marks={marks} />
    ) : (
      marks
    );
  /** Move or Copy, first in the body. */
  const choice = (target: string, source: string) =>
    offer && <MoveChoice offer={offer} value={how} onChange={setHow} target={target} source={source} onCheckAgain={() => setNonce((n) => n + 1)} />;
  const screen = (target: string, source: string, tool: string) =>
    offer && (
      <div className="flex flex-col gap-5" data-testid="switch-oneclick">
        {choice(target, source)}
        <MoveBody offer={offer} target={target} source={source} tool={tool} takeNotes={takeNotes} onTakeNotes={setTakeNotes} />
      </div>
    );
  const button = (target: string) => <MoveButton target={target} onMove={() => void start()} />;
  // Enter: Move when it's chosen, else what the switch does.
  const enter = (copy: () => void) => (moving ? () => void start() : copy);
  return { offer, how, moving, running: phase.kind === "moving", start, frame, header, choice, screen, button, enter };
}

/** The last button with Move chosen. */
function MoveButton({ target, onMove }: { target: string; onMove: () => void }): JSX.Element {
  return (
    <Button variant="primary" key="move-now" type="button" onClick={(event) => event.detail < 2 && onMove()} data-testid="switch-move-confirm" aria-keyshortcuts="Enter">
      Move to {target}
    </Button>
  );
}

/** While the move runs. */
function Moving({ target, holding }: { target: string; holding: boolean }): JSX.Element {
  return (
    <p className="text-body text-ink" data-testid="switch-moving" aria-live="polite">
      Moving to {target}: both profiles quit, the conversation moves, and both start again. This takes a minute
      {holding ? ". It can't be stopped half way, so it finishes first" : ""}.
    </p>
  );
}
