/**
 * 0.3.5 — where a switch can continue, for the preview, the tests and the
 * gallery (set one as `destinations` in the seam `__AIJ_SWITCH_PREFLIGHT__`).
 * Types only here, so a gallery spec can import it without the app's
 * transport.
 */
import type { DestinationsInfo } from "../../types/generated/DestinationsInfo";

/** The target has a Claude app profile, open now; it ran in a terminal. */
export const EXAMPLE_DESTINATIONS: DestinationsInfo = {
  suggested: "cli",
  desktop: { place_id: "place-claude-personal", name: "Claude app · Personal", app_id: "claude-app", running: true },
  desktop_why_not: null,
  cli_why_not: null,
};

/** The same, from the Claude app: Desktop is where it was, and the app
 * isn't open on the target (AI Juggler starts it). */
export const EXAMPLE_DESTINATIONS_DESKTOP: DestinationsInfo = {
  suggested: "desktop",
  desktop: { place_id: "place-claude-personal", name: "Claude app · Personal", app_id: "claude-app", running: false },
  desktop_why_not: null,
  cli_why_not: null,
};

/** 0.4: the target is a Claude app profile used in its Code tab, signed
 * in through the app and not in a terminal: CLI says why not (the confirm
 * step picks Desktop). It ran in the Claude app here. */
export const EXAMPLE_DESTINATIONS_APP_SIGN_IN: DestinationsInfo = {
  suggested: "desktop",
  desktop: { place_id: "place-claude-personal", name: "Claude app · Personal", app_id: "claude-app", running: true },
  desktop_why_not: null,
  cli_why_not: "Personal signs in through Claude app · Personal, and the tool in a terminal here isn't signed in: continue in Claude app · Personal, where it is.",
};

/** The target has no Claude app profile: Desktop is shown, with why not. */
export const EXAMPLE_DESTINATIONS_NO_DESKTOP: DestinationsInfo = {
  suggested: "cli",
  desktop: null,
  desktop_why_not: "No desktop app profile on Personal yet. Open a session on it in the app once, and Desktop is offered here.",
  cli_why_not: null,
};

/** The preview's answer: a conversation another app runs (its key) was in
 * the Claude app; Personal (signed out in the preview) and Codex have no
 * Claude app profile. */
export function previewDestinations(sessionId: string, targetProfileId: string): DestinationsInfo {
  if (targetProfileId === "recipe-codex") {
    return { suggested: "cli", desktop: null, desktop_why_not: "ChatGPT can't pick up a switched session yet.", cli_why_not: null };
  }
  if (targetProfileId === "recipe-personal") return EXAMPLE_DESTINATIONS_NO_DESKTOP;
  const desktop = { ...EXAMPLE_DESTINATIONS.desktop!, name: targetProfileId === "recipe-work-max" ? "Claude app · Work Max" : EXAMPLE_DESTINATIONS.desktop!.name };
  return { ...EXAMPLE_DESTINATIONS, desktop, suggested: sessionId.startsWith("conversation:") ? "desktop" : "cli" };
}
