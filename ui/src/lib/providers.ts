/**
 * Tool words for copy (the wire carries ids). AJ-E1525 (ADR-0019): every
 * word here comes from the tool's spec in the catalog (`lib/tools.ts`);
 * an id the catalog doesn't know renders as itself, never a guessed
 * product name.
 */
import { toolSpec } from "./tools";

export function providerDisplayName(id: string): string {
  return toolSpec(id).display_name;
}

/** The first mention of a tool's own sign-in on a surface (its spec says
 * whether to name the vendor: "Codex's own sign-in (OpenAI)"). */
export function ownSignIn(id: string): string {
  return toolSpec(id).sign_in_own;
}

/** A tool's sign-in in a list of tools ("Codex's (OpenAI)"). */
export function shortSignIn(id: string): string {
  return toolSpec(id).sign_in_short;
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

/** 0.0.11 walk (Q3): the tools that say they don't report their usage
 * limits. A tool the list doesn't name, or a failed call, keeps its limit
 * rows (as before). */
export function limitProviders(rows: { provider: string; reports_limits?: boolean }[]): InPlaceSupport {
  return new Set(rows.filter((row) => row.reports_limits === false).map((row) => row.provider));
}

export function reportsLimits(noLimits: InPlaceSupport, id: string): boolean {
  return noLimits === "unknown" || !noLimits.has(id);
}

/** What a tool without a limits feed says, once, on its account. */
export function noLimitsWords(id: string): string {
  return `${providerDisplayName(id)} doesn't report its limits to AI Juggler yet`;
}

export function supportsInPlace(support: InPlaceSupport, id: string): boolean {
  return support === "unknown" || support.has(id);
}

/** What "the same sign-in, settings and history" is the same AS, for a
 * set of tools: the tool's own words when there is one, else wherever
 * the user runs them now. */
export function sameAsWords(ids: string[]): string {
  const kinds = [...new Set(ids)];
  return kinds.length === 1 ? toolSpec(kinds[0]!).same_as : "as wherever you use them now";
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
