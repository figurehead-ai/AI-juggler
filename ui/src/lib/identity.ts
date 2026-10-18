/**
 * AJ-E1001 — an Instance's identity assertion in words (the rail's
 * compact marker and the inspector's chip share these, so the meaning
 * never depends on colour).
 */
import type { InstanceInfo } from "../types/generated/InstanceInfo";

export type IdentityWords = {
  /** The compact glyph (decoration — the words carry the meaning). */
  glyph: string;
  /** The full words. */
  text: string;
  /** `verified` | `first_seen` | `signed_out` | `unverified`. */
  state: string;
};

export function identityWords(session: Pick<InstanceInfo, "identity_state" | "identity_label">): IdentityWords | null {
  const label = session.identity_label ?? null;
  switch (session.identity_state) {
    case "verified":
      return { glyph: "✓", text: `Signed in as ${label ?? "the expected account"} — verified`, state: "verified" };
    case "first_seen":
      return {
        glyph: "✓",
        text: `Signed in as ${label ?? "an account"} — first seen; later launches must match`,
        state: "first_seen",
      };
    case "signed_out":
      return { glyph: "○", text: "Not signed in yet — the identity is checked once it is", state: "signed_out" };
    case "unverified":
      return { glyph: "?", text: "Identity unverified — this Provider exposes no account metadata", state: "unverified" };
    default:
      return null;
  }
}
