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

export type { AppliedInfo, OfferInfo, SetupCheckInfo };

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
    },
    {
      id: "mcp-github",
      kind: "mcp-server",
      name: "github",
      scope: "user",
      verdict: "needs-you",
      note: "it needs GITHUB_TOKEN in your environment: secrets are never copied",
      suggested: false,
    },
    {
      id: "skill-team-style",
      kind: "skill",
      name: "team-style",
      scope: "user",
      verdict: "impossible",
      note: "it comes with the claude.ai account, so it can't be copied to another one",
      suggested: false,
    },
  ],
};

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
      return {
        offer: id,
        record: offer ? `record-${id}` : null,
        message: offer ? `Set up ${offer.name}` : "It isn't on the source account any more",
      };
    });
  }
  return invoke<AppliedInfo[]>("switch_setup_apply", { sessionId, targetProfileId, offers });
}

/** Undoes one set-up. */
export async function switchSetupUndo(record: string): Promise<string> {
  if (!hasLiveBackend()) return `Undid ${record}`;
  return invoke<string>("switch_setup_undo", { record });
}
