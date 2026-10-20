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
