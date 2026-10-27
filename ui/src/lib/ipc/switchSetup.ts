/**
 * AJ-E1542: before a switch, what the target account lacks (MCP servers,
 * skills, commands, subagents, rules, plugins), and setting up what the
 * user picks. The preview and the tests read the seam `__AIJ_SWITCH_SETUP__`
 * instead of the backend.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";
import type { AppliedInfo } from "../../types/generated/AppliedInfo";
import type { OfferInfo } from "../../types/generated/OfferInfo";
import type { SetupCheckInfo } from "../../types/generated/SetupCheckInfo";
import type { SetUpInfo } from "../../types/generated/SetUpInfo";

export type { AppliedInfo, OfferInfo, SetupCheckInfo, SetUpInfo };

const seam = () =>
  (window as unknown as { __AIJ_SWITCH_SETUP__?: SetupCheckInfo }).__AIJ_SWITCH_SETUP__;

/** An example for tests and the gallery (set it as the seam): Personal
 * lacks a skill, a server that needs a token, and a skill that comes with
 * the claude.ai account. */
export const EXAMPLE_SETUP: SetupCheckInfo = {
  from: "Work Max",
  to: "Personal",
  already_there: 3,
  offers: [
    {
      id: "skill-release-notes",
      kind: "skill",
      name: "release-notes",
      scope: "user",
      verdict: "copy",
      note: null,
      suggested: true,
      about: null, steps: [], steps_page: null, steps_page_label: null,
    },
    {
      id: "mcp-github",
      kind: "mcp-server",
      name: "github",
      scope: "user",
      verdict: "needs-you",
      note: "it needs GITHUB_TOKEN in your environment: secrets are never copied",
      suggested: false,
      about: null, steps: [], steps_page: null, steps_page_label: null,
    },
    {
      id: "skill-team-style",
      kind: "skill",
      name: "team-style",
      scope: "user",
      verdict: "impossible",
      note: "it comes with the claude.ai account, so it can't be copied to another one",
      suggested: false,
      about: null,
      // The adapter's steps (claude_inventory::how_to_get), as it words them.
      steps: [
        "Open claude.ai settings in your browser, signed in as the account you're switching to.",
        "Find team-style there and turn it on.",
        "Come back here and choose Check again.",
      ],
      steps_page: "the target's tool",
      steps_page_label: "Open claude.ai settings",
    },
  ],
};

/** Kinds in words, as the app writes them ("the skill release-notes"). */
const KIND_WORDS: Record<string, string> = {
  "mcp-server": "MCP server",
  skill: "skill",
  command: "command",
  subagent: "subagent",
  rules: "rules file",
  "main-rules": "main rules file",
  plugin: "plugin",
  permissions: "permissions",
  "status-line": "status line",
};

/** What each preview set-up was, for its undo message. */
const previewSetUps = new Map<string, string>();

/** The preview's default: the target already has everything. */
const MOCK: SetupCheckInfo = { from: "Work Max", to: "Personal", already_there: 3, offers: [] };

/** What the target lacks before this switch (reads only). */
export async function switchSetupCheck(
  sessionId: string,
  targetProfileId: string,
): Promise<SetupCheckInfo> {
  if (!hasLiveBackend()) return seam() ?? MOCK;
  return invoke<SetupCheckInfo>("switch_setup_check", { sessionId, targetProfileId });
}

/** Sets up the picked offers on the target. */
export async function switchSetupApply(
  sessionId: string,
  targetProfileId: string,
  offers: string[],
): Promise<AppliedInfo[]> {
  if (!hasLiveBackend()) {
    const all = (seam() ?? MOCK).offers;
    return offers.map((id) => {
      const offer = all.find((candidate) => candidate.id === id);
      if (!offer) {
        return { offer: id, record: null, message: "It isn't on the source account any more", note: null };
      }
      const what = `${KIND_WORDS[offer.kind] ?? offer.kind} ${offer.name}`;
      previewSetUps.set(`record-${id}`, what);
      return { offer: id, record: `record-${id}`, message: `Set up the ${what}`, note: null };
    });
  }
  return invoke<AppliedInfo[]>("switch_setup_apply", { sessionId, targetProfileId, offers });
}

/** Every set-up still in place, newest first (the preview: what it set
 * up this session; tests set `__AIJ_SWITCH_SET_UPS__`). */
export async function switchSetupList(): Promise<SetUpInfo[]> {
  if (!hasLiveBackend()) {
    const seeded = (window as unknown as { __AIJ_SWITCH_SET_UPS__?: SetUpInfo[] })
      .__AIJ_SWITCH_SET_UPS__;
    return (
      seeded ??
      [...previewSetUps].map(([record, what]) => ({
        record,
        place: "Personal",
        what: `the ${what}`,
        at_ms: 0,
      }))
    );
  }
  return invoke<SetUpInfo[]>("switch_setup_list");
}

/** Undoes one set-up. */
export async function switchSetupUndo(record: string): Promise<string> {
  if (!hasLiveBackend()) {
    const what = previewSetUps.get(record) ?? record;
    previewSetUps.delete(record);
    const seam = window as unknown as { __AIJ_SWITCH_SET_UPS__?: SetUpInfo[] };
    seam.__AIJ_SWITCH_SET_UPS__ = seam.__AIJ_SWITCH_SET_UPS__?.filter((s) => s.record !== record);
    return `Removed the ${what}`;
  }
  return invoke<string>("switch_setup_undo", { record });
}

/** Stops keeping a set-up that can't be undone, leaving it as it is. */
export async function switchSetupForget(record: string): Promise<string> {
  if (!hasLiveBackend()) {
    const what = previewSetUps.get(record) ?? record;
    previewSetUps.delete(record);
    const seam = window as unknown as { __AIJ_SWITCH_SET_UPS__?: SetUpInfo[] };
    seam.__AIJ_SWITCH_SET_UPS__ = seam.__AIJ_SWITCH_SET_UPS__?.filter((s) => s.record !== record);
    return `Forgot the ${what}. It stays as it is now.`;
  }
  return invoke<string>("switch_setup_forget", { record });
}
