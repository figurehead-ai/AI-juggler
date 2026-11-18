/**
 * AJ-E3013 (D-116, D-119): AI Juggler's connector, in words and in the
 * reads every view of it shares — the consent card's, the canvas's, the
 * inspector's, New pool's and the switch's (one wording, one read).
 *
 * - The consent card: what Add does, its consequence, an example and the
 *   guide's section; each file of the preview in a line.
 * - A desktop member: connected ("wakes on new messages" or "reads
 *   messages on its next turn", D-119), added and waiting for its next
 *   start, or Watched with why (its app has no connector, or can't take
 *   one). A Codex app thread can't be woken; it says so.
 * - Reads: an App's connector (again when Add or Remove changes it, and
 *   every few seconds only while its guide is open), a pool's desktop
 *   members (again when the pool or a connector changes; never polled).
 */
import { useCallback, useEffect, useMemo, useState } from "react";

import { sharedReads, useSharedRead } from "./shared-read";
import { POOLS_GUIDE } from "./pool-words";
import { CONNECTOR_CHANGED, connectorDesktopMembers, connectorDesktopSessions, connectorStatus, type ConnectorStatusInfo, type DesktopMemberInfo, type DesktopSessionInfo } from "./ipc/connector";
import { usePoolEvents } from "./pool-live";
import type { AppModel } from "./apps";
import type { AgentBridgeInfo } from "./ipc/agent-bridge";

/** The guide's section on desktop apps in a pool. */
export const CONNECTOR_GUIDE = `${POOLS_GUIDE}#desktop-apps-in-a-pool`;

/** How often an open guide reads the App again (only while it's open). */
export const GUIDE_POLL_MS = 3_000;

/** What the connector is, said where it's first named (Casey's PC1 walk:
 * "connector" wasn't explained where it appeared). */
export const CONNECTOR_MEANING = "a small helper AI Juggler adds to an app so its sessions can read and send the pool's messages, notes and tasks";

/** The preview's first line: what Add does, in plain words, before any
 * path ("This adds a small helper to your Claude app (Work Max) so its
 * sessions can talk in this pool: 2 new files in its own settings folder;
 * nothing of yours changes."). */
export function previewSentence(files: readonly { action: string }[], app: { name: string; kind?: string; profile?: string | null }, where = "this pool"): string {
  const count = (action: string) => files.filter((file) => file.action === action).length;
  const plural = (n: number, one: string, many: string) => (n === 1 ? `1 ${one}` : `${n} ${many}`);
  const parts = [
    count("create") > 0 ? `${plural(count("create"), "new file", "new files")} in its own settings folder` : null,
    count("replace") > 0 ? `${plural(count("replace"), "older AI Juggler file", "older AI Juggler files")} updated` : null,
    count("merge") > 0 ? `its own table added to ${plural(count("merge"), "settings file", "settings files")}` : null,
  ].filter(Boolean);
  const who = app.kind && app.profile ? `your ${app.kind} (${app.profile})` : app.name;
  return `This adds a small helper to ${who} so its sessions can talk in ${where}${parts.length > 0 ? `: ${parts.join(", ")}` : ""}; nothing of yours changes.`;
}

/** The consent card's words. */
export const CONNECTOR = {
  title: (app: string, where = "this pool") => `Add AI Juggler's connector to ${app} so its sessions can talk in ${where}`,
  consequence: "AI Juggler writes its own folder into this app's settings: its pool tools and a note on new messages. Nothing of yours changes; you see every file first, and Remove takes out exactly what it wrote.",
  example: "For example: a Code session in Claude · Work reads that Codex · Personal finished its review, and claims the next task.",
  docs: "How desktop apps join a pool",
  add: "Add",
  confirm: (app: string) => `Add to ${app}`,
  cancel: "Cancel",
  remove: "Remove",
  confirmRemove: (app: string) => `Remove from ${app}`,
  added: "Added. A running session picks it up at its next start.",
  update: "Update",
  outdated: "An older edition of AI Juggler's connector is here: Update writes this edition's.",
  showGuide: "Show me how to add it by hand",
  meaning: `AI Juggler's connector is ${CONNECTOR_MEANING}.`,
  showFiles: "Show the files",
  // D-126: New pool's one-line notice under a chosen App's card.
  needs: (app: string) => `${app} needs AI Juggler's connector to message the team.`,
  older: (app: string) => `${app} has an older edition of AI Juggler's connector.`,
  seeFirst: "You see the exact change before anything is written.",
  previewAndAdd: "Preview and add",
  previewAndUpdate: "Preview and update",
};

