/**
 * AJ-E2321: Settings › Desktop apps › Moves: every move AI Juggler knows
 * of, with Undo and Finish by its id (the Moves door's rows). No moves:
 * no section.
 */
import type { JSX } from "react";
import { MoveRows, useMoves } from "../../components/switch/MovesDoor";
import { PaneSection } from "./parts";

export function MovesSection(): JSX.Element | null {
  const [moves, reload] = useMoves();
  if (!moves || moves.length === 0) return null;
  return (
    <PaneSection id="moves" title="Moves" footer="Each conversation moved to another App. Undo puts one back; Finish completes one AI Juggler stopped part way.">
      <div className="px-4">
        <MoveRows moves={moves} onChanged={reload} />
      </div>
    </PaneSection>
  );
}
