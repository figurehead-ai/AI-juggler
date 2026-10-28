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
    title: "Hide them (recommended)",
    consequence:
      "Keys and passwords in a conversation are replaced with ••• before another tool reads it. If it needs a key, it asks you.",
  },
  {
    value: "keep",
    title: "Leave them in",
    consequence:
      "The other tool, and the company behind it, read the keys exactly as written. Use this only if the work needs them.",
  },
  {
    value: "ask",
    title: "Ask on every switch",
    consequence: "Each switch to another tool lists what was found, and you decide for that switch only.",
  },
  {
    value: "decide_later",
    title: "Decide later",
    consequence:
      "Nothing is saved until you choose: your first switch to another tool asks.",
  },
];

/**
 * The Summary restatement (Nielsen H3): the ACTIVE choice in one line
 * — "Keys: hidden (all tools)". `null` is said plainly:
 * nothing chosen, the first switch asks.
 */
export function packPolicyRestatement(
  effective: PackPolicyWire | null,
  source: "global" | "override" | "unset",
  providerName: string,
): string {
  if (effective === null) {
    return "Keys: not chosen yet — your first switch to another tool asks";
  }
  const scope = source === "override" ? `${providerName} override` : "all tools";
  const words =
    effective === "strip"
      ? "hidden from other tools"
      : effective === "keep"
        ? "left in for other tools to read"
        : "asked on every switch";
  return `Keys: ${words} (${scope})`;
}
