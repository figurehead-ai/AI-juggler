/**
 * AJ-E2106 — the registry's icon names, drawn. One stroke icon per
 * action, the same in every menu and in ⌘K. Decoration only: the label
 * names the action, so every icon is `aria-hidden`.
 */
import type { JSX, SVGProps } from "react";
import type { ActionIconName } from "../lib/actions";
import { IconPin, IconProfiles, IconProjects, IconStateHandover } from "./icons";
import {
  IconArchive,
  IconBell,
  IconCarryIn,
  IconCompare,
  IconCopy,
  IconEditor,
  IconFolderOpen,
  IconGoTo,
  IconInfo,
  IconKey,
  IconLocate,
  IconMute,
  IconPage,
  IconPlus,
  IconRemove,
  IconRename,
  IconReplay,
  IconSliders,
  IconSnooze,
  IconStar,
  IconStop,
  IconTag,
  IconTrash,
} from "./icons-actions";

/** Every name the registry may use, and its drawing. */
export const ACTION_ICONS: Record<ActionIconName, (props: SVGProps<SVGSVGElement>) => JSX.Element> = {
  "go-to": IconGoTo,
  page: IconPage,
  project: IconProjects,
  // The hand-over state's own arrows: carrying a conversation over.
  "carry-over": IconStateHandover,
  "carry-in": IconCarryIn,
  rename: IconRename,
  snooze: IconSnooze,
  mute: IconMute,
  wake: IconBell,
  copy: IconCopy,
  pin: IconPin,
  stop: IconStop,
  new: IconPlus,
  editor: IconEditor,
  finder: IconFolderOpen,
  mark: IconTag,
  compare: IconCompare,
  account: IconProfiles,
  "sign-in": IconKey,
  settings: IconSliders,
  default: IconStar,
  replay: IconReplay,
  archive: IconArchive,
  relocate: IconLocate,
  remove: IconRemove,
  delete: IconTrash,
  info: IconInfo,
};

export function ActionIcon({ name, size = 14, className = "" }: { name: ActionIconName; size?: number; className?: string }): JSX.Element {
  const Icon = ACTION_ICONS[name];
  return <Icon width={size} height={size} aria-hidden="true" focusable="false" className={"shrink-0 " + className} data-icon={name} />;
}
