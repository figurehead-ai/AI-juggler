/**
 * AJ-E2327 — "Choose its folder…": an App whose sessions can't be shown
 * (nothing told which tool folder it uses) gets the one the user picks in
 * the system's picker (the same picker first run's "Import from another
 * folder…" opens). The folder is brought in where it is, never copied.
 * Nothing changes when the picker is cancelled.
 */
import { showToast } from "../components/Toast";
import { errorText } from "./bridge";
import { pickDirectory } from "./ipc/project-folders";
import { placesChooseFolder } from "./ipc/places";

/** Picks App `id`'s folder (named `label` in the words) and links it;
 * says what happened. A profile not added yet (`added: false`, first
 * run's Your apps, the 0.5.1 walk's F2) keeps the pick until its Add
 * links it: nothing is added here. Resolves whether a folder was taken. */
export async function chooseAppFolder(id: string, label: string, { added = true }: { added?: boolean } = {}): Promise<boolean> {
  const picked = await pickDirectory();
  if (picked === null) return false;
  try {
    const done = await placesChooseFolder(id, picked);
    const name = done.folder.name ?? picked;
    showToast(
      added
        ? { id: `place-folder-${id}`, tone: "success", message: `${label} uses ${name} now`, detail: "Its sessions show here; nothing was copied." }
        : { id: `place-folder-${id}`, tone: "success", message: `${label} will use ${name}`, detail: "When you add it. Nothing is added or copied until then." },
    );
    return true;
  } catch (cause) {
    showToast({ id: `place-folder-${id}`, tone: "warning", message: `${label}'s folder wasn't changed`, detail: errorText(cause) });
    return false;
  }
}
