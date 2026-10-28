/**
 * The switch dialog's words that are the UI's own (the backend says
 * what moves and whether keys matter): what a detected secret is, the
 * stages while the switch runs, and the badge. Never a detector id
 * ("sk-key"), never "pack" or "redaction" (D-25, the maintainer's walk
 * of 0.1.8's switch, 2026-10-27).
 */

/** A detected secret's kind, as people say it. */
const SECRET_KINDS: Record<string, string> = {
  "sk-key": "an API key",
  bearer: "a sign-in token",
  "env-value": "a secret in an environment value",
  "private-key": "a private key",
  "zcode-rollout": "a saved sign-in",
};

export function secretWords(kind: string): string {
  return SECRET_KINDS[kind] ?? "something that looks like a password";
}

/** "We found 2: an API key (s•••a91f) and a sign-in token (B•••x2)." */
export function foundWords(items: { kind: string; preview: string }[]): string {
  if (items.length === 0) return "We found no keys or passwords in this conversation.";
  const named = items.slice(0, 3).map((item) => `${secretWords(item.kind)} (${item.preview})`);
  const more = items.length > 3 ? `, and ${items.length - 3} more` : "";
  const list = named.length === 1 ? named[0] : `${named.slice(0, -1).join(", ")} and ${named[named.length - 1]}`;
  return `We found ${items.length}: ${list}${more}.`;
}

/** What runs, in order, while the switch works (plain names). */
export function stageWords(target: string): string[] {
  return ["Pausing the original", "Copying the conversation", "Checking the copy", `Starting on ${target}`, "Saving a record of the switch"];
}

/** The badge, the same everywhere in the dialog: what the move does, and
 * how sure that is. */
export function badgeWords(fidelityWords: string, validated: boolean | undefined): string {
  return `${fidelityWords} · ${validated ? "Proven" : "Beta · Not proven yet"}`;
}

/** The user guide's switching page, for "Learn more" (opened by the app,
 * never as a URL the page holds). */
export const GUIDE = {
  keys: "switching#keys-and-passwords-in-this-conversation",
  original: "switching#keep-or-stop-the-original",
  setUp: "switching#things-the-other-account-doesnt-have",
  carried: "switching#what-goes-along",
  room: "switching#an-account-with-little-or-no-room",
} as const;
