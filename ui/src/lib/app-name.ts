/**
 * D-62 (AJ-E3201) — one name per App, built here and nowhere else.
 *
 * An App is one app in one of its profiles: "Claude app · b-work",
 * "Claude Code · Work Max", "Codex · ~/.codex", "ChatGPT".
 * - A desktop app is named by its app and its instance (the profile it
 *   runs in). Its name on screen is the adapter's (`PlaceInfo.shown`:
 *   "Claude app", so it never reads as the tool Claude Code). Without one
 *   (an older reader, or a session AI Juggler started in the app), the
 *   same rule from the tool's spec: its desktop app gets "app" when the
 *   tool's own name starts with it. Never a branch on a tool's id.
 * - A CLI App is its tool plus its folder. The terminal or editor it runs
 *   in is where its window is, a detail of the session, never its name.
 */
import { toolSpec } from "./tools";
import { knownFolderTool } from "./palette";

/** A desktop app's name on screen: the adapter's (`shown`) when given;
 * else "Claude app" for a tool's own desktop app whose name the tool's
 * starts with (`Claude`, Claude Code), else the Dock's. */
export function desktopAppWord(app: string, tool: string | null, shown?: string | null): string {
  if (shown) return shown;
  if (app === "" || tool === null) return app;
  const spec = toolSpec(tool);
  return spec.desktop_app === app && spec.display_name.startsWith(app) ? `${app} app` : app;
}

/** AJ-E3203: a desktop App's profile in words. A profile is named by
 * the config folder its sessions use, as the user names it there
 * ("team-max", from its Code sessions), else by its own folder
 * ("b-team"). The app's default instance, and an app that runs one,
 * have none: "Claude app", "ChatGPT". */
export function desktopProfile(instance: string | null | undefined, folder: string | null | undefined): string | null {
  return instance ? folder || instance : null;
}

/** An App's name: its app, then its profile when it has one. */
export function appName(app: string, profile: string | null | undefined): string {
  return profile ? `${app} · ${profile}` : app;
}

/** A CLI App: tool `tool` on the folder labelled `folder`. */
export function cliAppName(tool: string, folder: string | null | undefined): string {
  return appName(toolSpec(tool).display_name, folder);
}

/** A CLI App's stored id: its folder's row (the backend's `cli_app_id`). */
export function cliAppId(folderId: string): string {
  return `cli:${folderId}`;
}

/** A CLI App's key: its tool and its folder's row. */
export function cliAppKey(tool: string, accountId: string | null, folder: string): string {
  return `cli|${tool}|${accountId ?? folder}`;
}

/** A desktop App's key: its app and its instance. */
export function desktopAppKey(appId: string | null, app: string, instance: string | null | undefined): string {
  return `app|${appId ?? app}|${instance ?? ""}`;
}

/** A desktop session AI Juggler started: keyed by its app and the folder
 * it runs on, the key a remembered profile on that folder is learnt by
 * too (lib/marks, lib/rail-apps), so it wears that App's colour and sits
 * in its group. Without a folder, by the folder's name. */
export function ownDesktopAppKey(appId: string | null, app: string, folderId: string | null, folderLabel: string): string {
  return folderId ? `app|${appId ?? app}|folder:${folderId}` : desktopAppKey(appId, app, folderLabel);
}

/** "in Terminal": where a CLI session's window is, as a row's detail. */
export function hostWords(host: string | null | undefined): string | null {
  return host ? `in ${host}` : null;
}

/** A folder's App by its full name ("Claude Code · Work Max"), from the
 * folders as last loaded; its label alone when its tool isn't known (the
 * 0.4 review: an App is never named by its folder alone). */
export function folderAppName(folderId: string | null | undefined, label: string): string {
  const tool = knownFolderTool(folderId);
  return tool ? cliAppName(tool, label) : label;
}
