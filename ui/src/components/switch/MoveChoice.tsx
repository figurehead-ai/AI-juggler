/**
 * ADR-0031 (D-93): how a Claude → Claude switch happens, Move (the
 * default when it can) or Copy, and what Move does, in plain words:
 * what happens to the original, that both profiles restart, and that
 * anything running in them stops and must be started again. Copy stays
 * as it always was (the rest of the switch screen).
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { switchMoveExecute, switchMoveOffer, switchMoves, type MoveDoneInfo, type MoveOfferInfo } from "../../lib/ipc/switch-move";
import { errorText } from "../../lib/bridge";
import { FailedStep } from "./FailedStep";
import { MoveDone, MoveUndo } from "./MoveDone";
import { PRIMARY } from "./SwitchDialogFrame";
import { ChoiceOption, Disclosure } from "../controls";
import { IconCarryIn, IconCopy } from "../icons-actions";

export type How = "move" | "copy";

/** Move when it can, else Copy. */
function defaultHow(offer: MoveOfferInfo | null): How {
  return offer?.offered ? "move" : "copy";
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
          if (alive) setOffer(found);
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
          <button type="button" data-autofocus onClick={close} className={PRIMARY} data-testid="switch-moved-close">
            Done
          </button>
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
  const choice = (target: string, source: string) =>
    offer && <MoveChoice offer={offer} value={how} onChange={setHow} target={target} source={source} onCheckAgain={() => setNonce((n) => n + 1)} />;
  const screen = (target: string, source: string) =>
    offer && (
      <div className="flex flex-col gap-5" data-testid="switch-oneclick">
        {choice(target, source)}
        <MoveBody offer={offer} target={target} source={source} takeNotes={takeNotes} onTakeNotes={setTakeNotes} />
      </div>
    );
  const button = (target: string) => <MoveButton target={target} onMove={() => void start()} />;
  // Enter: Move when it's chosen, else what the switch does.
  const enter = (copy: () => void) => (moving ? () => void start() : copy);
  return { offer, how, moving, running: phase.kind === "moving", start, frame, choice, screen, button, enter };
}

