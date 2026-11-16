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
import { useCallback, useEffect, useMemo } from "react";

import { sharedReads, useSharedRead } from "./shared-read";
import { POOLS_GUIDE } from "./pool-words";
import { CONNECTOR_CHANGED, connectorDesktopMembers, connectorStatus, type ConnectorStatusInfo, type DesktopMemberInfo } from "./ipc/connector";
import { POOL_RULES_CHANGED } from "./ipc/pool-model";
import type { AppModel } from "./apps";

/** The guide's section on desktop apps in a pool. */
export const CONNECTOR_GUIDE = `${POOLS_GUIDE}#desktop-apps-in-a-pool`;

/** How often an open guide reads the App again (only while it's open). */
export const GUIDE_POLL_MS = 3_000;

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
  showGuide: "Add it yourself instead",
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

/** What Remove deletes, in a line per file. */
export const removeLine = (path: string) => `Deletes ${shortPath(path)}`;

/** A file someone changed since Add: Remove leaves it, named. */
export const keptLine = (path: string) => `Leaves ${shortPath(path)}: it changed since AI Juggler wrote it`;

/** Where the connector stands, when it isn't simply there. */
export function statusLine(status: ConnectorStatusInfo): string | null {
  switch (status.state) {
    case "current":
      return "AI Juggler's connector is in this app.";
    case "outdated":
      return CONNECTOR.outdated;
    case "changed":
      return `AI Juggler's connector here was changed${status.changed.length > 0 ? ` (${status.changed.map(shortPath).join(", ")})` : ""}, so AI Juggler leaves it alone. Remove takes out the rest; then Add writes it again.`;
    case "unavailable":
      return `AI Juggler can't add its connector here: ${status.why_not ?? "this app has no way in yet"}.`;
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

/** A desktop member as the canvas and inspector say it. */
export type DesktopReach =
  /** Its sessions call through the connector. */
  | { kind: "connected"; line: string }
  /** The connector is there; its sessions join at their next start. */
  | { kind: "added"; line: string }
  /** No connector it can use: Watched, with why, and whether Add fixes it. */
  | { kind: "watched"; line: string; fix: "add" | "fix" | null };

export function reachOf(info: DesktopMemberInfo | undefined): DesktopReach {
  if (!info) return { kind: "watched", line: "Its app has no AI Juggler connector", fix: "add" };
  if (info.connected) return { kind: "connected", line: wakeLine(info.wake) };
  if (isAdded(info.connector)) return { kind: "added", line: `Added · joins at its next start · ${wakeLine(info.wake).toLowerCase()}` };
  if (info.connector === "changed") return { kind: "watched", line: "Its app's AI Juggler connector was changed", fix: "fix" };
  if (info.connector === "unavailable" || info.app_id === null) {
    const wake = info.wake === "cant_wake" ? "; it can't be woken" : "";
    return { kind: "watched", line: `Its app can't take AI Juggler's connector yet${wake}`, fix: null };
  }
  return { kind: "watched", line: "Its app has no AI Juggler connector", fix: "add" };
}

const statuses = sharedReads(connectorStatus);
const members = sharedReads(connectorDesktopMembers);

/** Read again on Add, Remove, and a pool's change. */
function useReadAgain(refresh: () => void, poolId?: string): void {
  useEffect(() => {
    const again = (event: Event) => {
      if (!(event instanceof CustomEvent) || !poolId || event.type === CONNECTOR_CHANGED || event.detail === poolId) refresh();
    };
    window.addEventListener(CONNECTOR_CHANGED, again);
    if (poolId) window.addEventListener(POOL_RULES_CHANGED, again);
    return () => {
      window.removeEventListener(CONNECTOR_CHANGED, again);
      window.removeEventListener(POOL_RULES_CHANGED, again);
    };
  }, [refresh, poolId]);
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
  useReadAgain(useCallback(() => void source?.refresh(), [source]), poolId);
  return useMemo(() => (value ? new Map(value.map((one) => [one.member, one])) : undefined), [value]);
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
