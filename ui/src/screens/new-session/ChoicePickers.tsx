/**
 * AJ-E3207 — Change folder and Change app, one step from the stated
 * choice: the shared Picker (the pickers rule), its button the link's
 * words. The folder picker opens on Recent and Pinned, then All
 * projects; the App picker on Suggested (the folder's suggestion, even
 * when the person picked another) and the Apps used in this project, then
 * All, the ones that can't start now last; a list too short for headings
 * keeps that order and its suggestion says "Suggested". Past 8 a search
 * field has focus. Out of room stays listed and choosable, its hint says
 * until when.
 */
import type { JSX } from "react";
import { Picker, type PickerOption } from "../../components/controls";
import { GROUPS_FROM } from "../../components/controls/Picker";
import { IconProjects, IconTerminal } from "../../components/icons";
import { IconStar } from "../../components/icons-actions";
import { runwayOf } from "../../components/AccountRunway";
import { pickDirectory } from "../../lib/ipc/project-folders";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { homeRelative } from "../../lib/project-words";
import { agoWords } from "../../lib/time-words";
import { outOfRoomWords } from "../../lib/room";
import { roomMarkOf } from "../../lib/room-mark";
import { appGroups, suggestedMarkOn, type AppChoice, type appsByRoom } from "../../lib/new-session-choice";
import { folderGroups, offeredProjects, workedAt } from "../../lib/new-session-folder";
import type { NewSessionData } from "./useNewSessionData";

/** The picker's "Choose another folder…" line (a project's value is its
 * root, an absolute path, so never this). */
const CHOOSE = "choose";
/** Change app's two lines that set or clear the project's default App
 * (PRINCIPLES' worked example: "Always use for web"); an App's value is
 * its folder id, never these. */
const ALWAYS = "always:set";
const NO_DEFAULT = "always:clear";

export function FolderPicker({ projects, root, onPick, now }: { projects: readonly ProjectInfo[]; root: string | null; onPick: (root: string) => void; now: number }): JSX.Element {
  const offered = offeredProjects(projects);
  const byId = new Map(offered.map((project) => [project.id, project.root]));
  const options: PickerOption[] = [
    { value: CHOOSE, label: "Choose another folder…", hint: "Or drop one on this window", pinned: true },
    ...offered.map((project) => {
      const at = workedAt(project);
      return {
        value: project.root,
        label: project.name,
        hint: [homeRelative(project.root), at > 0 ? agoWords(at, now) : null].filter(Boolean).join(" · "),
        empty: project.history_sessions + project.instance_count === 0,
        icon: IconProjects,
      };
    }),
  ];
  if (root && !offered.some((project) => project.root === root)) options.push({ value: root, label: root.split("/").pop() || root, hint: homeRelative(root), icon: IconProjects });
  const groups = folderGroups(projects).map((group) => ({ label: group.label, values: group.values.map((id) => byId.get(id)!).filter(Boolean) }));
  return (
    <Picker
      value={root}
      options={options}
      groups={groups}
      onChange={async (value) => {
        if (value !== CHOOSE) return onPick(value);
        const picked = await pickDirectory();
        if (picked) onPick(picked);
      }}
      label="Change folder"
      ariaLabel="Change folder"
      heading="Where to work"
      searchLabel="Search projects"
      allHeading="All projects"
      size="sm"
      look="link"
      testId="change-folder"
    />
  );
}

export function AppPicker({
  data,
  ranked,
  choice,
  suggested = choice,
  project,
  onPick,
  onAlways,
  now,
}: {
  data: NewSessionData;
  ranked: ReturnType<typeof appsByRoom>;
  choice: AppChoice | null;
  /** What New session suggests for this folder (`choice` unless the
   * person picked another). */
  suggested?: AppChoice | null;
  project: ProjectInfo | null;
  onPick: (folderId: string) => void;
  /** Sets (an App's folder id) or clears (`null`) the project's default. */
  onAlways?: (folderId: string | null) => void;
  now: number;
}): JSX.Element {
  // The pickers rule at any length (F-rc3f-1): the folder's suggestion and
  // the Apps related to it first, then the rest by room; the shared Picker
  // heads them only from six Apps, so a short list is ordered so, and the
  // suggestion says it is one.
  const groups = appGroups(suggested, ranked, project);
  const first = groups.flatMap((group) => group.values);
  const ordered = [...first.flatMap((id) => ranked.filter((row) => row.recipe.account_id === id)), ...ranked.filter((row) => !first.includes(row.recipe.account_id))];
  // Under its heading from six on; above that, its hint says so.
  // SK-6: never on an App that can't run now (the one offered instead).
  const tip = ranked.length < GROUPS_FROM ? suggestedMarkOn(suggested, ranked) : null;
  const options: PickerOption[] = ordered.map(({ recipe, kind }) => {
    const account = data.accountNameOf(recipe.account_id);
    // 0.4.1: out or low in the one mark's words ("Out · resets 9:06 PM"),
    // as its chip says it everywhere else.
    const mark = kind === "out" || kind === "low" ? roomMarkOf(data.displayOf(recipe.account_id), data.sessionsOf(recipe.account_id), now) : null;
    const room =
      kind === "blocked"
        ? "Not signed in"
        : mark
          ? mark.label
          : kind === "out"
            ? outOfRoomWords(data.roomOf(recipe.account_id).resetsAtMs, now)
            : data.roomKnown && kind !== "unknown"
              ? runwayOf(data.displayOf(recipe.account_id), data.sessionsOf(recipe.account_id)).words
              : null;
    return {
      value: recipe.account_id,
      label: data.appNameOf(recipe),
      // Not signed in: no account is in use, so none is named (the rc12
      // walk: "f…@example.com · Not signed in").
      hint: (kind === "blocked" ? [room] : [recipe.account_id === tip ? "Suggested" : null, account, room]).filter(Boolean).join(" · ") || undefined,
      empty: kind === "blocked",
      icon: IconTerminal,
    };
  });
  // With a project: make the App chosen now its default ("Always use for
  // web"), or clear the one it has; both first, as Change folder's
  // "Choose another folder…" is.
  const fixed = project?.default_account_id ? ranked.find((row) => row.recipe.account_id === project.default_account_id)?.recipe : undefined;
  if (project && onAlways) {
    const head: PickerOption[] = [];
    if (choice && choice.recipe.account_id !== fixed?.account_id)
      head.push({ value: ALWAYS, label: `Always use for ${project.name}`, hint: data.appNameOf(choice.recipe), pinned: true, icon: IconStar });
    if (fixed) head.push({ value: NO_DEFAULT, label: `No default for ${project.name}`, hint: "Back to the usual choice", pinned: true, icon: IconStar });
    options.unshift(...head);
  }
  return (
    <Picker
      value={choice?.recipe.account_id ?? null}
      options={options}
      groups={groups}
      onChange={(value) => {
        if (value === ALWAYS) return choice && onAlways?.(choice.recipe.account_id);
        if (value === NO_DEFAULT) return onAlways?.(null);
        onPick(value);
      }}
      // F-rc3g-5: the App shown, picked here, is the person's own: a note
      // on what a folder change did to it goes (as any pick's does).
      repick
      label="Change app"
      ariaLabel="Change app"
      heading="Which app"
      searchLabel="Search apps"
      allHeading="All apps"
      size="sm"
      look="link"
      testId="change-app"
    />
  );
}
