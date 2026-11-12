/**
 * ADR-0032 Amendment 3 (D-100) — an App's kind, Desktop or CLI, in the
 * maintainer's words ("Desktop", "CLI"; never "terminal"), from what an
 * App is keyed by:
 * - the rail's key: `app|…` a desktop App, `cli|…` a CLI App;
 * - the backend's id: `app:…` a desktop App, `cli:…` a CLI App;
 * - a place's or an App's `kind`: "desktop", else a CLI App.
 * AI Juggler's own place, and anything else, has none.
 */
export type AppKind = "desktop" | "cli";

/** An App's kind by its rail key or its backend id; `null` for anything
 * else (AI Juggler's own place, a pool, an Account). */
export function appKindOfKey(key: string | null | undefined): AppKind | null {
  if (!key) return null;
  if (key.startsWith("cli|") || key.startsWith("cli:")) return "cli";
  if (key.startsWith("app|") || key.startsWith("app:")) return "desktop";
  return null;
}

/** Its word on screen ("Desktop", "CLI") and its name for assistive
 * technology and the tooltip ("Desktop app", "CLI app"). */
export function appKindWords(kind: AppKind): { word: string; name: string } {
  return kind === "cli" ? { word: "CLI", name: "CLI app" } : { word: "Desktop", name: "Desktop app" };
}
