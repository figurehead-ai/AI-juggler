/**
 * AJ-E1542: before a switch, what the target account lacks (MCP servers,
 * skills, commands, subagents, rules, plugins), and setting up what the
 * user picks. What comes with the other account (a claude.ai skill, an
 * OpenAI plugin) is turned on there instead: each such offer carries
 * `get_it`, the steps, and a switch keeps a reminder until the account
 * has it (`switchSetupTurnOns`). The preview and the tests read the seams
 * `__AIJ_SWITCH_SETUP__` and `__AIJ_SWITCH_TURN_ONS__` instead of the
 * backend.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";
import type { AppliedInfo } from "../../types/generated/AppliedInfo";
import type { OfferInfo } from "../../types/generated/OfferInfo";
import type { SetupCheckInfo } from "../../types/generated/SetupCheckInfo";
import type { SetUpInfo } from "../../types/generated/SetUpInfo";
import type { GetItInfo } from "../../types/generated/GetItInfo";
import type { TurnOnInfo } from "../../types/generated/TurnOnInfo";

export type { AppliedInfo, GetItInfo, OfferInfo, SetupCheckInfo, SetUpInfo, TurnOnInfo };

/** The user guide's section on what the other account doesn't have
 * (the backend opens only this page and the ones it named). */
export const SWITCHING_GUIDE_URL =
  "https://github.com/figurehead-ai/AI-juggler/blob/main/docs/guide/switching.md#things-the-other-account-doesnt-have";

export { EXAMPLE_SETUP, EXAMPLE_SETUP_NOTES, EXAMPLE_SETUP_READY, EXAMPLE_SETUP_UNUSED } from "./switchSetupExamples";

const seam = () =>
  (window as unknown as { __AIJ_SWITCH_SETUP__?: SetupCheckInfo }).__AIJ_SWITCH_SETUP__;

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

/** The preview's reminders, from its switches. */
const previewTurnOns: TurnOnInfo[] = [];

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
    const check = seam() ?? MOCK;
    const all = check.offers;
    // As the backend: a reminder for each used thing to turn on.
    for (const offer of all) {
      if (offer.verdict !== "turn-on" || !offer.get_it || offer.used === false) continue;
      if (offer.can_copy && offers.includes(offer.id)) continue;
      if (previewTurnOns.some((turnOn) => turnOn.id === `turn-on-${offer.id}`)) continue;
      previewTurnOns.push({
        id: `turn-on-${offer.id}`,
        account: check.to,
        kind: offer.kind,
        name: offer.name,
        get_it: offer.get_it,
        at_ms: 0,
      });
    }
    return offers.map((id) => {
      const offer = all.find((candidate) => candidate.id === id);
      if (!offer) {
        return { offer: id, record: null, message: "It isn't on the source account any more", note: null };
      }
      if (offer.verdict === "turn-on" && !offer.can_copy) {
        return { offer: id, record: null, message: `${offer.name} isn't copied: turn it on for ${check.to} instead`, note: null };
      }
      // D-51: this project's notes, as the backend says it.
      const what =
        offer.kind === "memory"
          ? `${offer.count ?? 0} notes for this project from ${check.from}`
          : `${KIND_WORDS[offer.kind] ?? offer.kind} ${offer.name}`;
      previewSetUps.set(`record-${id}`, what);
      const done = offer.kind === "memory" ? "Copied" : "Set up";
      return { offer: id, record: `record-${id}`, message: `${done} the ${what}`, note: null };
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

/** What accounts still have to turn on after a switch, newest first; the
 * backend reads each account again, so this is Check again too. */
export async function switchSetupTurnOns(): Promise<TurnOnInfo[]> {
  if (!hasLiveBackend()) {
    const seeded = (window as unknown as { __AIJ_SWITCH_TURN_ONS__?: TurnOnInfo[] }).__AIJ_SWITCH_TURN_ONS__;
    return seeded ?? [...previewTurnOns];
  }
  return invoke<TurnOnInfo[]>("switch_setup_turn_ons");
}

/** Stops reminding about one thing. */
export async function switchSetupTurnOnDismiss(id: string): Promise<string> {
  if (!hasLiveBackend()) {
    const seam = window as unknown as { __AIJ_SWITCH_TURN_ONS__?: TurnOnInfo[] };
    const all = seam.__AIJ_SWITCH_TURN_ONS__ ?? previewTurnOns;
    const found = all.find((turnOn) => turnOn.id === id);
    seam.__AIJ_SWITCH_TURN_ONS__ = all.filter((turnOn) => turnOn.id !== id);
    const at = previewTurnOns.findIndex((turnOn) => turnOn.id === id);
    if (at >= 0) previewTurnOns.splice(at, 1);
    return `Stopped reminding you about ${found?.name ?? id}`;
  }
  return invoke<string>("switch_setup_turn_on_dismiss", { id });
}

/** Opens a page the backend named (an offer's or a reminder's, or the
 * switching guide) in the browser. The preview records it instead. */
export async function switchSetupOpen(url: string): Promise<void> {
  if (!hasLiveBackend()) {
    (window as unknown as { __AIJ_OPENED__?: string[] }).__AIJ_OPENED__ = [
      ...((window as unknown as { __AIJ_OPENED__?: string[] }).__AIJ_OPENED__ ?? []),
      url,
    ];
    return;
  }
  return invoke<void>("switch_setup_open", { url });
}
