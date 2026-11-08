/**
 * ADR-0031: what a move did, its four outcomes kept apart (the
 * conversation's files, the app's list, opened in the target profile, a
 * reply from the model, which only the user can check), and Undo.
 */
import { useState, type JSX } from "react";
import type { MoveDoneInfo } from "../../lib/ipc/switch-move";
import type { MoveOutcomeInfo } from "../../types/generated/MoveOutcomeInfo";
import { switchMoveUndo } from "../../lib/ipc/switch-move";
import { errorText } from "../../lib/bridge";
import { IconCheck } from "../icons-controls";
import { IconClock, IconWarn } from "../icons-menu";
import { Button } from "../primitives";
import { sentence } from "./MoveChoice";

const ROWS: { key: "payload" | "catalog" | "opened" | "model"; label: string }[] = [
  { key: "payload", label: "Its history" },
  { key: "catalog", label: "The app's list" },
  { key: "opened", label: "Opened" },
  { key: "model", label: "A reply" },
];

function Mark({ outcome }: { outcome: MoveOutcomeInfo }): JSX.Element {
  if (outcome.state === "verified") return <IconCheck aria-label="Checked" width={14} height={14} strokeWidth={2.5} className="text-success" />;
  if (outcome.state === "failed") return <IconWarn aria-label="Not as planned" width={14} height={14} className="text-danger" />;
  return <IconClock aria-label="Yours to check" width={14} height={14} className="text-muted" />;
}

export function MoveDone({ done, target }: { done: MoveDoneInfo; target: string }): JSX.Element {
  const [undo, setUndo] = useState<{ state: "idle" | "running" | "done" | "failed"; text?: string }>({ state: "idle" });
  const run = async () => {
    setUndo({ state: "running" });
    try {
      setUndo({ state: "done", text: await switchMoveUndo(done.move_id) });
    } catch (cause) {
      setUndo({ state: "failed", text: errorText(cause) });
    }
  };
  return (
    <div className="flex flex-col gap-4" data-testid="switch-moved">
      <h3 className="text-title text-ink">Moved to {target}</h3>
      <dl className="grid grid-cols-[auto_auto_1fr] items-baseline gap-x-2 gap-y-1.5 text-body">
        {ROWS.map(({ key, label }) => (
          <div key={key} className="contents" data-testid={`switch-moved-${key}`} data-state={done[key].state}>
            <span className="translate-y-[2px]">
              <Mark outcome={done[key]} />
            </span>
            <dt className="font-medium text-ink">{label}</dt>
            <dd className="text-muted">{sentence(done[key].detail)}</dd>
          </div>
        ))}
      </dl>
      {done.notes.length > 0 && (
        <ul className="flex flex-col gap-1 text-label text-muted" data-testid="switch-moved-notes">
          {done.notes.map((note) => (
            <li key={note}>{sentence(note)}</li>
          ))}
        </ul>
      )}
      <div className="flex flex-col items-start gap-1.5">
        {undo.state !== "done" && (
          <Button variant="secondary" disabled={undo.state === "running"} onClick={() => void run()} data-testid="switch-moved-undo">
            {undo.state === "running" ? "Undoing — both profiles restart…" : "Undo the move"}
          </Button>
        )}
        {undo.text && (
          <p className={"text-label " + (undo.state === "failed" ? "text-danger" : "text-ink")} data-testid="switch-moved-undo-said">
            {undo.text}
          </p>
        )}
      </div>
    </div>
  );
}
