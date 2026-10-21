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
 * ADR-0018: which tools' setups AI Juggler can use where they are, as
 * the backend reports them (`providers_capabilities` — the adapter adopts
 * a folder without writing to it). `"unknown"` when that call failed:
 * the choice then stays for every tool, in place first — the user
 * always decides, and the backend refuses anything it can't adopt.
 */
export type InPlaceSupport = ReadonlySet<string> | "unknown";

export function inPlaceProviders(rows: { provider: string; can_use_in_place: boolean }[]): InPlaceSupport {
  return new Set(rows.filter((row) => row.can_use_in_place).map((row) => row.provider));
}

export function supportsInPlace(support: InPlaceSupport, id: string): boolean {
  return support === "unknown" || support.has(id);
}

/** What "the same sign-in, settings and history" is the same AS, for a
 * set of tools: the Claude app, Codex run by hand, or both. */
export function sameAsWords(ids: string[]): string {
  const kinds = new Set(ids);
  if (kinds.size === 1 && kinds.has("codex")) return "as when you run Codex yourself";
  if (kinds.size === 1 && kinds.has("claude-code")) return "as your Claude app";
  return "as wherever you use them now";
}

/** A tool's install row, as far as these helpers read it. */
type InstallRow = { status: string; binary?: string | null };

/**
 * The 0.0.11 report: a tool AI Juggler found whose version check failed
 * (an npm-installed CLI whose `node` it couldn't run) is found, not
 * missing — it is offered like any installed tool.
 */
export function toolFound(row: InstallRow): boolean {
  return row.status === "installed" || row.status === "unverified" || (row.status === "probe_failed" && !!row.binary);
}

/** Found, but its version couldn't be read. */
export function versionUnchecked(row: InstallRow): boolean {
  return row.status === "probe_failed" && !!row.binary;
}

/** The words for a found tool whose version couldn't be read. */
export const FOUND_UNCHECKED = "Found; couldn't check its version";