/** Home as `~`, so a path reads short (and names no one). */
export function shortPath(path: string): string {
  return path.replace(/^\/Users\/[^/]+\//, "~/").replace(/^\/home\/[^/]+\//, "~/");
}

/** One file of the preview, in a line. */
export function fileLine(action: string, path: string): string {
  const where = shortPath(path);
  switch (action) {
    case "create":
      return `Creates ${where}`;
    case "replace":
      return `Updates ${where} (an older edition of AI Juggler's)`;
    case "merge":
      return `Adds AI Juggler's own table to ${where}; the rest stays as it is`;
    case "same":
      return `${where} is already as AI Juggler writes it`;
    default:
      return `Won't replace ${where}: something AI Juggler didn't write is there`;
  }
}

/** Changed files by name ("SKILL.md", "plugin.json and SKILL.md"). */
export function changedWords(paths: readonly string[]): string {
  const names = paths.map((path) => path.split("/").pop() ?? path);
  return names.length <= 1 ? (names[0] ?? "a file") : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
}

/** What Remove deletes, in a line per file. */
export const removeLine = (path: string) => `Deletes ${shortPath(path)}`;

/** A file someone changed since Add (Remove is all or nothing: it takes
 * nothing out then; Remove anyway deletes it too). */
export const keptLine = (path: string) => `${shortPath(path)} changed since AI Juggler wrote it`;

/** Why the connector can't go into an App, in the user's words, never
 * the backend's own text (round 6, Casey's run: "juggler-agent isn't
 * beside it, or isn't safe to run"). A reason code (`unavailable`, when
 * the status carries one) decides; until then the agent program's own
 * (the team tools' `AgentBridgeInfo`: the same program) does. */
export function unavailableWords(status: ConnectorStatusInfo, bridge: AgentBridgeInfo | null): string {
  // The status's own code (41af467d0); without one, the agent program's.
  const program = bridge && !bridge.available ? (bridge.unavailable ?? "missing") : null;
  const kind = status.unavailable ?? program;
  const where = status.unsafe_folder ?? bridge?.unsafe_folder ?? null;
  switch (kind) {
    case "missing":
      return "This build of AI Juggler doesn't include its helper.";
    case "unsafe_location":
      return `AI Juggler's helper can't run from a folder others can write${where ? ` (${where})` : ""}. Move AI Juggler to Applications, then open it again.`;
    case "not_offered":
      return "AI Juggler can't add its connector to this app yet: it can't tell this app's sessions apart.";
    case "unknown_app":
      return "AI Juggler doesn't know this app yet: open it once while AI Juggler runs.";
    case "no_folder":
      return "AI Juggler hasn't seen which settings folder this app uses yet: open a session in it once.";
    case "no_connector":
      return "AI Juggler has no connector for this kind of app yet.";
    default:
      return "AI Juggler can't add its connector to this app yet.";
  }
}

/** Where the connector stands, when it isn't simply there. */
export function statusLine(status: ConnectorStatusInfo, bridge: AgentBridgeInfo | null = null): string | null {
  switch (status.state) {
    case "current":
      return "AI Juggler's connector is in this app.";
    case "outdated":
      return CONNECTOR.outdated;
    case "changed":
      return `AI Juggler's connector here was changed${status.changed.length > 0 ? ` (${status.changed.map(shortPath).join(", ")})` : ""}, so AI Juggler leaves it alone. Remove shows what it would delete, and Remove anyway takes it out whole; then Add writes it again.`;
    case "unavailable":
      return unavailableWords(status, bridge);
    default:
      return null;
  }
}

/** Whether the connector is there for the App's sessions to use. */
export const isAdded = (state: string | undefined) => state === "current" || state === "outdated";

/** How a new pool message reaches a desktop member (D-119). */
export function wakeLine(wake: string): string {
  switch (wake) {
    case "wakes":
      return "Wakes on new messages";
    case "cant_wake":
      return "Can't be woken: it reads messages when you next use it";
    default:
      return "Reads messages on its next turn";
  }
}

/** D-119 (5): a message the person sends with Deliver wakes a desktop
 * member whatever the pool's level (the waking rule gates agents'
 * messages); Messages' Deliver says the same. */
export function deliverWakes(wake: string, many: boolean): string {
  return wake === "next_turn" ? ` A message you send with Deliver wakes ${many ? "them" : "it"}, whatever the pool's level.` : "";
}

/** A desktop member as the canvas and inspector say it. */
export type DesktopReach =
  /** Its sessions call through the connector. */
  | { kind: "connected"; line: string }
  /** The connector is there; its sessions join at their next start. */
  | { kind: "added"; line: string }
  /** No connector it can use: Watched, with why, and whether Add fixes it. */
  | { kind: "watched"; line: string; fix: "add" | "fix" | null };

export function reachOf(info: DesktopMemberInfo | undefined): DesktopReach {
  // The connector first (the aij-test proof: after Remove, a session that
  // had called lately still read "Wakes on new messages"): without one in
  // its app it's Watched, with the reason, whatever called before.
  if (!info) return { kind: "watched", line: "Its app has no AI Juggler connector", fix: "add" };
  if (info.connector === "changed") return { kind: "watched", line: "Its app's AI Juggler connector was changed", fix: "fix" };
  if (info.connector === "unavailable" || info.app_id === null) {
    const wake = info.wake === "cant_wake" ? "; it can't be woken" : "";
    return { kind: "watched", line: `Its app can't take AI Juggler's connector yet${wake}`, fix: null };
  }
  if (!isAdded(info.connector)) return { kind: "watched", line: "Its app has no AI Juggler connector", fix: "add" };
  if (info.connected) return { kind: "connected", line: wakeLine(info.wake) };
  return { kind: "added", line: `Added · joins at its next start · ${wakeLine(info.wake).toLowerCase()}` };
}

const statuses = sharedReads(connectorStatus);
const members = sharedReads(connectorDesktopMembers);
const sessions = sharedReads(connectorDesktopSessions);

/** A connector's state, read again on Add and Remove. */
function useReadAgain(refresh: () => void): void {
  useEffect(() => {
    window.addEventListener(CONNECTOR_CHANGED, refresh);
    return () => window.removeEventListener(CONNECTOR_CHANGED, refresh);
  }, [refresh]);
}

/** Where the connector stands in `appId` (`undefined` until read); read
 * every {@link GUIDE_POLL_MS} only while `polling` (its guide is open). */
export function useConnectorStatus(appId: string | null, polling = false): ConnectorStatusInfo | undefined {
  const source = appId ? statuses(appId) : null;
  const value = useSharedRead(source, { pollMs: polling ? GUIDE_POLL_MS : 0 });
  useReadAgain(useCallback(() => void source?.refresh(), [source]));
  return value;
}

/** A pool's desktop members, by member ref (`undefined` until read);
 * read every {@link GUIDE_POLL_MS} only while `polling` (a guide is open:
 * its first call shows the member connected). */
export function useDesktopMembers(poolId: string | undefined, polling = false): ReadonlyMap<string, DesktopMemberInfo> | undefined {
  const source = poolId ? members(poolId) : null;
  const value = useSharedRead(source, { pollMs: polling ? GUIDE_POLL_MS : 0 });
  usePoolEvents(poolId, useCallback(() => void source?.refresh(), [source]));
  return useMemo(() => (value ? new Map(value.map((one) => [one.member, one])) : undefined), [value]);
}

/** A pool's desktop sessions (members through the connector), read on
 * open and again when the pool or a connector changes; never polled. */
export function useDesktopSessions(poolId: string | undefined): readonly DesktopSessionInfo[] {
  const source = poolId ? sessions(poolId) : null;
  const value = useSharedRead(source);
  usePoolEvents(poolId, useCallback(() => void source?.refresh(), [source]));
  return value ?? NONE;
}

const NONE: readonly DesktopSessionInfo[] = [];

/** AJ-E3013: a pool's desktop sessions as Messages recipients, beside the
 * sessions AI Juggler runs: each by its member ref (what `to` takes),
 * named with its place; open while one of its calls is fresh. */
export function desktopRecipients(found: readonly DesktopSessionInfo[]): { session_id: string; name: string; live: boolean }[] {
  return found.map((one) => ({ session_id: one.member, name: one.place ? `${one.name} · ${one.place}` : one.name, live: one.session_id !== null }));
}

/** Several Apps' connectors at once (a step listing them), read on mount
 * and again on every Add and Remove; `undefined` for one not read yet. */
export function useConnectorStatuses(appIds: readonly string[]): ReadonlyMap<string, ConnectorStatusInfo> {
  const key = appIds.join("\n");
  const [found, setFound] = useState<ReadonlyMap<string, ConnectorStatusInfo>>(new Map());
  useEffect(() => {
    let alive = true;
    const read = () =>
      void Promise.all(key ? key.split("\n").map((id) => connectorStatus(id).then((status) => [id, status] as const, () => null)) : []).then((rows) => {
        if (alive) setFound(new Map(rows.filter((row): row is readonly [string, ConnectorStatusInfo] => row !== null)));
      });
    read();
    window.addEventListener(CONNECTOR_CHANGED, read);
    return () => {
      alive = false;
      window.removeEventListener(CONNECTOR_CHANGED, read);
    };
  }, [key]);
  return found;
}

/** Tells every view the connector changed in `appId`. */
export function connectorChanged(appId: string): void {
  window.dispatchEvent(new CustomEvent(CONNECTOR_CHANGED, { detail: appId }));
}

/** A desktop App's config folder's id, when known. */
export const folderOfApp = (app: AppModel): string | null => app.folder?.id ?? app.place?.account_id ?? null;

/** The desktop Apps behind some member refs (an App, or a session in a
 * desktop App's folder), once each. */
export function desktopAppsOf(refs: readonly string[], apps: readonly AppModel[]): AppModel[] {
  const desktop = apps.filter((app) => app.kind === "desktop");
  const out: AppModel[] = [];
  for (const ref of refs) {
    const folder = ref.startsWith("conversation:") ? ref.slice("conversation:".length).split("/")[0] : null;
    const app = desktop.find((one) => ref === `app:${one.id}` || (folder !== null && folderOfApp(one) === folder));
    if (app && !out.includes(app)) out.push(app);
  }
  return out;
}
