/**
 * AJ-E3011 round 2 (#1), D-30: the pool's "team tools" (AI Juggler's
 * agent tools in the sessions it starts) as a newcomer reads them: what
 * they give, who they cover, what they cost, what happens without them,
 * and where to switch them off. D-30 asks once, the first time pools are
 * used; after that they're on, switched off per project or app in
 * Settings › Agent access (never "one switch for every pool": that read
 * made run 2's tester leave them off).
 *
 * The card (the setup's Review line, the pool's inspector) and Review's
 * "Team tools" line read one state, so they can't disagree.
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
  | { kind: "missing" };

export const TEAM_TOOLS = {
  title: "Let your assistants work as a team",
  gives: "They can message each other and share notes and tasks.",
  example: "Example: Claude sees that Codex is reviewing and sends it a note.",
  who: "It covers the sessions AI Juggler starts or connects in your pools, never a session outside a pool.",
  cost: "Nothing extra: a session uses its own turns only when it acts.",
  without: "Without it, the pool is set up, but nobody in it can message.",
  where: "Asked once. After that you can switch it off for a project or an app in",
  button: "Turn on team tools",
};

/** Settings › Agent access, its agent tools section (the switches per
 * project and app). */
export const AGENT_ACCESS = "Settings › Agent access";

export function teamTools(info: AgentBridgeInfo, projects: readonly ToolsPlace[], apps: readonly ToolsPlace[]): TeamTools {
  if (!info.available) return { kind: "missing" };
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

/** Review's line: "On: …", "Off: they can't message yet.", "Off for web:
 * they can't message there." */
export function teamToolsLine(state: TeamTools): string {
  switch (state.kind) {
    case "on":
      return "On: they can message each other and share notes and tasks.";
    case "off-here":
      return `Off for ${names(state.places)}: they can't message there.`;
    case "missing":
      return "Not in this build of AI Juggler: they can't message.";
    default:
      return "Off: they can't message yet.";
  }
}

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
