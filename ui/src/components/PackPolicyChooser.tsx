/**
 * AJ-E309 Req.1 (D-14): the pack secret-handling choice as a consent
 * surface. The words are AJ-E208's — the same options the switch
 * chooser and Accounts → Pack secrets offer — and each states its
 * consequence in one sentence (Nielsen H3). NOTHING is pre-selected:
 * `value === null` renders every radio unchecked, and "Decide later"
 * is a real, un-shamed option that writes nothing.
 */

/** The stored policy words (AJ-E208 wire). */
export type PackPolicyWire = "strip" | "keep" | "ask";

/** A first-run answer: a stored policy, or an explicit "later". */
export type PackPolicyChoice = PackPolicyWire | "decide_later";

export const PACK_POLICY_OPTIONS: {
  value: PackPolicyChoice;
  title: string;
  consequence: string;
}[] = [
  {
    value: "strip",
    title: "Strip — replace keys with •••",
    consequence:
      "Keys and tokens found in a pack are replaced with ••• before it's built — the safest choice.",
  },
  {
    value: "keep",
    title: "Keep — pack stays on this Mac",
    consequence:
      "Packs keep secrets as they are. They never leave this Mac, but the account you switch to can read them.",
  },
  {
    value: "ask",
    title: "Ask each time",
    consequence: "Each switch lists what was found and you decide for that switch only.",
  },
  {
    value: "decide_later",
    title: "Decide later",
    consequence:
      "Nothing is saved and no pack is built until you choose — your first switch will ask.",
  },
];

/**
 * The Summary restatement (Nielsen H3): the ACTIVE choice in one line
 * — "Packs: strip secrets (all tools)". `null` is said plainly:
 * nothing chosen, the first switch asks.
 */
export function packPolicyRestatement(
  effective: PackPolicyWire | null,
  source: "global" | "override" | "unset",
  providerName: string,
): string {
  if (effective === null) {
    return "Packs: not chosen yet — your first switch asks before any pack is built";
  }
  const scope = source === "override" ? `${providerName} override` : "all tools";
  const words =
    effective === "strip"
      ? "strip secrets"
      : effective === "keep"
        ? "keep secrets — packs stay on this Mac"
        : "ask each time";
  return `Packs: ${words} (${scope})`;
}
