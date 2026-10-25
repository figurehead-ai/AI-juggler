/**
 * AJ-E1532 — "Change its mark…" from a project's menu: its colour (one
 * of the twelve), and what the mark shows — its own icon when it has one,
 * letters, or an emoji. A preview shows the mark as it will be. Clones
 * of one repository share the change. "Use its own" forgets the choice.
 */
import { useState, type JSX } from "react";
import { Button } from "./primitives";
import { Field, Input, Modal } from "./form";
import { MarkTile, useProjectIcon } from "./ProjectMark";
import { errorText, type ProjectInfo } from "../lib/bridge";
import { projectMarkSet } from "../lib/ipc/marks";
import { initialsOf, PALETTE, PALETTE_NAMES, swatchColor } from "../lib/palette";
import type { MarkChoice } from "../types/generated/MarkChoice";
import type { MarkShows } from "../types/generated/MarkShows";
import type { ProjectMarkInfo } from "../types/generated/ProjectMarkInfo";

export function MarkSheet({ project, onClose, onDone }: { project: ProjectInfo; onClose: () => void; onDone: () => void }): JSX.Element {
  const mark: ProjectMarkInfo = project.mark ?? { letters: initialsOf(project.name), swatch: 0, shows: "letters" };
  const icon = useProjectIcon(project.id, mark.icon);
  const chosen = mark.choice ?? {};
  const hasIcon = mark.icon !== undefined;
  const [swatch, setSwatch] = useState(mark.swatch);
  const [shows, setShows] = useState<MarkShows>(mark.shows);
  const [letters, setLetters] = useState(mark.letters);
  const [emoji, setEmoji] = useState(chosen.emoji ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const choice: MarkChoice = {};
  if (swatch !== mark.swatch || chosen.swatch !== undefined) choice.swatch = swatch;
  if (shows === "letters" && (letters !== mark.letters || chosen.letters !== undefined || hasIcon)) choice.letters = letters;
  if (shows === "emoji") choice.emoji = emoji;
  const preview: ProjectMarkInfo = { ...mark, swatch, shows, letters: shows === "letters" ? letters || "?" : mark.letters, emoji: shows === "emoji" ? emoji || "?" : undefined };

  const save = async (next: MarkChoice) => {
    setBusy(true);
    setError(null);
    try {
      await projectMarkSet(project.id, next);
      onDone();
    } catch (cause) {
      setError(errorText(cause));
      setBusy(false);
    }
  };

  return (
    <Modal title={`Change ${project.name}'s mark`} onClose={onClose}>
      <form
        className="flex flex-col gap-4"
        data-testid="mark-sheet"
        onSubmit={(event) => {
          event.preventDefault();
          void save(choice);
        }}
      >
        <div className="flex items-center gap-3">
          <MarkTile mark={preview} icon={shows === "icon" ? icon : null} size={40} />
          <p className="text-label leading-relaxed text-muted">
            {project.mark?.clone !== undefined ? "Its clones change with it: they share one mark." : "It shows wherever the project does."}
          </p>
        </div>
        <fieldset className="flex flex-col gap-2">
          <legend className="mb-1.5 text-label font-medium text-ink">Colour</legend>
          <div role="radiogroup" aria-label="Colour" className="flex flex-wrap gap-1.5">
            {/* Around the wheel, not in the order they are given out. */}
            {[...PALETTE.keys()].sort((a, b) => PALETTE[a]!.h - PALETTE[b]!.h).map((index) => (
              <label key={PALETTE_NAMES[index]} title={PALETTE_NAMES[index]} className="relative">
                <input
                  type="radio"
                  name="mark-colour"
                  className="peer sr-only"
                  checked={swatch === index}
                  onChange={() => setSwatch(index)}
                  data-testid={`mark-colour-${index}`}
                />
                <span className="sr-only">{PALETTE_NAMES[index]}</span>
                <span
                  aria-hidden="true"
                  className="block size-6 rounded-md ring-offset-2 ring-offset-surface peer-checked:ring-2 peer-checked:ring-ink peer-focus-visible:ring-2 peer-focus-visible:ring-accent"
                  style={{ background: swatchColor(index) }}
                />
              </label>
            ))}
          </div>
        </fieldset>
        <fieldset className="flex flex-col gap-2">
          <legend className="mb-1.5 text-label font-medium text-ink">It shows</legend>
          <div role="radiogroup" aria-label="It shows" className="flex flex-wrap gap-2">
            {(
              [
                ...(hasIcon ? [{ id: "icon", label: "Its icon" }] : []),
                { id: "letters", label: "Letters" },
                { id: "emoji", label: "An emoji" },
              ] as { id: MarkShows; label: string }[]
            ).map((option) => (
              <label
                key={option.id}
                className={"flex items-center gap-2 rounded-lg border px-3 py-1.5 text-sm " + (shows === option.id ? "border-accent/60 bg-accent-soft" : "border-hairline")}
              >
                <input type="radio" name="mark-shows" checked={shows === option.id} onChange={() => setShows(option.id)} data-testid={`mark-shows-${option.id}`} />
                {option.label}
              </label>
            ))}
          </div>
        </fieldset>
        {shows === "letters" && (
          <Field label="Letters" hint="One or two letters or digits.">
            <Input value={letters} maxLength={2} onChange={(event) => setLetters(event.target.value)} data-testid="mark-letters" />
          </Field>
        )}
        {shows === "emoji" && (
          <Field label="Emoji" hint="One emoji. Your Mac's emoji picker opens with Control-Command-Space.">
            <Input value={emoji} maxLength={16} placeholder="🚀" onChange={(event) => setEmoji(event.target.value)} data-testid="mark-emoji" />
          </Field>
        )}
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex items-center justify-end gap-2">
          {mark.choice && (
            <Button size="sm" variant="ghost" className="mr-auto" disabled={busy} onClick={() => void save({})} data-testid="mark-reset">
              Use its own
            </Button>
          )}
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" type="submit" disabled={busy} data-testid="mark-save">
            {busy ? "Saving…" : "Save"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
