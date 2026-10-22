/**
 * AJ-E1532 — a project's mark: its own icon (as a `data:` URL the backend
 * read from the project's folder) and the user's choice of letters, emoji
 * or colour (ENGINEERING-GUIDE U3: one IPC module per feature). In the
 * preview, icons come from the seam `__AIJ_PROJECT_ICONS__` (by project
 * id) or the replica fixture's, and a choice changes the preview's own
 * rows.
 */
import { invoke } from "./transport";
import { hasLiveBackend, MOCK_PROJECTS } from "../bridge";
import { initialsOf } from "../palette";
import type { MarkChoice } from "../../types/generated/MarkChoice";

/** A project's own icon, or `null` when it has none. */
export async function projectIconRead(projectId: string): Promise<string | null> {
  if (!hasLiveBackend()) {
    const seams = window as unknown as { __AIJ_PROJECT_ICONS__?: Record<string, string>; __AIJ_FIXTURE__?: { project_icons?: Record<string, string> } };
    return seams.__AIJ_PROJECT_ICONS__?.[projectId] ?? seams.__AIJ_FIXTURE__?.project_icons?.[projectId] ?? null;
  }
  return invoke<string | null>("projects_icon_read", { projectId });
}

/** Choose a project's mark (its clones share it); `{}` resets it. */
export async function projectMarkSet(projectId: string, choice: MarkChoice): Promise<void> {
  if (!hasLiveBackend()) {
    const project = MOCK_PROJECTS.find((row) => row.id === projectId);
    if (!project) throw { code: "not_found", retryable: false, user_message: "That project isn't there any more." };
    const kept: MarkChoice = {};
    if (choice.letters?.trim()) kept.letters = choice.letters.trim();
    if (choice.emoji?.trim()) kept.emoji = choice.emoji.trim();
    if (choice.swatch !== undefined) kept.swatch = choice.swatch;
    // Clones of one repository share the change, as in the app.
    const clones = MOCK_PROJECTS.filter((row) => row.id === projectId || (project.repo !== null && row.repo === project.repo));
    for (const row of clones) {
      const base = row.mark ?? { letters: initialsOf(row.name), swatch: 0, shows: "letters" as const };
      const own = base.choice?.letters ? initialsOf(row.name) : base.letters;
      row.mark = {
        ...base,
        letters: kept.letters ?? own,
        emoji: kept.emoji,
        swatch: kept.swatch ?? base.swatch,
        shows: kept.emoji ? "emoji" : !kept.letters && base.icon ? "icon" : "letters",
        choice: Object.keys(kept).length > 0 ? kept : undefined,
      };
    }
    return;
  }
  return invoke<void>("projects_mark_set", { projectId, choice });
}
