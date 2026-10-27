/**
 * §4.15, §8: a setting shows its new value at once, and a save that fails
 * puts the old one back and says why (the 2026-10-27 audit: Settings kept
 * showing a value that was never saved). One helper for every such
 * setting, so none forgets the revert.
 */
import { errorText } from "./bridge";

/** Why a save failed, and that the setting is as it was. */
export function revertedWords(cause: unknown): string {
  return `Couldn't save that: ${errorText(cause).replace(/\.$/, "")}. The setting is back to what it was.`;
}

/**
 * Shows `next`, saves it, and keeps what the save answered (or `next`);
 * if it fails, shows `previous` again and reports why.
 */
export async function saveOptimistic<T>({
  previous,
  next,
  show,
  save,
  onSaved,
  onFailed,
}: {
  previous: T;
  next: T;
  show: (value: T) => void;
  save: (value: T) => Promise<T | void>;
  onSaved?: (saved: T) => void;
  onFailed: (words: string) => void;
}): Promise<void> {
  show(next);
  try {
    const answered = await save(next);
    const saved = (answered ?? next) as T;
    show(saved);
    onSaved?.(saved);
  } catch (cause) {
    show(previous);
    onFailed(revertedWords(cause));
  }
}
