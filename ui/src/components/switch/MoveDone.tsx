/**
 * ADR-0031: what a move did, its four outcomes kept apart (the
 * conversation's files, the app's list, opened in the target profile, a
 * reply from the model, which only the user can check), and Undo.
 */
import { useState, type JSX } from "react";
import type { MoveDoneInfo, MoveUndoneInfo } from "../../lib/ipc/switch-move";
import type { MoveOutcomeInfo } from "../../types/generated/MoveOutcomeInfo";
import { switchMoveUndo } from "../../lib/ipc/switch-move";
import { errorText } from "../../lib/bridge";
import { IconCheck } from "../icons-controls";
import { IconClock, IconWarn } from "../icons-menu";
import { IconCopy } from "../icons-actions";
import { Button } from "../primitives";
import { Alert } from "../Alert";
import { Fact, FactPanel } from "./Facts";
import { sentence } from "./MoveChoice";

const ROWS: { key: "payload" | "catalog" | "opened" | "model"; label: string }[] = [
  { key: "payload", label: "Its history" },
  { key: "catalog", label: "The app's list" },
  { key: "opened", label: "Opened" },
  { key: "model", label: "A reply" },
];

/** Each outcome's icon, tone and word: checked, not as planned, or the
 * user's to check (the word said to screen readers; the icon is its look). */
function look(outcome: MoveOutcomeInfo): { icon: typeof IconCheck; tone: "done" | "warn" | "plain"; word: string } {
  if (outcome.state === "verified") return { icon: IconCheck, tone: "done", word: "Checked" };
  if (outcome.state === "failed") return { icon: IconWarn, tone: "warn", word: "Not as planned" };
  return { icon: IconClock, tone: "plain", word: "Yours to check" };
}

/** The text Copy details puts on the clipboard after an Undo: its words
 * and where what changed since the move is kept (never on screen, R4). */
export function undoneDetails(undone: MoveUndoneInfo, at: Date = new Date()): string {
  return [`AI Juggler · a move was undone · ${at.toISOString()}`, `What it said: ${undone.words}`, `Kept since the move: ${undone.kept || "nothing"}`].join("\n");
}

/** R4 (the 0.5.0 re-walk): Undo's end state, under its own heading. After
 * Undo the dialog still read "Moved to personal" over its four green rows,
 * with a raw /tmp path in the sentence; now it says where the conversation
 * is ("Back on work"), in words, the path only in Copy details. */
export function MoveUndone({ undone, source }: { undone: MoveUndoneInfo; source: string | null }): JSX.Element {
  const [copied, setCopied] = useState<"yes" | "no" | null>(null);
  const copy = () =>
    void navigator.clipboard?.writeText(undoneDetails(undone)).then(
      () => setCopied("yes"),
      () => setCopied("no"),
    );
  return (
    <div className="flex flex-col gap-5" data-testid="switch-undone">
      <div className="flex items-center gap-3">
        <span aria-hidden="true" className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-surface-2 text-ink-2">
          <IconCheck width={20} height={20} strokeWidth={2.5} />
        </span>
        <div className="flex min-w-0 flex-col gap-0.5">
          <h3 className="type-section" data-testid="switch-undone-title">
            {source ? `Back on ${source}` : "Move undone"}
          </h3>
          <p className="type-help" data-testid="switch-moved-undo-said">
            {sentence(undone.words)}
          </p>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <Button size="sm" variant="secondary" type="button" onClick={copy} data-testid="switch-undone-copy">
          <IconCopy aria-hidden="true" width={14} height={14} />
          Copy details
        </Button>
        {copied && (
          <output className="text-label text-muted">{copied === "yes" ? "Copied" : "Couldn't copy"}</output>
        )}
      </div>
    </div>
  );
}

/** Undo the move; once undone, `onUndone` takes the dialog to its end
 * state (MoveUndone); a refusal is said under the button. */
export function MoveUndo({ moveId, onUndone }: { moveId: string; onUndone: (undone: MoveUndoneInfo) => void }): JSX.Element {
  const [undo, setUndo] = useState<{ state: "idle" | "running" | "failed"; text?: string }>({ state: "idle" });
  const run = async () => {
    setUndo({ state: "running" });
    try {
      onUndone(await switchMoveUndo(moveId));
    } catch (cause) {
      setUndo({ state: "failed", text: errorText(cause) });
    }
  };
  return (
    <div className="flex flex-col items-start gap-1.5">
      <Button variant="secondary" disabled={undo.state === "running"} onClick={() => void run()} data-testid="switch-moved-undo">
        {undo.state === "running" ? "Undoing — both profiles restart…" : "Undo the move"}
      </Button>
      {undo.text && (
        <p className="text-label text-danger" data-testid="switch-moved-undo-failed">
          {sentence(undo.text)}
        </p>
      )}
    </div>
  );
}

export function MoveDone({ done, target, source = null }: { done: MoveDoneInfo; target: string; source?: string | null }): JSX.Element {
  const [undone, setUndone] = useState<MoveUndoneInfo | null>(null);
  if (undone) return <MoveUndone undone={undone} source={source} />;
  return (
    <div className="flex flex-col gap-6" data-testid="switch-moved">
      <div className="flex items-center gap-3">
        <span aria-hidden="true" className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-success-soft text-success">
          <IconCheck width={20} height={20} strokeWidth={2.5} />
        </span>
        <div className="flex min-w-0 flex-col gap-0.5">
          <h3 className="type-section">Moved to {target}</h3>
          <p className="type-help">On {target} now. Send a message there to check it answers.</p>
        </div>
      </div>
      <FactPanel label="What the move did">
        {ROWS.map(({ key, label }) => {
          const { icon, tone, word } = look(done[key]);
          return (
            <Fact key={key} icon={icon} label={label} tone={tone}>
              <span data-testid={`switch-moved-${key}`} data-state={done[key].state}>
                <span className="sr-only">{word}: </span>
                {sentence(done[key].detail)}
              </span>
            </Fact>
          );
        })}
      </FactPanel>
      {done.notes.length > 0 && (
        <Alert level="info" live={null} testId="switch-moved-notes">
          {done.notes.map((note) => (
            <p key={note}>{sentence(note)}</p>
          ))}
        </Alert>
      )}
      <MoveUndo moveId={done.move_id} onUndone={setUndone} />
    </div>
  );
}
