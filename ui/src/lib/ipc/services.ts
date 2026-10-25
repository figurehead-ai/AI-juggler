/**
 * AJ-E1601 (D-34) — accounts through another service (OpenCode Go, Kimi)
 * on Claude Code and Codex. The backend's `features::service_accounts`.
 *
 * The key goes to the backend once, when the account is added (or its key
 * replaced), and lands in the macOS Keychain. Nothing here keeps it, logs
 * it or gets it back.
 */
import { invoke } from "./transport";
import { hasLiveBackend, type AccountInfo } from "../bridge";
import { PREVIEW_SERVICES } from "../../preview/services";
import type { AccountServiceInfo } from "../../types/generated/AccountServiceInfo";
import type { ServiceAccountRequest } from "../../types/generated/ServiceAccountRequest";
import type { ServiceInfo } from "../../types/generated/ServiceInfo";

export type { AccountServiceInfo, ServiceAccountRequest, ServiceInfo };

type PreviewWindow = { __AIJ_SERVICES__?: ServiceInfo[]; __AIJ_SERVICE_ADDED__?: Omit<ServiceAccountRequest, "key">[] };

/** The services an account can be added through, with the tools here that
 * can run through each. */
export async function servicesList(): Promise<ServiceInfo[]> {
  if (!hasLiveBackend()) return (window as unknown as PreviewWindow).__AIJ_SERVICES__ ?? PREVIEW_SERVICES;
  return invoke<ServiceInfo[]>("service_accounts_catalog");
}

/** Adds the account: its key to the Keychain, then each profile. The
 * preview records what was asked, without the key. */
export async function serviceAccountAdd(request: ServiceAccountRequest): Promise<AccountInfo[]> {
  if (!hasLiveBackend()) {
    const page = window as unknown as PreviewWindow;
    const asked = { service: request.service, plan: request.plan, profiles: request.profiles };
    page.__AIJ_SERVICE_ADDED__ = [...(page.__AIJ_SERVICE_ADDED__ ?? []), asked];
    return [];
  }
  return invoke<AccountInfo[]>("service_accounts_add", { request });
}

/** Replaces the key of the account `profileId` is on. */
export async function serviceAccountSetKey(profileId: string, key: string): Promise<void> {
  if (!hasLiveBackend()) return;
  return invoke<void>("service_accounts_set_key", { profileId, key });
}
