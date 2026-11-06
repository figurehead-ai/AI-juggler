/**
 * AJ-E3207 — Change folder and Change app, one step from the stated
 * choice: the shared Picker (the pickers rule), its button the link's
 * words. The folder picker opens on Recent and Pinned, then All
 * projects; the App picker on Suggested and the Apps used in this
 * project, then All, the ones that can't start now last. Past 8 a search
 * field has focus. Out of room stays listed and choosable, its hint says
 * until when.
 */
import type { JSX } from "react";
import { Picker, type PickerOption } from "../../components/controls";
import { IconProjects, IconTerminal } from "../../components/icons";
import { runwayOf } from "../../components/AccountRunway";
import { pickDirectory } from "../../lib/ipc/project-folders";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { homeRelative } from "../../lib/project-words";
import { agoWords } from "../../lib/time-words";
import { outOfRoomWords } from "../../lib/room";
import { appGroups, folderGroups, offeredProjects, workedAt, type AppChoice, type appsByRoom } from "../../lib/new-session-choice";
import type { NewSessionData } from "./useNewSessionData";

/** The picker's "Choose another folder…" line (a project's value is its
 * root, an absolute path, so never this). */
const CHOOSE = "choose";

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
  project,
  onPick,
  now,
}: {
  data: NewSessionData;
  ranked: ReturnType<typeof appsByRoom>;
  choice: AppChoice | null;
  project: ProjectInfo | null;
  onPick: (folderId: string) => void;
  now: number;
}): JSX.Element {
  const options: PickerOption[] = ranked.map(({ recipe, kind }) => {
    const account = data.accountNameOf(recipe.account_id);
    const room =
      kind === "blocked"
        ? "Not signed in"
        : kind === "out"
          ? outOfRoomWords(data.roomOf(recipe.account_id).resetsAtMs, now)
          : data.roomKnown && kind !== "unknown"
            ? runwayOf(data.displayOf(recipe.account_id), data.sessionsOf(recipe.account_id)).words
            : null;
    return {
      value: recipe.account_id,
      label: data.appNameOf(recipe),
      hint: [account, room].filter(Boolean).join(" · ") || undefined,
      empty: kind === "blocked",
      icon: IconTerminal,
    };
  });
  return (
    <Picker
      value={choice?.recipe.account_id ?? null}
      options={options}
      groups={appGroups(choice, ranked, project)}
      onChange={onPick}
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
