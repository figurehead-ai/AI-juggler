import { useId } from "react";

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

export function PackPolicyChooser({
  value,
  onChange,
  legend = "How should context packs handle secrets?",
  hint = "Packs carry your work between accounts when you switch. You choose once; it's remembered, and each Provider can override it in Accounts.",
  allowDecideLater = true,
  name = "pack-policy",
  testId = "policy-panel",
}: {
  value: PackPolicyChoice | null;
  onChange: (choice: PackPolicyChoice) => void;
  legend?: string;
  hint?: string;
  allowDecideLater?: boolean;
  name?: string;
  testId?: string;
}): JSX.Element {
  const base = useId();
  const hintId = `${base}-hint`;
  return (
    <fieldset
      className="flex flex-col gap-2 rounded-xl border border-hairline px-4 py-3"
      aria-describedby={hintId}
      data-testid={testId}
      data-chosen={value ?? "none"}
    >
      <legend className="px-1 text-xs font-medium text-ink">{legend}</legend>
      <p id={hintId} className="text-[11px] leading-relaxed text-muted">
        {hint}
      </p>
      {PACK_POLICY_OPTIONS.filter((option) => allowDecideLater || option.value !== "decide_later").map(
        (option) => {
          const consequenceId = `${base}-${option.value}`;
          const checked = value === option.value;
          return (
            <label
              key={option.value}
              className={
                "flex cursor-default items-start gap-2.5 rounded-lg border px-3 py-2 text-xs transition-colors " +
                (checked
                  ? "border-accent/40 bg-accent-soft"
                  : "border-hairline hover:border-accent/30")
              }
              data-testid="policy-option"
              data-value={option.value}
              data-selected={checked ? "true" : "false"}
            >
              <input
                type="radio"
                name={name}
                value={option.value}
                checked={checked}
                onChange={() => onChange(option.value)}
                aria-describedby={consequenceId}
                className="mt-0.5 size-3.5 shrink-0 accent-[var(--color-accent)]"
              />
              <span className="flex flex-col gap-0.5">
                <span className="font-medium text-ink">{option.title}</span>
                <span id={consequenceId} className="text-[11px] leading-relaxed text-muted">
                  {option.consequence}
                </span>
              </span>
            </label>
          );
        },
      )}
    </fieldset>
  );
}

/**
 * The Summary restatement (Nielsen H3): the ACTIVE choice in one line
 * — "Packs: strip secrets (all Providers)". `null` is said plainly:
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
  const scope = source === "override" ? `${providerName} override` : "all Providers";
  const words =
    effective === "strip"
      ? "strip secrets"
      : effective === "keep"
        ? "keep secrets — packs stay on this Mac"
        : "ask each time";
  return `Packs: ${words} (${scope})`;
}
