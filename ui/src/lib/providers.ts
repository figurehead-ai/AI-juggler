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
