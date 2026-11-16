/**
 * AJ-E3013 (D-116, D-119): AI Juggler's connector in a desktop App (a
 * Claude app profile's Code tab; the Codex app), on the user's say-so,
 * and a pool's desktop members as the canvas shows them. Only the app's
 * own window calls these; an agent can't.
 *
 * The preview never writes anything: it keeps each App's state in memory,
 * seeded by `__AIJ_CONNECTOR__` (by App id: part of a
 * {@link ConnectorStatusInfo}, plus `refused` for a preview that won't
 * add) and `__AIJ_DESKTOP_MEMBERS__` (by pool id), and records the calls
 * that change it in `__AIJ_CONNECTOR_CALLS__`. Without a seed a Claude
 * app profile has none yet and the Codex app can't take it.
 */
import { call } from "./transport";
import { hasLiveBackend, refused } from "../bridge";
import type { ConnectorFileInfo } from "../../types/generated/ConnectorFileInfo";
import type { ConnectorPreviewInfo } from "../../types/generated/ConnectorPreviewInfo";
import type { ConnectorRemovalInfo } from "../../types/generated/ConnectorRemovalInfo";
import type { ConnectorStatusInfo } from "../../types/generated/ConnectorStatusInfo";
import type { DesktopMemberInfo } from "../../types/generated/DesktopMemberInfo";

export type { ConnectorFileInfo, ConnectorPreviewInfo, ConnectorRemovalInfo, ConnectorStatusInfo, DesktopMemberInfo };

/** Said on the window when the connector was added to or taken out of an
 * App (`detail`: its id), so every view of it reads it again. */
export const CONNECTOR_CHANGED = "aij:connector-changed";

type Seed = Partial<ConnectorStatusInfo> & { refused?: string | null };
declare global {
  interface Window {
    __AIJ_CONNECTOR__?: Record<string, Seed>;
    __AIJ_DESKTOP_MEMBERS__?: Record<string, DesktopMemberInfo[]>;
    __AIJ_CONNECTOR_CALLS__?: string[];
  }
}
const seams = (): Window => window;
const record = (called: string) => (seams().__AIJ_CONNECTOR_CALLS__ ??= []).push(called);

/** The preview's Apps, as Add and Remove left them. */
const store = new Map<string, Seed>();

const CODEX_WHY = "Codex can't yet tell which thread is calling";
const AGENT = "~/AI Juggler/juggler-agent";
const SOCKET = "~/Library/Application Support/AI Juggler/agent/bridge.sock";

/** A Claude app profile's Claude Code folder in the preview. */
const folderOf = (appId: string) => `/Users/you/Profiles/${appId.split("/").pop() || "Work"}/claude-code`;

function seedOf(appId: string): Seed {
  const seeded = store.get(appId) ?? seams().__AIJ_CONNECTOR__?.[appId];
  if (seeded) return seeded;
  if (appId.includes("codex-app")) return { state: "unavailable", why_not: CODEX_WHY };
  if (appId.includes("claude-app")) return { state: "absent" };
  return { state: "unavailable", why_not: "AI Juggler has no connector for this app yet" };
}

function previewStatus(appId: string): ConnectorStatusInfo {
  const seed = seedOf(appId);
  const unavailable = seed.state === "unavailable";
  return {
    app_id: appId,
    state: seed.state ?? "absent",
    changed: seed.changed ?? [],
    why_not: seed.why_not ?? null,
    folder: unavailable ? null : (seed.folder ?? folderOf(appId)),
    picked_up: unavailable ? null : (seed.picked_up ?? "New Code-tab sessions of this profile get it when they start; sessions open now get it when they next start."),
    attests: seed.attests ?? !unavailable,
    can_wake: seed.can_wake ?? !unavailable,
  };
}

/** The plugin's manifest as the preview shows it: the bridge by its path,
 * its socket, the two inbox hooks; no secret. */
function manifest(): string {
  const hook = (event: string) => [{ hooks: [{ type: "command", command: `'${AGENT}' hook ${event} --connector`, timeout: 5 }] }];
  return `${JSON.stringify(
    {
      name: "ai-juggler",
      version: "0.4.6",
      description: "AI Juggler's pool connector: this profile's sessions read and send their pool's messages, shared docs and tasks while AI Juggler runs. Added by AI Juggler; remove it there.",
      mcpServers: { "ai-juggler": { command: AGENT, args: ["--mcp", "--plugin"], env: { JUGGLER_AGENT_SOCKET: SOCKET } } },
      hooks: { SessionStart: hook("SessionStart"), UserPromptSubmit: hook("UserPromptSubmit") },
    },
    null,
    2,
  )}\n`;
}

