/**
 * D-62 (AJ-E3201) — one name per App, built here and nowhere else.
 *
 * An App is one app in one of its profiles: "Claude app · b-work",
 * "Claude Code · Work Max", "Codex · ~/.codex", "ChatGPT".
 * - A desktop app is named by its app and its instance (the profile it
 *   runs in). A tool's own desktop app gets "app" after its name ("Claude
 *   app"), so it never reads as the tool ("Claude Code"). That comes from
 *   the tool's spec (`desktop_app`), never from a branch on a tool's id.
 * - A CLI App is its tool plus its folder. The terminal or editor it runs
 *   in is where its window is, a detail of the session, never its name.
 */
import { toolSpec } from "./tools";

/** "Claude app" for a tool's own desktop app (`Claude` for Claude Code),
 * else the app's name as the Dock shows it. */
export function desktopAppWord(app: string, tool: string | null): string {
  if (app === "") return app;
  return tool !== null && toolSpec(tool).desktop_app === app ? `${app} app` : app;
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
