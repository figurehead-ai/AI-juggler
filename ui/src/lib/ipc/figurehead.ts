/**
 * AJ-E1202/E1203: the optional Figurehead account.
 */
import { call, liveBackend as hasBackend } from "./transport";
import type { CloudFeatureInfo } from "../../types/generated/CloudFeatureInfo";
import type { FigureheadStatusInfo } from "../../types/generated/FigureheadStatusInfo";
import { previewHooks } from "../preview-hooks";

//
// Scripted mock (browser/Playwright): `__AIJ_FIGUREHEAD__` holds the
// status; `__AIJ_FIGUREHEAD_APPROVE__ = "approved" | "denied" | "expired"`
// decides the next poll; a pairing code "BAD" is refused;
// `__AIJ_FIGUREHEAD_FEATURES__` overrides the account's features
// (e.g. a paid-locked one).

type FigureheadSeams = {
  __AIJ_FIGUREHEAD__?: FigureheadStatusInfo;
  __AIJ_FIGUREHEAD_APPROVE__?: "approved" | "denied" | "expired";
  __AIJ_FIGUREHEAD_FEATURES__?: CloudFeatureInfo[];
  __AIJ_FLEET_CALLS__?: string[];
};

const fhSeams = () => window as unknown as FigureheadSeams;

function fhStatus(): FigureheadStatusInfo {
  return (
    fhSeams().__AIJ_FIGUREHEAD__ ?? {
      state: "signed_out",
      email: null,
      plan: null,
      device_name: "Studio Mac",
      features: [],
      link: null,
      refreshed_at_ms: null,
      offline: false,
      notice: null,
      consent_version: 0,
      consent_current: 1,
    }
  );
}

function fhSet(next: FigureheadStatusInfo): FigureheadStatusInfo {
  fhSeams().__AIJ_FIGUREHEAD__ = next;
  return { ...next };
}

function fhCall(called: string): void {
  const seams = fhSeams();
  seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), called];
}

function fhLinked(name?: string): FigureheadStatusInfo {
  return fhSet({
    ...fhStatus(),
    state: "linked",
    email: "you@example.com",
    plan: "Free",
    device_name: name?.trim() || fhStatus().device_name,
    features: (fhSeams().__AIJ_FIGUREHEAD_FEATURES__ ?? previewHooks().fleet.MOCK_FEATURES).map((f) => ({ ...f })),
    link: null,
    refreshed_at_ms: Date.now(),
    notice: null,
  });
}

/** The Figurehead status (local — never a network call). */
export async function figureheadStatus(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) return { ...fhStatus() };
  return call("figurehead_status");
}

/** Starts a browser link and opens the approval page. */
export async function figureheadLinkStart(name?: string): Promise<FigureheadStatusInfo> {
  if (!hasBackend) {
    fhCall(`fh-link-start:${name ?? ""}`);
    return fhSet({
      ...fhStatus(),
      state: "linking",
      link: { user_code: "QVM3-GABM", phase: "waiting", expires_at_ms: Date.now() + 600_000, message: null },
    });
  }
  return call("figurehead_link_start", { name: name ?? null });
}

/** Polls the browser link (the app paces it to the server's interval). */
export async function figureheadLinkPoll(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) {
    const decision = fhSeams().__AIJ_FIGUREHEAD_APPROVE__;
    const status = fhStatus();
    if (status.state !== "linking" || !status.link || !decision) return { ...status };
    if (decision === "approved") return fhLinked();
    return fhSet({ ...status, link: { ...status.link, phase: decision } });
  }
  return call("figurehead_link_poll");
}

/** Cancels a browser link. */
export async function figureheadLinkCancel(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) return fhSet({ ...fhStatus(), state: "signed_out", link: null });
  return call("figurehead_link_cancel");
}

/** Links with a pairing code from the Figurehead dashboard. */
export async function figureheadClaim(code: string, name?: string): Promise<FigureheadStatusInfo> {
  if (!hasBackend) {
    fhCall(`fh-claim:${code}`);
    if (code.trim().toUpperCase() === "BAD") {
      throw {
        code: "validation",
        retryable: false,
        user_message: "That pairing code is wrong or has expired — generate a new one on figurehead.ai.",
      };
    }
    return fhLinked(name);
  }
  return call("figurehead_claim", { code, name: name ?? null });
}

/** Refreshes the account now. */
export async function figureheadRefresh(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) return { ...fhStatus() };
  return call("figurehead_refresh");
}

/** Renames this Mac on Figurehead. */
export async function figureheadRename(name: string): Promise<FigureheadStatusInfo> {
  if (!hasBackend) {
    fhCall(`fh-rename:${name.trim()}`);
    return fhSet({ ...fhStatus(), device_name: name.trim() });
  }
  return call("figurehead_rename", { name });
}

/** Unlinks this Mac (local cleanup happens even offline). */
export async function figureheadUnlink(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) {
    fhCall("fh-unlink");
    return fhSet({
      ...fhStatus(),
      state: "signed_out",
      email: null,
      plan: null,
      features: [],
      link: null,
      consent_version: 0,
    });
  }
  return call("figurehead_unlink");
}

/** Dismisses the one-time notice. */
export async function figureheadDismissNotice(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) return fhSet({ ...fhStatus(), notice: null });
  return call("figurehead_dismiss_notice");
}

/** Stores the cloud choices (consent screen / toggles). */
export async function figureheadSaveChoices(features: Record<string, boolean>): Promise<FigureheadStatusInfo> {
  if (!hasBackend) {
    fhCall(`fh-choices:${Object.entries(features).filter(([, on]) => on).map(([k]) => k).sort().join(",")}`);
    const status = fhStatus();
    return fhSet({
      ...status,
      consent_version: status.consent_current,
      features: status.features.map((f) => ({
        ...f,
        on: Boolean(features[f.key]) && f.access === "included" && f.key !== "usageSync",
      })),
    });
  }
  return call("figurehead_save_choices", { features });
}

/** Opens a Figurehead page (`verify` | `dashboard` | `devices` | `upgrade`). */
export async function figureheadOpenPage(target: "verify" | "dashboard" | "devices" | "upgrade"): Promise<void> {
  if (!hasBackend) {
    fhCall(`fh-open:${target}`);
    return;
  }
  return call("figurehead_open_page", { target });
}
