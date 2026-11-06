/**
 * What a switch carries and redacts: the redaction policy.
 */
import { call, liveBackend as hasBackend } from "./transport";

/** AJ-E208: the resolved redaction policy for one Provider. */
export interface RedactionInfoData {
  global: string | null;
  override: string | null;
  effective: string | null;
  source: "global" | "override" | "unset";
  /** The verbatim confirm line ("" when unset). */
  line: string;
}

/** AJ-E208: one masked detection (values never cross the bridge). */
export interface RedactionItemData {
  kind: string;
  preview: string;
  segment: number;
}

/** AJ-E208: the redaction policy for one Provider (settings + the
 * switch confirm line). */
export async function redactionPolicyGet(
  provider: string,
): Promise<RedactionInfoData> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_REDACTION_POLICY__?: RedactionInfoData })
        .__AIJ_REDACTION_POLICY__ ?? {
        global: null,
        override: null,
        effective: null,
        source: "unset",
        line: "",
      }
    );
  }
  return call("redaction_policy_get", { provider });
}

/** AJ-E208: sets (or clears) the GLOBAL policy — Accounts → Pack
 * secrets, the first-run chooser, and the switch chooser. */
export async function redactionPolicySetGlobal(
  policy: string | null,
): Promise<void> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_REDACTION_SET_GLOBAL__?: (string | null)[];
      __AIJ_REDACTION_POLICY__?: RedactionInfoData;
    };
    seams.__AIJ_REDACTION_SET_GLOBAL__ = [
      ...(seams.__AIJ_REDACTION_SET_GLOBAL__ ?? []),
      policy,
    ];
    // Keep the preview coherent: a later read reflects the choice.
    const current = seams.__AIJ_REDACTION_POLICY__;
    const override = current?.override ?? null;
    seams.__AIJ_REDACTION_POLICY__ = {
      global: policy,
      override,
      effective: override ?? policy,
      source: override !== null ? "override" : policy !== null ? "global" : "unset",
      line: current?.line ?? "",
    };
    return;
  }
  await call("redaction_policy_set_global", { policy });
}

/** AJ-E208: sets (or clears) one Provider's override — Provider
 * settings. */
export async function redactionPolicySetOverride(
  provider: string,
  policy: string | null,
): Promise<void> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_REDACTION_SET_OVERRIDE__?: [string, string | null][] };
    seams.__AIJ_REDACTION_SET_OVERRIDE__ = [
      ...(seams.__AIJ_REDACTION_SET_OVERRIDE__ ?? []),
      [provider, policy],
    ];
    return;
  }
  await call("redaction_policy_set_override", { provider, policy });
}
