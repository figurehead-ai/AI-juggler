/**
 * AJ-E3011, D-30: the pool's "team tools" (AI Juggler's agent tools in
 * the sessions it starts). D-30 asks once, the first time pools are used:
 * a question with Yes and Not now, on the pool page, as New pool's own
 * step and in Settings, one component (round 4, main's ruling). After
 * that they're on, switched off per project or app in Settings › Agent
 * access. Answered Not now (or off for one of the pool's projects or
 * Apps), every place that says so offers Turn on beside it.
 */
import { agentBridgeAnswer, agentBridgeSet, type AgentBridgeInfo, type AgentToolsScope } from "./ipc/agent-bridge";

/** A project or an App (its config folder) the pool covers. */
export interface ToolsPlace {
  id: string;
  name: string;
}

/** Where the pool stands. */
export type TeamTools =
  | { kind: "on" }
  /** Not asked yet (D-30's one question). */
  | { kind: "ask" }
  /** Asked, and answered Not now. */
  | { kind: "off" }
  /** On, but off for some of this pool's projects or Apps. */
  | { kind: "off-here"; places: (ToolsPlace & { scope: AgentToolsScope })[] }
  /** This build has none. */
  | { kind: "missing" }
  /** AJ-E3013: there, but in a folder others can write, so never run
   * (`folder`: the one that failed the check, home as ~). */
  | { kind: "unsafe"; folder: string };

/** D-30's one question, asked once, the first time pools are used (main's
 * ruling after three walks: a question, not a switch to find). */
export const TEAM_TOOLS = {
  question: "Let the sessions in your pools message each other and share notes and tasks?",
  gives: "They see who else is in their pool and what each is doing, and send each other notes and tasks: Claude sees that Codex is reviewing and sends it a note.",
  // Round 5 (Sam: "costs nothing extra: turns" was unclear).
  cost: "It costs nothing extra. Your assistants use their own turns only when they act.",
  // Casey's PC1 walk, after adding the connector: "does this change my
  // Claude app a second time?"
  local: "This is a setting in AI Juggler; it changes nothing in your apps.",
  // Sam's round-4 walk: "your pools" was vague; whom it covers, said.
  where: "This is for every pool you make; you can switch it off for a project or an app in",
  yes: "Yes, let them work together",
  notNow: "Not now",
  turnOn: "Turn on",
};

/** Settings › Agent access, its agent tools section (the switches per
 * project and app). */
export const AGENT_ACCESS = "Settings";

export function teamTools(info: AgentBridgeInfo, projects: readonly ToolsPlace[], apps: readonly ToolsPlace[]): TeamTools {
  if (!info.available) return info.unavailable === "unsafe_location" ? { kind: "unsafe", folder: info.unsafe_folder ?? "" } : { kind: "missing" };
  if (!info.asked) return { kind: "ask" };
  if (!info.on) return { kind: "off" };
  const places = [
    ...projects.filter((one) => info.off_projects.includes(one.id)).map((one) => ({ ...one, scope: "project" as const })),
    ...apps.filter((one) => info.off_profiles.includes(one.id)).map((one) => ({ ...one, scope: "profile" as const })),
  ];
  return places.length > 0 ? { kind: "off-here", places } : { kind: "on" };
}

/** "web", "web and docs", "web, docs and Codex CLI · Personal". */
const names = (places: readonly ToolsPlace[]) => {
  const all = places.map((one) => one.name);
  return all.length <= 1 ? (all[0] ?? "") : `${all.slice(0, -1).join(", ")} and ${all[all.length - 1]}`;
};

/** Where the team tools stand, in one line (the summary, Review, the
 * pool page's quiet line). Never "can't message" without its Turn on
 * beside it ({@link needsTurnOn}). */
export function teamToolsLine(state: TeamTools): string {
  switch (state.kind) {
    case "on":
      return "On: they can message each other and share notes and tasks.";
    case "ask":
      return "Not answered yet: they can't message each other until you say yes.";
    case "off-here":
      return `Off for ${names(state.places)}: they can't message each other there.`;
    case "missing":
      return "Not in this build of AI Juggler.";
    case "unsafe":
      return `AI Juggler's team tools can't run from a folder others can write${state.folder ? ` (${state.folder})` : ""}. Move AI Juggler to Applications, then open it again.`;
    default:
      return "Off: they can't message each other yet.";
  }
}

/** Why they can't run here, in one line; null when they can. Never with
 * Yes or Turn on beside it. */
export const unavailableLine = (info: AgentBridgeInfo): string | null => (info.available ? null : teamToolsLine(teamTools(info, [], [])));

/** The setup checklist's last line, every step done but the tools not on:
 * what's left and where (null once on). */
export function toolsLeftLine(state: TeamTools | null): string | null {
  switch (state?.kind) {
    case undefined:
    case "on":
      return null;
    case "ask":
      return "Ready once the team tools are on: answer the question at the top.";
    case "missing":
      return "Ready once the team tools are on: they're not in this build of AI Juggler.";
    case "unsafe":
      return "Ready once the team tools can run: what to do is at the top.";
    default:
      return "Ready once the team tools are on: Turn on is at the top.";
  }
}

/** Whether New pool asks D-30's question as its own step: unanswered (or
 * answered there, the step then says where they stand), and either able to
 * run or unable from AI Juggler's folder (the step says why and what to
 * do). Not in this build at all: nothing to ask. */
export const asksInNewPool = (info: AgentBridgeInfo | null, answeredHere: boolean): boolean =>
  info !== null && (!info.asked || answeredHere) && (info.available || info.unavailable === "unsafe_location");

/** Whether the line needs its one-click fix beside it. */
export const needsTurnOn = (state: TeamTools): boolean => state.kind === "ask" || state.kind === "off" || state.kind === "off-here";

/** Turn on what's off for this pool: the one question's yes, or the
 * project's and App's own switches. */
export async function turnOnTeamTools(state: TeamTools): Promise<AgentBridgeInfo> {
  const [first, ...rest] = state.kind === "off-here" ? state.places : [];
  if (!first) return agentBridgeAnswer(true);
  let info = await agentBridgeSet(first.scope, first.id, true);
  for (const place of rest) info = await agentBridgeSet(place.scope, place.id, true);
  return info;
}

/** Opens Settings › Agent access at its agent tools section. */
export function openAgentAccess(): void {
  window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "agent-tools" } }));
}

/** A draft pool's projects and CLI Apps, as the switches name them (a
 * desktop App's sessions start in its own window: not switched here). */
export function draftPlaces(draft: { every: boolean; projects: readonly (string | null)[]; members: readonly { ref: string; label: string }[] }, projects: readonly ToolsPlace[]): { projects: ToolsPlace[]; apps: ToolsPlace[] } {
  return {
    projects: draft.every ? [] : projects.filter((one) => draft.projects.includes(one.id)),
    apps: draft.members.filter((member) => member.ref.startsWith("app:cli:")).map((member) => ({ id: member.ref.slice("app:cli:".length), name: member.label })),
  };
}
