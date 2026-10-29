/**
 * Examples of the switch's set-up list for the tests and the gallery (set
 * one as the seam `__AIJ_SWITCH_SETUP__`). Types only here, so a gallery
 * spec can import it without the app's transport.
 */
import type { GetItInfo } from "../../types/generated/GetItInfo";
import type { OfferInfo } from "../../types/generated/OfferInfo";
import type { SetupCheckInfo } from "../../types/generated/SetupCheckInfo";
import { EXAMPLE_GITHUB_OFFER } from "../../preview/mcp-secret-offer";

/** The steps a claude.ai skill's offer carries, for the examples (the
 * backend's words, filled in for Personal). */
const TURN_ON_COMPUTER_USE: GetItInfo = {
  steps: [
    "Sign in to claude.ai as Personal.",
    "In Settings > Capabilities, turn on Code execution and file creation (it may be called Create and edit files).",
    "In Customize > Skills, find computer-use under From Anthropic & Partners and turn it on.",
    "Start a session on Personal so it syncs.",
  ],
  command: null,
  open_url: "https://claude.ai/customize/skills",
  needs: [
    "Needs a Pro or Max plan: computer use isn't offered on Team or Enterprise.",
    "On a Team or Enterprise plan, your organization's owner may need to allow Skills first.",
  ],
  then: "It arrives when that session starts, or within about 10 minutes in one that's already running. Then choose Check again.",
};

/** An offer's fields that most examples share. */
const OFFER = {
  scope: "user",
  note: null,
  about: null,
  suggested: false,
  used: null,
  origin: "own",
  provided_by: null,
  get_it: null,
  can_copy: false,
} satisfies Partial<OfferInfo>;

/** A skill that comes with the claude.ai account, for the examples. */
function provided(id: string, name: string, used: boolean | null): OfferInfo {
  const get_it = {
    ...TURN_ON_COMPUTER_USE,
    steps: TURN_ON_COMPUTER_USE.steps.map((step) => step.replace("computer-use", name)),
    needs: name === "computer-use" ? TURN_ON_COMPUTER_USE.needs : TURN_ON_COMPUTER_USE.needs.slice(1),
  };
  return { ...OFFER, id, kind: "skill", name, verdict: "turn-on", used, origin: "provided", provided_by: "Anthropic", get_it };
}


/** An example for tests and the gallery (set it as the seam): this
 * conversation used a skill Personal can be given, a server that needs a
 * token (with the steps to give Personal its own) and a claude.ai skill
 * Personal turns on itself; two more claude.ai skills weren't used here. */
export const EXAMPLE_SETUP: SetupCheckInfo = {
  from: "Work Max",
  to: "Personal",
  already_there: 3,
  offers: [
    { ...OFFER, id: "skill-release-notes", kind: "skill", name: "release-notes", verdict: "copy", suggested: true, used: true },
    EXAMPLE_GITHUB_OFFER,
    provided("skill-computer-use", "computer-use", true),
    provided("skill-deep-research", "deep-research", false),
    provided("skill-setup-writing-style", "setup-writing-style", false),
  ],
};

/** Only what can be set up, all used here (the gallery's first state). */
export const EXAMPLE_SETUP_READY: SetupCheckInfo = {
  ...EXAMPLE_SETUP,
  offers: EXAMPLE_SETUP.offers.filter((offer) => offer.verdict !== "turn-on"),
};

/** Nothing the conversation used is missing: the maintainer's case, five
 * claude.ai skills Personal lacks, none used here. */
export const EXAMPLE_SETUP_UNUSED: SetupCheckInfo = {
  ...EXAMPLE_SETUP,
  offers: [
    provided("skill-built-in-browser", "built-in-browser", false),
    provided("skill-chrome-browser", "chrome-browser", false),
    provided("skill-computer-use", "computer-use", false),
    provided("skill-deep-research", "deep-research", false),
    provided("skill-setup-writing-style", "setup-writing-style", false),
  ],
};


/** D-51: Work Max's notes for this project that Personal lacks (3), one of
 * them kept as Personal has it, beside a skill used here. */
export const EXAMPLE_SETUP_NOTES: SetupCheckInfo = {
  ...EXAMPLE_SETUP,
  offers: [
    EXAMPLE_SETUP.offers[0],
    {
      ...OFFER,
      id: "memory-0123456789abcdef",
      kind: "memory",
      name: "notes for this project",
      scope: "project-private",
      verdict: "copy",
      about: "3 notes",
      count: 3,
      note: "Personal has its own project-codename.md with other words: it stays as it is, and Work Max's isn't copied",
    },
  ],
};

/** D-51 for a tool that keeps no notes (Codex): Work Max's notes for this
 * project, two of them offered for the handover, one left out for what
 * looks like a key; unticked, all three stay. */
export const EXAMPLE_SETUP_NOTES_HANDOVER: SetupCheckInfo = {
  ...EXAMPLE_SETUP,
  to: "Codex",
  already_there: 0,
  notes_stay: 3,
  offers: [
    {
      ...OFFER,
      id: "memory-fedcba9876543210",
      kind: "memory",
      name: "notes for this project",
      scope: "project-private",
      verdict: "handover",
      about: "2 notes",
      count: 2,
      note: "deploy.md looks like it holds a key or password, so it's left out",
    },
  ],
};
