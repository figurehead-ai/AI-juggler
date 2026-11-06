/**
 * The tools AI Juggler knows: the provider list, what is installed
 * (AJ-E301) and what each can do, and the full environment scan.
 */
import { invoke, listen, liveBackend as hasBackend } from "./transport";
import type { DiscoveredInfo } from "../../types/generated/DiscoveredInfo";
import type { ProviderInfo } from "../../types/generated/ProviderInfo";

export async function providersList(): Promise<ProviderInfo[]> {
  if (!hasBackend) {
    return [{ id: "claude-code", name: "Claude Code" }];
  }
  return invoke<ProviderInfo[]>("providers_list");
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

/** The no-backend Step-0 default (preview mode shows an installed
 * Claude + a missing second Provider — both gate states visible). */
const DEFAULT_PROVIDER_INSTALLS: ProviderInstallData[] = [
  {
    provider: "claude-code",
    display_name: "Claude Code",
    install_url: "https://claude.com/download",
    status: "installed",
    version: "2.1.296",
    binary: "/usr/local/bin/claude",
    notes: [],
  },
  {
    provider: "codex",
    display_name: "Codex",
    install_url: "https://developers.openai.com/codex",
    status: "not_installed",
    version: null,
    binary: null,
    notes: [],
  },
];

/** AJ-E301 Req.2/3: the per-Provider install probe (Step 0). A
 * missing binary is a ROW, never an error. */
export async function providersDiscover(): Promise<ProviderInstallData[]> {
  if (!hasBackend) {
    // Test seam: the probe itself fails.
    const failure = (window as unknown as { __AIJ_PROVIDER_INSTALLS_ERROR__?: unknown }).__AIJ_PROVIDER_INSTALLS_ERROR__;
    if (failure !== undefined) throw failure;
    return (
      (window as unknown as { __AIJ_PROVIDER_INSTALLS__?: ProviderInstallData[] })
        .__AIJ_PROVIDER_INSTALLS__ ?? DEFAULT_PROVIDER_INSTALLS
    );
  }
  return invoke<ProviderInstallData[]>("providers_discover");
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

const DEFAULT_PROVIDER_CAPABILITIES: ProviderCapabilityData[] = [
  { provider: "claude-code", can_use_in_place: true, reports_limits: true },
  // Its rollouts carry its own limit readings (the 0.0.12 report).
  { provider: "codex", can_use_in_place: true, reports_limits: true },
];

/** ADR-0018: each registered Provider's static capabilities. */
export async function providersCapabilities(): Promise<ProviderCapabilityData[]> {
  if (!hasBackend) {
    // Test seam: the call itself fails.
    const failure = (window as unknown as { __AIJ_PROVIDER_CAPABILITIES_ERROR__?: unknown }).__AIJ_PROVIDER_CAPABILITIES_ERROR__;
    if (failure !== undefined) throw failure;
    return (
      (window as unknown as { __AIJ_PROVIDER_CAPABILITIES__?: ProviderCapabilityData[] }).__AIJ_PROVIDER_CAPABILITIES__ ??
      DEFAULT_PROVIDER_CAPABILITIES
    );
  }
  return invoke<ProviderCapabilityData[]>("providers_capabilities");
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

/** The no-backend report default: one install + ten honest probes. */
export const DEFAULT_SCAN_REPORT: ScanReportData = {
  installs: [
    {
      harness: "claude-code",
      path: "/Users/you/.claude",
      origin: "vendor-default",
      set_by: null,
      login: "logged_in",
      bytes: 48_000_000,
      last_used_secs: Math.floor(Date.now() / 1000) - 86_400,
      managed: false,
      in_use: [],
      notes: ["Claude Code's standard location — what a plain `claude` command uses."],
      inference: {
        purpose: "Main",
        purpose_confidence: "found",
        purpose_rule: "usual-folder",
        login: "signed-in",
        login_confidence: "found",
        login_rule: "sign-in-present",
        activity: "active",
        activity_confidence: "found",
        activity_rule: "newest-conversation",
        activity_words: "Active this week",
        keychain_bound: false,
      },
      signals: ["vendor-default"],
    },
    {
      harness: "claude-code",
      path: "/Users/you/.claude-side",
      origin: "home-pattern",
      set_by: null,
      login: "logged_out",
      bytes: 1_200_000,
      last_used_secs: Math.floor(Date.now() / 1000) - 40 * 86_400,
      managed: false,
      in_use: [],
      notes: ["A separate Claude configuration — created manually or by another tool."],
      inference: {
        purpose: "Personal",
        purpose_confidence: "likely",
        purpose_rule: "name-word",
        login: "not-signed-in",
        login_confidence: "found",
        login_rule: "no-sign-in",
        activity: "dormant",
        activity_confidence: "found",
        activity_rule: "newest-conversation",
        activity_words: "Dormant 40 d",
        keychain_bound: false,
      },
      signals: ["home-pattern", "CLAUDE_CONFIG_DIR in ~/.zshrc:12"],
    },
  ],
  probes: ([
    ["s1-config-dirs", "Folders named .claude…", "1 candidate setup(s)"],
    ["s2-env-overrides", "A config folder set for this app (CLAUDE_CONFIG_DIR)", "none set in this app's environment"],
    ["s3-manager-t3", "Claude profiles (ClaudeCodeProfiles)", "not present"],
    ["s4-manager-cc-switch", "cc-switch profiles", "not present"],
    ["s5-shell-aliases", "Your shell settings (aliases, CLAUDE_CONFIG_DIR)", "0 assignment(s)"],
    ["s6-installed-clis", "Claude Code installed", "claude-code: installed"],
    ["s7-login-metadata", "Sign-in status", "1 of 1 signed in"],
    ["s8-usage-recency", "When each was last used", "1 setup(s) with recent activity"],
    ["s9-projects", "Recent projects", "3 project name(s)"],
    ["s10-desktop", "Desktop apps", "Claude.app"],
  ] as const).map(([id, label, detail]) => ({
    id,
    label,
    health: "ok",
    reason: null,
    detail,
  })),
  seeds: ["-users-dev-app", "-users-dev-ai-juggler", "-users-dev-playground"],
  desktop_apps: ["Claude.app"],
  cancelled: false,
};

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
    return seam ?? DEFAULT_SCAN_REPORT;
  }
  let stop: (() => void) | undefined;
  if (progress) {
    stop = await listen<{ done: number; total: number }>("scan-progress", (event) => {
      progress(event.payload.done, event.payload.total);
    }).then((unlisten) => unlisten);
  }
  try {
    return await invoke<ScanReportData>("environment_scan_full");
  } finally {
    stop?.();
  }
}
