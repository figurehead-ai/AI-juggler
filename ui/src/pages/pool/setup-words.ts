/**
 * 0.4.6 (D-126 §3): the words of a pool's page while it's being set up —
 * its checklist (short names, each with its value under it, the open one
 * with what it does), the header's progress, and the calm live view once
 * it runs. The wizard's words are
 * its own (lib/pool-wizard-words); these are the made pool's.
 */
import { POOLS_GUIDE, type SETUP_STEPS } from "../../lib/pool-words";

type Step = (typeof SETUP_STEPS)[number];

/** The checklist's rows by their short names (the mock-up's). */
export const CHECK_TITLES: Record<Step, string> = {
  where: "Where",
  who: "Who's in",
  lead: "Who leads",
  everyone: "Roles",
  talk: "Messages",
  own: "On their own",
};

/** The team tools as the checklist's last item (D-30's one question,
 * there instead of a banner over the page). */
export const TOOLS_ITEM = {
  title: "Let them message each other",
  body: "Team tools give your pools' sessions messages, shared notes and tasks. Nothing extra to pay; nothing changes in your apps.",
  learn: "Learn more",
  guide: `${POOLS_GUIDE}#team-tools`,
  turnOn: "Turn on team tools",
  on: "On",
} as const;

/** The open item's one action, by step. */
export const OPEN_ACTION: Record<Step, string> = {
  where: "Choose where",
  who: "Add a member",
  lead: "Choose a lead",
  everyone: "Set the roles",
  talk: "Set who may message whom",
  own: "Choose what they may do",
};

export const SETUP = {
  title: (pool: string) => `Set up ${pool}`,
  /** The header's state pill. */
  pill: (done: number, total: number) => `Setting up · ${done} of ${total}`,
  foot: "Once a session joins and setup is done, this panel becomes the pool's live view.",
  hide: "Hide setup",
  settings: "All of this pool's settings",
  edit: "Edit setup",
};

/** What's left, under the checklist's title. */
export function leftWords(left: number, toolsLeft: boolean): string {
  if (left === 0) return "Every step is done";
  if (left === 1) return toolsLeft ? "One thing left before they can talk" : "One thing left";
  return `${left} things left`;
}
