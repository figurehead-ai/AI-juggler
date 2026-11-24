/**
 * AJ-E2321: Settings › Desktop apps › Moves: every move AI Juggler knows
 * of, with Undo and Finish by its id (the Moves door's rows). AJ-E2325:
 * how long a finished move's backup (its Undo) is kept, 7 days by
 * default; the moves in rich mode (sticky header and footer: the backups'
 * total and, when the disk is nearly full, the offer to clear the older
 * ones). No moves: no section.
 */
import { useEffect, useState, type JSX } from "react";
import { Segmented } from "../../components/controls";
import { useMoves } from "../../components/switch/MovesDoor";
import { MovesPanel } from "../../components/switch/MovesBackups";
import { appSettingGet, appSettingSet } from "../../lib/ipc/settings";
import { PaneSection, SettingRow } from "./parts";

/** The setting's key and its choices, in days. */
export const KEEP_KEY = "moves.keep_backups";
type KeepChoice = "1" | "7" | "30" | "forever";
const KEEPS: { value: KeepChoice; label: string }[] = [
  { value: "1", label: "1 day" },
  { value: "7", label: "7 days" },
  { value: "30", label: "30 days" },
  { value: "forever", label: "Until I delete them" },
];

function KeepRow({ onChange }: { onChange: () => void }): JSX.Element {
  const [keep, setKeep] = useState<KeepChoice>("7");
  useEffect(() => {
    void appSettingGet(KEEP_KEY).then(
      (value) => setKeep(KEEPS.find((k) => k.value === value)?.value ?? "7"),
      () => {},
    );
  }, []);
  const choose = (value: KeepChoice) => {
    setKeep(value);
    void appSettingSet(KEEP_KEY, value).then(onChange, () => {});
  };
  return (
    <SettingRow
      title="Keep move backups for"
      description="Undo restores a move from its backup. Moves AI Juggler stopped part way keep theirs until you finish or undo them."
      testId="moves-keep"
      control={<Segmented label="Keep move backups for" value={keep} segments={KEEPS} onChange={choose} testId="moves-keep-choice" />}
    />
  );
}

export function MovesSection(): JSX.Element | null {
  const [moves, reload] = useMoves();
  if (!moves || moves.length === 0) return null;
  return (
    <PaneSection id="moves" title="Moves" footer="Each conversation moved to another App. Undo puts one back; Finish completes one AI Juggler stopped part way.">
      <KeepRow onChange={reload} />
      <li className="list-none">
        <MovesPanel moves={moves} onChanged={reload} />
      </li>
    </PaneSection>
  );
}
