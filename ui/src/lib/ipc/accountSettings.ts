/**
 * AJ-E1508: a new account starts with the user's usual settings from an
 * account of the same tool: its MCP servers, permissions, status line and
 * plugins. The tests read the seam `__AIJ_ACCOUNT_SETTINGS__` (what each
 * source account offers) instead of the backend, and the preview its
 * example data.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";
import type { AppliedInfo, SetupCheckInfo } from "./switchSetup";
import { EXAMPLE_ACCOUNT_SETTINGS } from "../../preview/account-settings";

const seam = () =>
  (window as unknown as { __AIJ_ACCOUNT_SETTINGS__?: Record<string, SetupCheckInfo> }).__AIJ_ACCOUNT_SETTINGS__ ??
  EXAMPLE_ACCOUNT_SETTINGS;

/** What a new account (`target`, or one about to be made: `null`) could
 * start with from `source`. Reads only. */
export async function accountSettingsCheck(target: string | null, source: string): Promise<SetupCheckInfo> {
  if (!hasLiveBackend()) {
    return seam()?.[source] ?? { from: source, to: "the new account", offers: [], already_there: 0 };
  }
  return invoke<SetupCheckInfo>("account_settings_check", { target, source });
}

/** Copies the picked settings from `source` into the new account `target`. */
export async function accountSettingsApply(target: string, source: string, offers: string[]): Promise<AppliedInfo[]> {
  if (!hasLiveBackend()) {
    const check = seam()?.[source];
    return offers.map((id) => {
      const offer = check?.offers.find((candidate) => candidate.id === id);
      return offer
        ? { offer: id, record: `record-${id}`, message: `Set up the ${offer.name}`, note: null }
        : { offer: id, record: null, message: "It isn't on that account any more", note: null };
    });
  }
  return invoke<AppliedInfo[]>("account_settings_apply", { target, source, offers });
}
