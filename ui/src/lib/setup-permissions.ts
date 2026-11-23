/**
 * AJ-E1545 step 2 (with AJ-E1547's shape): the permissions the setup
 * wizard primes, one card each. The card's words are the OS-prompt
 * registry's entry shape — what the system will show, what to click, why,
 * what happens if declined, whether this setup needs it — so the step
 * moves onto that registry (`os_prompts_list`) by changing this one
 * adapter and nothing else.
 *
 * Until then the entries come from the OS access list the first run
 * already used (`os_access_list`: only what this platform and setup can
 * use, D-127), with the registry's extra words kept here, per id.
 */
import { osAccessList } from "./ipc/settings";
import type { OsAccessInfo } from "../types/generated/OsAccessInfo";
import { SYSTEM_NAME } from "./machine-words";

/** One permission, in the OS-prompt registry's shape. */
export interface PermissionEntry {
  id: string;
  title: string;
  why: string;
  /** What the system's own dialog says, in its words. */
  system_says: string;
  /** What to click in it. */
  click: string;
  /** What happens if the user says no. */
  if_declined: string;
  /** This setup uses it (only these are shown, D-127). */
  needed: boolean;
  /** `allowed` | `not_allowed` | `not_asked`. */
  status: string;
  /** A picture of the system's dialog, when the registry has one. */
  picture: string | null;
  /** The access list's next action and its note (Allow, System
   * Settings or Reopen; `lib/osAccess`). */
  source: OsAccessInfo;
}

/** The registry's words for the prompts the access list knows today. */
const WORDS: Record<string, Pick<PermissionEntry, "system_says" | "click" | "if_declined">> = {
  focus: {
    system_says: "“AI Juggler” would like to control this computer using accessibility features.",
    click: "Open System Settings, then turn on AI Juggler",
    if_declined: "Opening a session still brings its app forward; you pick the window yourself.",
  },
  preview: {
    system_says: "“AI Juggler” would like to record this computer's screen and audio.",
    click: "Open System Settings, then turn on AI Juggler",
    if_declined: "Sessions show without a live picture of their window.",
  },
};

const FALLBACK = {
  click: "Allow",
  if_declined: "AI Juggler works without it, and asks again only when you first use what it's for.",
};

/** An access-list entry as a registry entry. */
export function fromAccess(entry: OsAccessInfo): PermissionEntry {
  const words = WORDS[entry.id];
  return {
    id: entry.id,
    title: entry.title,
    why: entry.why,
    system_says: words?.system_says ?? `${SYSTEM_NAME} asks about ${entry.system_name}.`,
    click: words?.click ?? FALLBACK.click,
    if_declined: words?.if_declined ?? FALLBACK.if_declined,
    needed: true,
    status: entry.status,
    picture: null,
    source: entry,
  };
}

/** The permissions this setup needs, in the registry's shape. */
export async function permissionEntries(): Promise<PermissionEntry[]> {
  return (await osAccessList()).map(fromAccess).filter((entry) => entry.needed);
}

/** The step's value under its name: "1 of 2 allowed", "All allowed". */
export function permissionsValue(entries: readonly PermissionEntry[]): string {
  const allowed = entries.filter((entry) => entry.status === "allowed").length;
  if (entries.length > 0 && allowed === entries.length) return "All allowed";
  return `${allowed} of ${entries.length} allowed`;
}
