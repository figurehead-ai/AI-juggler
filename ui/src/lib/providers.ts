/**
 * Provider display names for copy (the wire carries ids). Unknown ids
 * render as themselves — never a guessed product name.
 */
const NAMES: Record<string, string> = {
  "claude-code": "Claude Code",
  codex: "Codex",
  zcode: "ZCode",
  gemini: "Gemini CLI",
};

export function providerDisplayName(id: string): string {
  return NAMES[id] ?? id;
}

/**
 * The first mention of a Provider's own sign-in on a surface. Codex names
 * its vendor once ("Codex's own sign-in (OpenAI)"); later mentions on the
 * same surface say just "Codex's".
 */
export function ownSignIn(id: string): string {
  return id === "codex" ? "Codex's own sign-in (OpenAI)" : `${providerDisplayName(id)}'s own sign-in`;
}

/**
 * ADR-0018: the tools whose setups AI Juggler can use where they are
 * (their adapter adopts a folder without writing to it). Any other
 * tool's setups come in as copies.
 */
const IN_PLACE = new Set(["claude-code", "codex"]);

export function canUseInPlace(id: string): boolean {
  return IN_PLACE.has(id);
}

/** What "the same sign-in, settings and history" is the same AS, for a
 * set of tools: the Claude app, Codex run by hand, or both. */
export function sameAsWords(ids: string[]): string {
  const kinds = new Set(ids);
  if (kinds.size === 1 && kinds.has("codex")) return "as when you run Codex yourself";
  if (kinds.size === 1 && kinds.has("claude-code")) return "as your Claude app";
  return "as wherever you use them now";
}
