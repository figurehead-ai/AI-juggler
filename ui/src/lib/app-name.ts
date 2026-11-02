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

/** A desktop app's name on screen: the adapter's (`shown`) when given;
 * else "Claude app" for a tool's own desktop app whose name the tool's
 * starts with (`Claude`, Claude Code), else the Dock's. */
export function desktopAppWord(app: string, tool: string | null, shown?: string | null): string {
  if (shown) return shown;
  if (app === "" || tool === null) return app;
  const spec = toolSpec(tool);
  return spec.desktop_app === app && spec.display_name.startsWith(app) ? `${app} app` : app;
}

/** An App's name: its app, then its profile when it has one. */
export function appName(app: string, profile: string | null | undefined): string {
  return profile ? `${app} · ${profile}` : app;
}

/** A CLI App: tool `tool` on the folder labelled `folder`. */
export function cliAppName(tool: string, folder: string | null | undefined): string {
  return appName(toolSpec(tool).display_name, folder);
}

/** A CLI App's key: its tool and its folder's row. */
export function cliAppKey(tool: string, accountId: string | null, folder: string): string {
  return `cli|${tool}|${accountId ?? folder}`;
}

/** A desktop App's key: its app and its instance. */
export function desktopAppKey(appId: string | null, app: string, instance: string | null | undefined): string {
  return `app|${appId ?? app}|${instance ?? ""}`;
}

/** "in Terminal": where a CLI session's window is, as a row's detail. */
export function hostWords(host: string | null | undefined): string | null {
  return host ? `in ${host}` : null;
}
