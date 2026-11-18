/**
 * 0.4.6 (D-126 §3): the words only a pool's page says while it's being
 * set up — its checklist's open item and what's left, the header's
 * progress, the panel's foot, and the planned team's own lines. The
 * wizard's own words are lib/pool-wizard-words'; what both say — each
 * step's short name, a role's, a lead's, an App's kind, the Messages
 * ways — is lib/pool-words', and each step's value lib/pool-values'.
 */
import { POOLS_GUIDE, type SETUP_STEPS } from "../../lib/pool-words";

type Step = (typeof SETUP_STEPS)[number];

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

/** The planned team on the diagram, before any session has joined (its
 * cards' kinds, the lead's line and everyone's are pool-words'). */
export const PLANNED = {
  title: "Your team is drawn. No one has joined yet.",
  line: (where: string) => `Sessions show up here as they start in ${where}. Click a card to see what it may do.`,
  leadBody: "No session yet. Start one and it takes the lead.",
  appBody: (where: string) => `Its sessions in ${where} join as they start.`,
  everyoneSub: "every new session",
  adding: "Adding…",
};
