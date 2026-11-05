/**
 * AJ-E2906: how sure first run is about each setup it found, in words
 * (wf16, "inference is never silent"). A row says one word — Found,
 * Likely or Guessed — for what it claims (its name and its sign-in, the
 * weaker of the two) and one line of where the setup came from; Details
 * lists each field with its own word and source, and the scan's notes.
 * The backend sends which rule decided each field and where the folder
 * came from as closed sets; this is the one place they become words
 * (never an internal id on screen, never a score).
 */
import type { DiscoveredInfo } from "../lib/bridge";

export type Confidence = "found" | "likely" | "guessed";

const WORD: Record<Confidence, string> = { found: "Found", likely: "Likely", guessed: "Guessed" };
const RANK: Record<Confidence, number> = { found: 0, likely: 1, guessed: 2 };

/** An unknown value never claims more than a guess. */
function level(raw: string | undefined): Confidence {
  return raw === "found" || raw === "likely" ? raw : "guessed";
}

export const confidenceWord = (value: Confidence): string => WORD[value];

/** The row's word: the weaker of its name and its sign-in. */
export function rowConfidence(found: DiscoveredInfo): Confidence {
  const name = level(found.inference?.purpose_confidence);
  const signIn = level(found.inference?.login_confidence);
  return RANK[name] >= RANK[signIn] ? name : signIn;
}

/** Where the setup came from, in one line. */
export function sourceWords(found: DiscoveredInfo, toolName: string): string {
  switch (found.origin) {
    case "vendor-default":
      return `${toolName}'s usual folder`;
    case "env-override":
    case "shell-alias":
      return "set in your shell's settings";
    case "home-pattern":
      return `a folder named like a ${toolName} setup`;
    case "other-manager":
      return found.set_by ? `named by ${found.set_by}` : "named by another app";
    default:
      return "found on this computer";
  }
}

const NAME_SOURCE: Record<string, string> = {
  "usual-folder": "the tool's usual folder",
  "other-app": "the other app's name for it",
  "folder-name": "the folder's name",
};
const SIGN_IN_VALUE: Record<string, string> = {
  "signed-in": "Signed in",
  "maybe-expired": "May have expired",
  "not-signed-in": "Not signed in",
};
const SIGN_IN_SOURCE: Record<string, string> = {
  "sign-in-present": "its sign-in is in its folder",
  "system-keychain": "its sign-in is kept by the system",
  "sign-in-empty": "its sign-in file is there but empty",
  "no-sign-in": "no sign-in in its folder",
};
const USED_SOURCE: Record<string, string> = {
  "newest-conversation": "its newest conversation",
  "no-conversations": "it has no conversations yet",
};

export interface FieldRow {
  field: string;
  value: string;
  confidence: Confidence;
  source: string;
}

/** Details: each field the row shows, its word and where it came from. */
export function fieldRows(found: DiscoveredInfo, shown: { name: string; used: string }): FieldRow[] {
  const inference = found.inference ?? ({} as Partial<DiscoveredInfo["inference"]>);
  return [
    {
      field: "Name",
      value: shown.name,
      confidence: level(inference.purpose_confidence),
      // A word in the name: say what it reads as ("Personal").
      source: inference.purpose_rule === "name-word" ? `the folder's name, which reads as ${inference.purpose}` : (NAME_SOURCE[inference.purpose_rule ?? ""] ?? "the folder's name"),
    },
    {
      field: "Sign-in",
      value: SIGN_IN_VALUE[inference.login ?? ""] ?? "Not known",
      confidence: level(inference.login_confidence),
      source: SIGN_IN_SOURCE[inference.login_rule ?? ""] ?? "its folder",
    },
    { field: "Last used", value: shown.used, confidence: level(inference.activity_confidence), source: USED_SOURCE[inference.activity_rule ?? ""] ?? "its folder" },
  ];
}

/** A sign-in that may have expired (an empty sign-in file). */
export const mayHaveExpired = (found: DiscoveredInfo): boolean => found.inference?.login_rule === "sign-in-empty";
