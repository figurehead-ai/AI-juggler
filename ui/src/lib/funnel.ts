/**
 * AJ-E305: the onboarding funnel seam. Every flow step emits one
 * `aij:funnel` CustomEvent on `window` — `{ flow, step }` with
 * string keys only — which the Playwright click-budget specs listen
 * to for missing-step gaps. Tests can also spy on `funnelStep`.
 *
 * AJ-E309: the seam now also feeds the LOCAL event store. Steps are
 * mirrored as `flow_step{flow,step}`, and `funnelEvent` records the
 * Req.6 events (scan, proposal, import, login, expert, first launch,
 * inference shown/edit, policy choice). Fields are counts, fixed
 * vocabulary words, or lowercase codes — the backend's allow-list
 * schema refuses anything else, so a path or label can never land in
 * the store. Emission is fire-and-forget: the UI never waits on
 * instrumentation, and a refused event is dropped silently.
 */

import { funnelEventEmit } from "./bridge";

export type FunnelFlow = "a-first-install" | "b-new-project" | "c-add-provider";
export type FunnelStep = string;

/** The Req.6/7 event kinds (mirrors the backend allow-list). */
export type FunnelEventKind =
  | "scan_started"
  | "scan_completed"
  | "proposal_shown"
  | "proposal_accepted"
  | "proposal_customised"
  | "import_started"
  | "import_completed"
  | "import_failed"
  | "login_queued"
  | "login_completed"
  | "login_skipped"
  | "login_expired"
  | "expert_opened"
  | "first_launch"
  | "inference_shown"
  | "inference_edit"
  | "policy_chosen";

const seen = new Map<string, string[]>();

/** Records one event in the local store (fire-and-forget). Numbers
 * are stringified; the backend validates every value's shape. */
export function funnelEvent(
  kind: FunnelEventKind,
  fields: Record<string, string | number | boolean> = {},
): void {
  const wire: Record<string, string> = {};
  for (const [key, value] of Object.entries(fields)) {
    wire[key] = String(value);
  }
  void funnelEventEmit(kind, wire).catch(() => null);
}

/** Emit one funnel step. Idempotent per (flow, step): a step the user
 * re-visits (Back) does not double-count. */
export function funnelStep(flow: FunnelFlow, step: FunnelStep): void {
  const log = seen.get(flow) ?? [];
  if (log[log.length - 1] !== step) {
    log.push(step);
    seen.set(flow, log);
    void funnelEventEmit("flow_step", { flow, step }).catch(() => null);
  }
  window.dispatchEvent(
    new CustomEvent("aij:funnel", { detail: { flow, step } }),
  );
}

/** The ordered steps observed for a flow (test seam). */
export function funnelSeen(flow: FunnelFlow): string[] {
  return [...(seen.get(flow) ?? [])];
}

/** Forget a flow's history (test seam). */
export function funnelReset(flow?: FunnelFlow): void {
  if (flow === undefined) {
    seen.clear();
  } else {
    seen.delete(flow);
  }
}

/** The coded reason of a rejected command (`already_imported`, …) —
 * the `import_failed{reason_code}` field. Never the message text. */
export function reasonCode(cause: unknown): string {
  if (cause !== null && typeof cause === "object") {
    const code = (cause as { code?: unknown }).code;
    if (typeof code === "string" && /^[a-z][a-z0-9_-]{0,47}$/.test(code)) {
      return code;
    }
  }
  return "unknown";
}
