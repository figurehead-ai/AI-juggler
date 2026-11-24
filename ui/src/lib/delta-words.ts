/**
 * 0.5.1 F71: the one update a switch back sends the original (the
 * backend's `delta_turn_text`, the words the model reads), read back for
 * the person: its lead line, each turn as "You" or "Reply" with its words,
 * grouped by where it happened when the work went through several places,
 * and the closing notes. Nothing is dropped: every line of the text is in
 * one of these.
 */

export interface DeltaTurn {
  who: "You" | "Reply";
  text: string;
}

export interface DeltaGroup {
  /** "On Personal", when the work went through several places. */
  where: string | null;
  turns: DeltaTurn[];
}

export interface DeltaRead {
  lead: string;
  groups: DeltaGroup[];
  notes: string[];
}

const TURN = /^- \[(user|assistant)\] ?(.*)$/;
const WHERE = /^On (.+):$/;

export function readDelta(text: string): DeltaRead {
  const lines = text.split("\n").map((line) => line.trim()).filter((line) => line !== "");
  const [lead = "", ...rest] = lines;
  const groups: DeltaGroup[] = [];
  const notes: string[] = [];
  let group: DeltaGroup | null = null;
  for (const line of rest) {
    const turn = TURN.exec(line);
    if (turn) {
      if (!group) groups.push((group = { where: null, turns: [] }));
      group.turns.push({ who: turn[1] === "user" ? "You" : "Reply", text: turn[2] ?? "" });
      continue;
    }
    const where = WHERE.exec(line);
    if (where) {
      groups.push((group = { where: `On ${where[1]}`, turns: [] }));
      continue;
    }
    notes.push(line);
  }
  return { lead, groups: groups.filter((g) => g.turns.length > 0), notes };
}