/** The last button with Move chosen. */
function MoveButton({ target, onMove }: { target: string; onMove: () => void }): JSX.Element {
  return (
    <button key="move-now" type="button" onClick={(event) => event.detail < 2 && onMove()} data-testid="switch-move-confirm" className={PRIMARY} aria-keyshortcuts="Enter">
      Move to {target}
    </button>
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

const plural = (n: number, one: string, many: string) => (n === 1 ? one : many);

/** A line the backend wrote as a clause, shown as a sentence: capital
 * first, full stop last. */
export function sentence(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) return trimmed;
  const capital = trimmed[0]!.toUpperCase() + trimmed.slice(1);
  return /[.!?…]$/.test(capital) ? capital : `${capital}.`;
}

/** "the conversation and its 2 sessions". */
function familyWords(offer: MoveOfferInfo): string {
  const others = offer.members.filter((m) => m.side === "moves").length - 1;
  return others > 0 ? `the conversation and its ${others} ${plural(others, "session", "sessions")}` : "the conversation";
}

/** A size in words ("1.2 GB"). */
function size(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${Math.round(bytes / 1e6)} MB`;
  return `${Math.max(1, Math.round(bytes / 1e3))} KB`;
}

function MoveChoice({
  offer,
  value,
  onChange,
  target,
  source,
  onCheckAgain,
}: {
  offer: MoveOfferInfo;
  value: How;
  onChange: (how: How) => void;
  target: string;
  source: string;
  /** Reads the offer again (Move waits on something that may be done now). */
  onCheckAgain?: () => void;
}): JSX.Element {
  return (
    <div className="flex flex-col gap-2" data-testid="switch-how">
      <span id="switch-how-legend" className="text-meta font-medium text-ink">
        How
      </span>
      <div role="radiogroup" aria-labelledby="switch-how-legend" className="grid grid-cols-1 gap-2 sm:grid-cols-2">
        <ChoiceOption
          type="radio"
          look="card"
          name="switch-how"
          icon={IconCarryIn}
          label={
            <>
              Move to account
              {offer.offered && <span className="ml-1.5 rounded-full border border-success/30 bg-success-soft px-1.5 py-px text-label font-medium text-success">Recommended</span>}
            </>
          }
          hint={`The same conversation continues on ${target}; ${source} no longer has it.`}
          reason={offer.why_not[0] && sentence(offer.why_not[0])}
          disabled={!offer.offered}
          checked={value === "move"}
          onChange={() => onChange("move")}
          data-testid="switch-how-move"
        />
        <ChoiceOption
          type="radio"
          look="card"
          name="switch-how"
          icon={IconCopy}
          label="Copy"
          // 0.4.1 (the rc6 walk): the original is said once, by the line
          // below the choice, so this hint says only where the copy goes.
          hint={`A copy continues on ${target}.`}
          checked={value === "copy"}
          onChange={() => onChange("copy")}
          data-testid="switch-how-copy"
        />
      </div>
      {!offer.offered && offer.why_not.length > 1 && (
        <ul className="ml-4 list-disc text-label text-muted" data-testid="switch-how-why">
          {offer.why_not.slice(1).map((why) => (
            <li key={why}>{sentence(why)}</li>
          ))}
        </ul>
      )}
      {!offer.offered && onCheckAgain && (
        <button type="button" onClick={onCheckAgain} className="w-fit text-label text-accent-hi underline-offset-2 hover:underline" data-testid="switch-how-again">
          Check again
        </button>
      )}
    </div>
  );
}

/** What Move does, on the confirm screen. */
function MoveBody({
  offer,
  target,
  source,
  takeNotes,
  onTakeNotes,
}: {
  offer: MoveOfferInfo;
  target: string;
  source: string;
  takeNotes: boolean;
  onTakeNotes: (take: boolean) => void;
}): JSX.Element {
  const moving = offer.members.filter((m) => m.side === "moves");
  const relinked = offer.members.filter((m) => m.side === "relinked");
  const restarts = offer.restarts.join(" and ");
  return (
    <div className="flex flex-col gap-4" data-testid="switch-move">
      <ul className="flex flex-col gap-2 text-body text-ink">
        <li data-testid="switch-move-what">
          <span className="font-medium">What moves:</span> {familyWords(offer)} ({size(offer.bytes)}), with every message, sub-session and rewind point. Nothing is copied, so it takes no extra room.
        </li>
        <li data-testid="switch-move-original">
          <span className="font-medium">The original:</span> {source} no longer lists it; {target} does, as the same conversation. Undo puts both back exactly.
        </li>
        {offer.app && restarts && (
          <li data-testid="switch-move-restart">
            <span className="font-medium">Restarts:</span> {restarts} quit and start again. Anything running in {offer.restarts.length === 1 ? "it" : "them"} stops, including other conversations&apos; work: start it again after the move.
          </li>
        )}
        <li data-testid="switch-move-model">
          <span className="font-medium">After:</span> it opens in {target}. AI Juggler sends no message; send one to check it answers on {target}.
        </li>
      </ul>
      {moving.length > 1 && (
        <Disclosure
          testId="switch-move-family"
          summary={`${moving.length} conversations${relinked.length > 0 ? `, and ${relinked.length} already on ${target} linked to it` : ""}`}
        >
          <ul className="mt-1 ml-5 list-disc text-label text-muted">
            {offer.members.map((m, index) => (
              <li key={`${m.title}-${index}`}>
                {m.title}
                {m.archived ? " (archived, stays archived)" : ""}
                {m.side === "relinked" ? ` (on ${target} already)` : ""}
                {m.folder_missing ? " (its folder is gone; its history is kept)" : ""}
              </li>
            ))}
          </ul>
        </Disclosure>
      )}
      {offer.notes.length > 0 && (
        <ul className="flex flex-col gap-1 text-label text-muted" data-testid="switch-move-notes">
          {offer.notes.map((note) => (
            <li key={note}>{sentence(note)}</li>
          ))}
        </ul>
      )}
      {offer.notes_conflict && (
        <ChoiceOption
          type="checkbox"
          label={`Use this conversation's project notes on ${target}`}
          hint={`Every conversation of this project on ${target} reads them. Off: ${target} keeps its own; either way, Undo restores them.`}
          checked={takeNotes}
          onChange={(event) => onTakeNotes(event.currentTarget.checked)}
          data-testid="switch-move-take-notes"
        />
      )}
      {offer.kept_outputs > 0 && (
        <p className="text-label text-muted" data-testid="switch-move-outputs">
          {offer.kept_outputs} {plural(offer.kept_outputs, "folder", "folders")} of tool output {plural(offer.kept_outputs, "stays", "stay")} on {source}, where old messages point to {plural(offer.kept_outputs, "it", "them")}. Cleanup keeps {plural(offer.kept_outputs, "it", "them")}.
        </p>
      )}
    </div>
  );
}
