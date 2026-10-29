/**
 * D-51: the set-up list's row for this project's notes, the memory a tool
 * keeps for itself about the project (the maintainer, 2026-10-28: "the
 * switch check offers to copy this project's memory to the new account,
 * the same way it offers missing MCP servers and skills, and copies only
 * with my approval"). One row stands for all of them, as a question, never
 * ticked for the user. A note the other account has with other words is
 * kept as it is, and the row says so. The consequence, an example and the
 * guide's section are under "What does this mean?", as on every row.
 *
 * When the other account's tool keeps no notes like these (Codex), the
 * same row asks whether to include them in the handover, the file the new
 * session reads first (verdict `handover`): ticked, the switch quotes their
 * text there. A note that looks like it holds a key or password is left
 * out, and the row says which.
 */
import type { JSX } from "react";
import type { OfferInfo, SetupCheckInfo } from "../../lib/ipc/switchSetup";
import { GUIDE } from "../../lib/switch-words";
import { Meaning } from "../SwitchSetupTurnOn";
import { IN_HANDOVER } from "../../lib/useSwitchSetup";

/** The offer kind the backend gives this project's notes. */
export const NOTES_KIND = "memory";

/** The verdict when the other tool keeps no notes: quoted in the handover. */
export { IN_HANDOVER };

/** "Work Max's notes for this project (2): include them in the handover?" */
export function handoverQuestion(count: number, from: string): string {
  return count === 1
    ? `${from}'s note for this project (1): include it in the handover?`
    : `${from}'s notes for this project (${count}): include them in the handover?`;
}

/** "Work Max's notes for this project (3) aren't on Personal: copy them?" */
export function notesQuestion(count: number, from: string, to: string): string {
  return count === 1
    ? `${from}'s note for this project (1) isn't on ${to}: copy it?`
    : `${from}'s notes for this project (${count}) aren't on ${to}: copy them?`;
}

/** What ticking it does, and what it never does. */
export function notesConsequence(to: string): string {
  return `Copied as new files when you switch; nothing ${to} has is replaced. You can undo it after.`;
}

/** "What does this mean?" for the notes: what they are, and an example. */
export function notesMeaning(from: string, to: string): { text: string; example: string } {
  return {
    text: `The tool keeps notes it wrote for itself about this project (what it learned, what you asked it to remember) in each account's own folder, so ${to} starts without ${from}'s. Ticked, AI Juggler copies the ones ${to} doesn't have and adds them to ${to}'s list of notes. A note ${to} has with other words is kept as it is. Undo takes out what was copied while it's unchanged.`,
    example: `a note that the project's codename is Bluebird is copied, so ${to} knows it too.`,
  };
}

/** What ticking it does when the other tool keeps no notes. */
export function handoverConsequence(to: string): string {
  return `${to}'s tool keeps no notes like these. Ticked, their text goes in the handover ${to} reads first, for this switch only; nothing is written to ${to}'s own files.`;
}

/** "What does this mean?" when they go in the handover. */
export function handoverMeaning(from: string, to: string): { text: string; example: string } {
  return {
    text: `The tool keeps notes it wrote for itself about this project (what it learned, what you asked it to remember) in ${from}'s own folder. ${to}'s tool has no place for them, so AI Juggler can quote them in the handover file the new session reads first, under their own heading. A note that looks like it holds a key or password is never quoted.`,
    example: `a note that the project's codename is Bluebird is quoted, so ${to} knows it too.`,
  };
}

export function NotesRow({
  offer,
  check,
  picked,
  onToggle,
}: {
  offer: OfferInfo;
  check: SetupCheckInfo;
  picked: ReadonlySet<string>;
  onToggle: (id: string) => void;
}): JSX.Element {
  const id = `switch-setup-${offer.id}`;
  const handover = offer.verdict === IN_HANDOVER;
  const count = offer.count ?? 0;
  return (
    <li className="flex items-start gap-2" data-testid="switch-setup-row" data-verdict={offer.verdict} data-kind={NOTES_KIND}>
      <input id={id} type="checkbox" className="mt-1" checked={picked.has(offer.id)} onChange={() => onToggle(offer.id)} />
      <div className="flex min-w-0 flex-col gap-1">
        <label htmlFor={id} className="text-body text-ink" data-testid="switch-notes-question">
          {handover ? handoverQuestion(count, check.from) : notesQuestion(count, check.from, check.to)}
        </label>
        {offer.note && (
          <span className="text-meta text-ink" data-testid={handover ? "switch-notes-left-out" : "switch-notes-kept"}>
            {/[.!?]$/.test(offer.note) ? offer.note : `${offer.note}.`}
          </span>
        )}
        <span className="text-meta text-ink" data-testid="switch-setup-consequence">
          {handover ? handoverConsequence(check.to) : notesConsequence(check.to)}
        </span>
        <Meaning {...(handover ? handoverMeaning(check.from, check.to) : notesMeaning(check.from, check.to))} guide={GUIDE.notes} />
      </div>
    </li>
  );
}