const SKILL = "---\nname: pool-tasks\ndescription: Work on your pool's tasks: claim one, say what you did, hand it on.\n---\n\nYour pool's tasks are in AI Juggler. Claim one before you start it…\n";

function previewFiles(status: ConnectorStatusInfo, seed: Seed): ConnectorFileInfo[] {
  const folder = status.folder ?? folderOf(status.app_id);
  const action = seed.refused ? "refuse" : status.state === "current" ? "same" : status.state === "outdated" ? "replace" : "create";
  return [
    { path: `${folder}/skills/ai-juggler/.claude-plugin/plugin.json`, action, before: action === "create" ? null : manifest(), after: manifest() },
    { path: `${folder}/skills/ai-juggler/skills/pool-tasks/SKILL.md`, action: action === "refuse" ? "create" : action, before: action === "create" || action === "refuse" ? null : SKILL, after: SKILL },
  ];
}

const GUIDE = {
  steps: [
    "In the Claude app, open a Code session in this profile.",
    "Click + below the prompt, then Add plugins.",
    "Add the marketplace figurehead-ai/AI-juggler, then install ai-juggler (for your user).",
    "Start a new Code session: AI Juggler sees it join the pool.",
  ],
  command: "/plugin marketplace add figurehead-ai/AI-juggler",
  open_url: null,
  needs: [],
  then: "The session's first call to AI Juggler's tools shows it here as connected.",
};

/** Tests: the preview starts over from its seeds. */
export function resetPreviewConnector(): void {
  store.clear();
}

/** Where the connector stands in the desktop App `appId`. */
export async function connectorStatus(appId: string): Promise<ConnectorStatusInfo> {
  if (!hasLiveBackend()) return previewStatus(appId);
  return call("connector_status", { appId });
}

/** What Add would write into `appId`, each file before and after. */
export async function connectorPreview(appId: string): Promise<ConnectorPreviewInfo> {
  if (!hasLiveBackend()) {
    const status = previewStatus(appId);
    if (status.state === "unavailable") return { status, files: [], refused: null, guide: null };
    const seed = seedOf(appId);
    const files = previewFiles(status, seed);
    const refusedWords = seed.refused ?? null;
    return { status, files, refused: refusedWords, guide: GUIDE };
  }
  return call("connector_preview", { appId });
}

/** Adds the connector to `appId` (the user's click, after the preview). */
export async function connectorAdd(appId: string): Promise<ConnectorStatusInfo> {
  if (!hasLiveBackend()) {
    const seed = seedOf(appId);
    if (seed.state === "unavailable") throw refused(seed.why_not ?? "it can't be added here");
    if (seed.refused) throw refused(seed.refused);
    record(`add:${appId}`);
    store.set(appId, { ...seed, state: "current", changed: [] });
    return previewStatus(appId);
  }
  return call("connector_add", { appId });
}

/** Takes the connector out of `appId`: what someone changed stays, named. */
export async function connectorRemove(appId: string): Promise<ConnectorRemovalInfo> {
  if (!hasLiveBackend()) {
    const before = previewStatus(appId);
    if (before.state === "absent" || before.state === "unavailable") throw refused("AI Juggler has no record of adding its connector to this app, so it takes nothing out");
    record(`remove:${appId}`);
    const files = previewFiles(before, {}).map((file) => file.path);
    const kept = files.filter((path) => before.changed.includes(path));
    store.set(appId, { ...seedOf(appId), state: kept.length > 0 ? "changed" : "absent", changed: kept });
    return { status: previewStatus(appId), removed: files.filter((path) => !kept.includes(path)), kept };
  }
  return call("connector_remove", { appId });
}

/** A pool's desktop members: their App's connector, connected, and how a
 * new pool message reaches each. */
export async function connectorDesktopMembers(poolId: string): Promise<DesktopMemberInfo[]> {
  if (!hasLiveBackend()) {
    // As seeded, each with its App's connector as Add and Remove left it.
    return (seams().__AIJ_DESKTOP_MEMBERS__?.[poolId] ?? []).map((one) => (one.app_id ? { ...one, connector: previewStatus(one.app_id).state } : one));
  }
  return call("connector_desktop_members", { poolId });
}
