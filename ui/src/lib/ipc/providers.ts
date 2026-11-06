/**
 * The tools AI Juggler knows: the provider list, what is installed
 * (AJ-E301) and what each can do, and the full environment scan.
 */
import { call, listen, liveBackend as hasBackend } from "./transport";
import type { DiscoveredInfo } from "../../types/generated/DiscoveredInfo";
import type { ProviderInfo } from "../../types/generated/ProviderInfo";
import { previewHooks } from "../preview-hooks";

export async function providersList(): Promise<ProviderInfo[]> {
  if (!hasBackend) {
    return [{ ...previewHooks().fixtures.PREVIEW_TOOL }];
  }
  return call("providers_list");
}

/** @deprecated legacy alias (AJ-E107) — use providersList. */
export const harnessesList = providersList;

/** AJ-E301: one Step-0 install-gate row. */
export interface ProviderInstallData {
  provider: string;
  display_name: string;
  install_url: string;
  /** installed | not_installed | unverified | probe_failed | found (its version not asked) */
  status: string;
  version?: string | null;
  binary?: string | null;
  notes: string[];
  /** AJ-E306 flow (c): the verified version range(s) — the card
   * badge; absent/empty renders "unverified" honestly. */
  verified_against?: string[];
  /** `terminal` | `desktop` — the kind badge (Req.3). */
  kind?: "terminal" | "desktop";
  /** The single-instance desktop limit, stated BEFORE selection. */
  single_instance_note?: string | null;
}

/** AJ-E301 Req.2/3: the per-Provider install probe (Step 0). A
 * missing binary is a ROW, never an error. */
export async function providersDiscover(): Promise<ProviderInstallData[]> {
  if (!hasBackend) {
    // Test seam: the probe itself fails.
    const failure = (window as unknown as { __AIJ_PROVIDER_INSTALLS_ERROR__?: unknown }).__AIJ_PROVIDER_INSTALLS_ERROR__;
    if (failure !== undefined) throw failure;
    return (
      (window as unknown as { __AIJ_PROVIDER_INSTALLS__?: ProviderInstallData[] })
        .__AIJ_PROVIDER_INSTALLS__ ?? previewHooks().providers.DEFAULT_PROVIDER_INSTALLS
    );
  }
  return call("providers_discover");
}

/** What a registered Provider supports — from the registry alone, no
 * install probe (cheap on every screen). */
export interface ProviderCapabilityData {
  provider: string;
  /** ADR-0018: its setups can be used where they are. */
  can_use_in_place: boolean;
  /** It reports its own usage limits (5-hour and weekly windows). */
  reports_limits: boolean;
}

/** ADR-0018: each registered Provider's static capabilities. */
export async function providersCapabilities(): Promise<ProviderCapabilityData[]> {
  if (!hasBackend) {
    // Test seam: the call itself fails.
    const failure = (window as unknown as { __AIJ_PROVIDER_CAPABILITIES_ERROR__?: unknown }).__AIJ_PROVIDER_CAPABILITIES_ERROR__;
    if (failure !== undefined) throw failure;
    return (
      (window as unknown as { __AIJ_PROVIDER_CAPABILITIES__?: ProviderCapabilityData[] }).__AIJ_PROVIDER_CAPABILITIES__ ??
      previewHooks().providers.DEFAULT_PROVIDER_CAPABILITIES
    );
  }
  return call("providers_capabilities");
}

/** AJ-E302: one probe's health row (S1–S10). */
export interface ProbeRowData {
  id: string;
  label: string;
  /** ok | partial | denied */
  health: string;
  reason?: string | null;
  detail: string;
}

/** AJ-E302: the 10-signal scan report — installs plus per-probe
 * health, project seeds, desktop apps, and the cancellation flag. */
export interface ScanReportData {
  installs: DiscoveredInfo[];
  probes: ProbeRowData[];
  seeds: string[];
  desktop_apps: string[];
  cancelled: boolean;
}

/** AJ-E302: the full 10-signal scan. `progress` streams
 * (done, total) — the backend emits events; preview mode simulates.
 * Read-only + metadata-only. */
export async function environmentScanFull(
  progress?: (done: number, total: number) => void,
): Promise<ScanReportData> {
  if (!hasBackend) {
    const seam =
      (window as unknown as { __AIJ_SCAN_REPORT__?: ScanReportData }).__AIJ_SCAN_REPORT__;
    const total = 10;
    for (let done = 1; done <= total; done += 1) {
      progress?.(done, total);
      await new Promise((resolve) => setTimeout(resolve, 40));
    }
    const fails = (window as unknown as { __AIJ_SCAN_FAILS__?: string }).__AIJ_SCAN_FAILS__;
    if (fails) throw new Error(fails);
    return seam ?? previewHooks().providers.DEFAULT_SCAN_REPORT;
  }
  let stop: (() => void) | undefined;
  if (progress) {
    stop = await listen<{ done: number; total: number }>("scan-progress", (event) => {
      progress(event.payload.done, event.payload.total);
    }).then((unlisten) => unlisten);
  }
  try {
    return await call("environment_scan_full");
  } finally {
    stop?.();
  }
}
