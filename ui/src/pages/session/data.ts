/**
 * AJ-E2108 — what a session's page reads besides the session: every
 * account's runway and the profiles' names. Read once per half minute at
 * most and shared by every session page (a page that opens, a tab that
 * comes back, a live refresh never asks again while it's fresh). 0.3.4:
 * the last answer shows in a page's first frame (lib/shared-read); the
 * runways are the ones Home, Accounts and Usage keep current.
 */
import { accountsList, type AccountInfo, type QuotaDisplayData } from "../../lib/bridge";
import { useQuotaDisplays } from "../../lib/kept-reads";
import { sharedRead, useSharedRead } from "../../lib/shared-read";

const FRESH_MS = 30_000;

/** Every profile, as the session pages last read it. */
export const profiles = sharedRead<AccountInfo[]>(() => accountsList());

const NO_PROFILES: AccountInfo[] = [];

/** Every account's runway (the shared one; read when older than half a
 * minute). */
export function useRunways(): QuotaDisplayData[] {
  return useQuotaDisplays({ pollMs: 0, maxAgeMs: FRESH_MS }).displays;
}

/** Every profile (shared, fresh for half a minute). */
export function useProfiles(): AccountInfo[] {
  return useSharedRead(profiles, { maxAgeMs: FRESH_MS }) ?? NO_PROFILES;
}
