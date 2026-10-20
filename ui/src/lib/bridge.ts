
// AJ-E307: per top-level entry bytes (the copy receipt breakdown)
// and the degraded flag; provenance echoes on AccountInfo.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import type { AppInfo } from "../types/generated/AppInfo";
import type { NotificationPrefsInfo } from "../types/generated/NotificationPrefsInfo";
import type { OsAccessInfo } from "../types/generated/OsAccessInfo";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { DiscoveredInfo } from "../types/generated/DiscoveredInfo";
import type { ImportResult } from "../types/generated/ImportResult";
import type { SourceInspectInfo } from "../types/generated/SourceInspectInfo";
import type { ExpertSetupInfo } from "../types/generated/ExpertSetupInfo";
import type { FleetSelfInfo } from "../types/generated/FleetSelfInfo";
import type { FleetPeerInfo } from "../types/generated/FleetPeerInfo";
import type { LinkStatusInfo } from "../types/generated/LinkStatusInfo";
import type { DiscoveredMachineInfo } from "../types/generated/DiscoveredMachineInfo";
import type { PairingStateInfo } from "../types/generated/PairingStateInfo";
import type { FigureheadStatusInfo } from "../types/generated/FigureheadStatusInfo";
import type { CloudFeatureInfo } from "../types/generated/CloudFeatureInfo";
import type { RemoteMachineInfo } from "../types/generated/RemoteMachineInfo";
import type { IdentityCheckInfo } from "../types/generated/IdentityCheckInfo";
import type { IsolationOfferInfo } from "../types/generated/IsolationOfferInfo";
import type { ParkedInfo } from "../types/generated/ParkedInfo";
import type { OvernightSummaryInfo } from "../types/generated/OvernightSummaryInfo";
import type { AccountMacsInfo } from "../types/generated/AccountMacsInfo";
import type { AccountMacInfo } from "../types/generated/AccountMacInfo";
import type { CloudBackupStatusInfo } from "../types/generated/CloudBackupStatusInfo";
import type { CloudBackupInfo } from "../types/generated/CloudBackupInfo";
import type { LastBackupInfo } from "../types/generated/LastBackupInfo";
import type { VaultStatusInfo } from "../types/generated/VaultStatusInfo";
import type { RecoveryCodeInfo } from "../types/generated/RecoveryCodeInfo";
import type { UpdateStatusInfo } from "../types/generated/UpdateStatusInfo";
import type { AnnouncedInfo } from "../types/generated/AnnouncedInfo";
import type { BackupExportInfo } from "../types/generated/BackupExportInfo";
import type { RestorePlanInfo } from "../types/generated/RestorePlanInfo";
import type { RestoreResultInfo } from "../types/generated/RestoreResultInfo";
import type { BackupManifestInfo } from "../types/generated/BackupManifestInfo";
import type { DeletePreviewInfo } from "../types/generated/DeletePreviewInfo";
import type { TranscriptHitInfo } from "../types/generated/TranscriptHitInfo";
import type { ReplayInfo } from "../types/generated/ReplayInfo";
import type { ReplayEntryInfo } from "../types/generated/ReplayEntryInfo";
import type { AccountStorageInfo } from "../types/generated/AccountStorageInfo";
import type { RetentionPreviewInfo } from "../types/generated/RetentionPreviewInfo";
import type { SummaryRowInfo } from "../types/generated/SummaryRowInfo";
import type { ParkedOfferInfo } from "../types/generated/ParkedOfferInfo";
import type { WorktreeRemovalInfo } from "../types/generated/WorktreeRemovalInfo";
import type { GitStatusInfo } from "../types/generated/GitStatusInfo";
import type { IdentityAlternativeInfo } from "../types/generated/IdentityAlternativeInfo";
import type { RemoteSwitchTargetInfo } from "../types/generated/RemoteSwitchTargetInfo";
import type { RemoteProjectOption } from "../types/generated/RemoteProjectOption";
import type { FleetSwitchRequest } from "../types/generated/FleetSwitchRequest";
import type { FleetSwitchOutcomeInfo } from "../types/generated/FleetSwitchOutcomeInfo";
import type { FleetSwitchProgressInfo } from "../types/generated/FleetSwitchProgressInfo";
import type { FleetSwitchBackPlanInfo } from "../types/generated/FleetSwitchBackPlanInfo";
import type { DivergenceInfo } from "../types/generated/DivergenceInfo";
import type { DivergenceRecordInfo } from "../types/generated/DivergenceRecordInfo";
import type { RemoteInstanceInfo } from "../types/generated/RemoteInstanceInfo";
import type { RemoteResourcesInfo } from "../types/generated/RemoteResourcesInfo";
import type { RemoteCommandOutcomeInfo } from "../types/generated/RemoteCommandOutcomeInfo";
import type { FleetPromptInfo } from "../types/generated/FleetPromptInfo";
import type { RemoteProcessInfo } from "../types/generated/RemoteProcessInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { InstanceQuotaInfo } from "../types/generated/InstanceQuotaInfo";
import type { InstanceResourcesInfo } from "../types/generated/InstanceResourcesInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { OrphanVerifyInfo } from "../types/generated/OrphanVerifyInfo";
import type { DesktopLaunchInfo } from "../types/generated/DesktopLaunchInfo";
import type { PreviewFrameInfo } from "../types/generated/PreviewFrameInfo";
import type { PreviewStateInfo } from "../types/generated/PreviewStateInfo";
import type { FocusOutcomeInfo } from "../types/generated/FocusOutcomeInfo";
import type { ProcessesInfo } from "../types/generated/ProcessesInfo";
import type { StopOutcomeInfo } from "../types/generated/StopOutcomeInfo";
import type { LaunchResult } from "../types/generated/LaunchResult";
import type { NeedsAttention } from "../types/generated/NeedsAttention";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { ProviderInfo } from "../types/generated/ProviderInfo";
import type { QuotaSnapshotInfo } from "../types/generated/QuotaSnapshotInfo";
import type { QualitySignalsInfo } from "../types/generated/QualitySignalsInfo";
import type { StatsInfo } from "../types/generated/StatsInfo";
import type { TerminalInfo } from "../types/generated/TerminalInfo";
import type { TokenCountsInfo } from "../types/generated/TokenCountsInfo";

export type {
  AppInfo,
  FocusOutcomeInfo,
  PreviewFrameInfo,
  PreviewStateInfo,
  QualitySignalsInfo,
  ProcessesInfo,
  StopOutcomeInfo,
  OrphanVerifyInfo,
  AccountInfo,
  DiscoveredInfo,
  ImportResult,
  InstanceInfo,
  InstanceQuotaInfo,
  InstanceResourcesInfo,
  SourceInspectInfo,
  ExpertSetupInfo,
  FleetSelfInfo,
  FleetPeerInfo,
  LinkStatusInfo,
  DiscoveredMachineInfo,
  PairingStateInfo,
  FigureheadStatusInfo,
  CloudFeatureInfo,
  RemoteMachineInfo,
  IdentityCheckInfo,
  IdentityAlternativeInfo,
  IsolationOfferInfo,
  ParkedInfo,
  ParkedOfferInfo,
  OvernightSummaryInfo,
  AccountMacsInfo,
  AccountMacInfo,
  CloudBackupStatusInfo,
  CloudBackupInfo,
  LastBackupInfo,
  VaultStatusInfo,
  RecoveryCodeInfo,
  UpdateStatusInfo,
  AnnouncedInfo,
  BackupExportInfo,
  RestorePlanInfo,
  RestoreResultInfo,
  BackupManifestInfo,
  DeletePreviewInfo,
  TranscriptHitInfo,
  ReplayInfo,
  ReplayEntryInfo,
  AccountStorageInfo,
  RetentionPreviewInfo,
  SummaryRowInfo,
  WorktreeRemovalInfo,
  GitStatusInfo,
  RemoteSwitchTargetInfo,
  RemoteProjectOption,
  FleetSwitchRequest,
  FleetSwitchOutcomeInfo,
  FleetSwitchProgressInfo,
  FleetSwitchBackPlanInfo,
  DivergenceInfo,
  DivergenceRecordInfo,
  RemoteInstanceInfo,
  RemoteResourcesInfo,
  RemoteCommandOutcomeInfo,
  FleetPromptInfo,
  RemoteProcessInfo,
  LaunchRecipeInfo,
  LaunchResult,
  NeedsAttention,
  ProjectInfo,
  ProviderInfo,
  QuotaSnapshotInfo,
  StatsInfo,
  TerminalInfo,
  TokenCountsInfo,
};

// Deprecated one-version aliases (AJ-E107).
export type { ProviderInfo as HarnessInfo } from "../types/generated/ProviderInfo";
export type { LaunchRecipeInfo as ProfileInfo } from "../types/generated/LaunchRecipeInfo";
export type { InstanceInfo as SessionInfo } from "../types/generated/InstanceInfo";

/** Event names mirrored from juggler_app constants (single source: Rust). */
export const EVENTS = {
  appReady: "app://ready",
  sessionsUpdated: "session://updated",
  /** 5 s resource tick (I-07/I-08 deltas; AJ-E401). */
  sessionsResources: "session://resources",
  /** 30 s quota tick (I-05 deltas; AJ-E401). */
  sessionsQuota: "session://quota",
  /** Global "Go to instance…" hotkey (AJ-E414). */
  goTo: "app://go-to",
  /** Live-preview frame (AJ-E415). */
  previewFrame: "preview://frame",
  /** Live-preview phase change (AJ-E415). */
  previewState: "preview://state",
  /** Cross-machine switch push progress (AJ-E706). */
  fleetSwitchProgress: "fleet://switch-progress",
  /** Backup export progress (AJ-E1009): at least every 1 %. */
  backupProgress: "backup://progress",
  /** Update status changes (AJ-E1010). */
  updateStatus: "update://status",
} as const;

const hasBackend =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function hasLiveBackend(): boolean {
  return hasBackend;
}

// --- design-preview mocks (plain-browser + visual harness) ----------------
// The harness (AJ-E802) seeds `window.__AIJ_FIXTURE__` before this module
// loads; mock calls then serve the seeded fixture set (deterministic
// captures) instead of the defaults below.

export interface FixtureSet {
  accounts: AccountInfo[];
  profiles: LaunchRecipeInfo[];
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  stats: StatsInfo;
}

declare global {
  interface Window {
    __AIJ_FIXTURE__?: Partial<FixtureSet>;
  }
}

const seeded = (typeof window !== "undefined" && window.__AIJ_FIXTURE__) || {};

/** A `YYYY-MM-DD` string N days from today (fixture dates move with the clock). */
function renewalFixtureInDays(days: number): string {
  const target = new Date(Date.now() + days * 86_400_000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${target.getFullYear()}-${pad(target.getMonth() + 1)}-${pad(target.getDate())}`;
}

const MOCK_ACCOUNTS: AccountInfo[] = [
  { id: "claude-work", provider: "claude-code", label: "Work Max", plan: Some("max-20x"), login_state: "logged_in", cleanup_opt_in: false, renewal_date: Some(renewalFixtureInDays(6)), preview_opt_in: false },
  { id: "claude-personal", provider: "claude-code", label: "Personal", plan: None(), login_state: "logged_out", cleanup_opt_in: false, renewal_date: None(), preview_opt_in: false },
];

const MOCK_PROFILES: LaunchRecipeInfo[] = [
  { id: "p-1", name: "Deep work", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: Some("opus"), default_cwd: None() },
  { id: "p-2", name: "Quick fixes", account_id: "claude-work", account_label: "Work Max", provider: "claude-code", model: Some("sonnet"), default_cwd: None() },
  { id: "p-3", name: "Weekend build", account_id: "claude-personal", account_label: "Personal", provider: "claude-code", model: None(), default_cwd: None() },
];

const MOCK_QUALITY: Record<string, QualitySignalsInfo> = {
  // sess-8 sorts first (needs-me) and is what the captures click; sess-2
  // is the running row.
  "sess-8": {
    tokens_bars: [300, 410, 380, 500, 440],
    avg_tokens_per_min: 406,
    error_label: "1 errors / 12 turns",
    errors_not_measurable: false,
    context_used: null,
    context_window: null,
    cumulative: { input: 4_200, output: 1_800, cache_read: 20, cache_write: 40, reasoning: 8 },
    session_cost_usd: 0.19,
    priced_model: "opus",
    quota_forecast: "not burning",
    stuck: { rule: "stuck_no_output", inputs: ["no output 8m", "threshold 5m", "cpu 0.3%", "transcript stale 8m"], since_ms: Date.now() - 8 * 60_000 },
  },
  "sess-2": {
    tokens_bars: [420, 510, 380, 600, 540],
    avg_tokens_per_min: 490,
    error_label: "2 errors / 41 turns",
    errors_not_measurable: false,
    context_used: null,
    context_window: null,
    cumulative: { input: 12_345, output: 6_789, cache_read: 100, cache_write: 200, reasoning: 42 },
    session_cost_usd: 0.69,
    priced_model: "opus",
    quota_forecast: "not burning",
    stuck: null,
  },
  "sess-live-1": {
    tokens_bars: [420, 510, 380, 600, 540],
    avg_tokens_per_min: 490,
    error_label: "2 errors / 41 turns",
    errors_not_measurable: false,
    context_used: null,
    context_window: null,
    cumulative: { input: 12_345, output: 6_789, cache_read: 100, cache_write: 200, reasoning: 42 },
    session_cost_usd: 0.69,
    priced_model: "opus",
    quota_forecast: "not burning",
    stuck: null,
  },
};

// AJ-E410 mock trees: a 3-level live tree (the §7.3 wireframe shape)
// plus a 2-orphan section — one fresh, one already `gone`.
const MOCK_PROCESSES: Record<string, ProcessesInfo> = {
  "sess-8": {
    rows: [
      { pid: 41201, parent_pid: 40100, name: "claude", cpu_pct: 11.0, rss_bytes: 890_000_000, age_ms: 42 * 60_000, child_count: 2, depth: 0 },
      { pid: 41233, parent_pid: 41201, name: "node (mcp)", cpu_pct: 1.0, rss_bytes: 96_000_000, age_ms: 40 * 60_000, child_count: 1, depth: 1 },
      { pid: 41302, parent_pid: 41233, name: "rg (search)", cpu_pct: 4.0, rss_bytes: 120_000_000, age_ms: 12_000, child_count: 0, depth: 2 },
    ],
    spark: Array.from({ length: 24 }, (_, i) => ({
      at_ms: Date.now() - (24 - i) * 2_000,
      cpu_pct: 6 + ((i * 7) % 9),
      rss_bytes: 980_000_000 + ((i * 13) % 5) * 20_000_000,
    })),
    orphans: [
      { session_id: "sess-8", pid: 39871, name: "node dev-server", rss_bytes: 210_000_000, parent_label: "claude", age_since_parent_exit_ms: 18 * 60_000, stop_state: null },
      { session_id: "sess-8", pid: 39890, name: "vite", rss_bytes: 88_000_000, parent_label: "claude", age_since_parent_exit_ms: 18 * 60_000, stop_state: "gone" },
    ],
    unavailable_reason: null,
  },
  "sess-2": {
    rows: [
      { pid: 51001, parent_pid: 50900, name: "claude", cpu_pct: 3.0, rss_bytes: 240_000_000, age_ms: 9 * 60_000, child_count: 0, depth: 0 },
    ],
    spark: Array.from({ length: 24 }, (_, i) => ({
      at_ms: Date.now() - (24 - i) * 2_000,
      cpu_pct: 2 + ((i * 3) % 4),
      rss_bytes: 240_000_000,
    })),
    orphans: [],
    unavailable_reason: null,
  },
};

const MOCK_STATS: StatsInfo =
  seeded.stats ?? {
    accounts: 2,
    profiles: 3,
    active_sessions: 0,
    ledger_degraded: false,
  };

// Mock rows mirror the AJ-E401 read-model fixtures: one full row, one
// quota-less row, one model-less/terminal row — nulls where the backend
// reports absence, never zeros or guessed names.
const MOCK_SESSIONS: InstanceInfo[] = seeded.sessions ?? [
  {
    id: "sess-live-1",
    adopted: false,
    launch_recipe_id: "p-1",
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: Some("opus"),
    launch_model: "opus",
    running_model: null,
    model_source: "opus" === null ? "unknown" : "launch",
    status: "running",
    quota: Some({
      used_percentage: 42.5,
      resets_at_ms: Date.now() + 3_600_000,
      confidence: "computed",
    }),
    tokens: Some({ input: 12_345, output: 6_789, cache_read: 100, cache_write: 200, reasoning: 42 }),
    cpu_pct: Some(3.5),
    rss_bytes: Some(123_456),
    child_count: Some(2),
    subtask_count: null,
    git_branch: Some("feat/read-model"),
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: Date.now() - 1_200_000,
    last_activity_ms: Date.now() - 30_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/ai-juggler",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-ai-juggler",
    conversation_id: "822e4125-e07d-4866-9954-ec9295efb079",
  },
  {
    id: "sess-done-1",
    adopted: false,
    launch_recipe_id: "p-3",
    provider: "claude-code",
    account_id: "claude-personal",
    account_label: "Personal",
    model: null,
    launch_model: null,
    running_model: null,
    model_source: null === null ? "unknown" : "launch",
    status: "finished",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: Some(0),
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: Date.now() - 86_400_000,
    last_activity_ms: Date.now() - 80_000_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/ai-juggler",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-payments",
    conversation_id: null,
  },
  {
    id: "sess-done-2",
    adopted: false,
    launch_recipe_id: "p-2",
    provider: "claude-code",
    account_id: "claude-work",
    account_label: "Work Max",
    model: Some("sonnet"),
    launch_model: "sonnet",
    running_model: null,
    model_source: "sonnet" === null ? "unknown" : "launch",
    status: "finished",
    quota: Some({ used_percentage: 100, resets_at_ms: Date.now() + 7_200_000, confidence: "measured" }),
    tokens: Some({ input: 98_000, output: 51_000, cache_read: 9_000, cache_write: 4_000, reasoning: 800 }),
    cpu_pct: null,
    rss_bytes: null,
    child_count: Some(0),
    subtask_count: null,
    git_branch: Some("main"),
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: Date.now() - 43_200_000,
    last_activity_ms: Date.now() - 600_000,
    machine_id: "this-mac",
    cwd: "/Users/dev/ai-juggler",
    kind: "terminal",
    isolation_label: null,
    project_id: "proj-payments",
    conversation_id: "c3f10b52-9a41-4f0e-8f2a-7717db9c04aa",
  },
];

const MOCK_PROJECTS: ProjectInfo[] = seeded.projects ?? [
  {
    id: "proj-ai-juggler",
    root: "/Users/you/Dev/ai-juggler",
    name: "ai-juggler",
    last_activity_ms: Date.now() - 60_000,
    active_instance: MOCK_SESSIONS[0],
    accounts_used: ["Work Max"],
    instance_count: 1,
    history_sessions: 0,
    missing: false,
  },
  {
    id: "proj-payments",
    root: "/Users/you/Dev/payments-api",
    name: "payments-api",
    last_activity_ms: Date.now() - 3_600_000,
    active_instance: null,
    accounts_used: ["Personal", "Work Max"],
    instance_count: 2,
    history_sessions: 14, missing: false,
  },
];

function Some<T>(value: T): T | null {
  return value;
}

function None<T>(): T | null {
  return null;
}

// --- typed calls -----------------------------------------------------------

export async function fetchAppInfo(): Promise<AppInfo | null> {
  if (!hasBackend) {
    // Gallery seam: a Nightly or Dev window (D-23).
    const seeded = (window as unknown as { __AIJ_APP_INFO__?: AppInfo }).__AIJ_APP_INFO__;
    return seeded ?? { version: "0.0.0-preview", name: "AI Juggler", edition: "main" };
  }
  try {
    return await invoke<AppInfo>("app_info");
  } catch {
    return null;
  }
}

export async function providersList(): Promise<ProviderInfo[]> {
  if (!hasBackend) {
    return [{ id: "claude-code", name: "Claude Code" }];
  }
  return invoke<ProviderInfo[]>("providers_list");
}

/** @deprecated legacy alias (AJ-E107) — use providersList. */
export const harnessesList = providersList;

export async function accountsList(): Promise<AccountInfo[]> {
  if (!hasBackend) {
    return seeded.accounts ?? MOCK_ACCOUNTS;
  }
  return invoke<AccountInfo[]>("accounts_list");
}

export async function createAccount(
  harness: string,
  label: string,
  plan: string | null,
): Promise<AccountInfo> {
  if (!hasBackend) {
    return {
      id: `claude-${label.toLowerCase().replace(/\s+/g, "-")}`,
      provider: harness,
      label,
      plan,
      login_state: "logged_out",
      cleanup_opt_in: false,
      renewal_date: null,
      preview_opt_in: false,
    };
  }
  return invoke<AccountInfo>("account_create", { harness, label, plan });
}

/**
 * Scans the machine for existing harness setups the onboarding wizard can
 * import (metadata-only). Mock: a plausible "found your default install"
 * result so the wizard is reviewable in a plain browser.
 */
export async function environmentScan(): Promise<DiscoveredInfo[]> {
  if (!hasBackend) {
    return [
      {
        harness: "claude-code",
        path: "/Users/you/.claude",
        origin: "vendor-default",
        login: "logged_in",
        bytes: 48_000_000,
        last_used_secs: Math.floor(Date.now() / 1000) - 86_400,
        managed: false,
        in_use: [],
        inference: {
          purpose: "Main",
          purpose_confidence: "guessed",
          login: "signed-in",
          login_confidence: "found",
          activity: "active",
          activity_confidence: "found",
          activity_words: "Active this week",
          keychain_bound: false,
        },
        signals: ["vendor-default"],
        notes: [
          "Claude Code's standard location — what a plain `claude` command uses.",
          "Signed in — importing keeps your settings and session history.",
        ],
      },
      {
        harness: "claude-code",
        path: "/Users/you/.claude-side",
        origin: "home-pattern",
        login: "logged_out",
        bytes: 1_200_000,
        last_used_secs: Math.floor(Date.now() / 1000) - 2_592_000,
        managed: false,
        in_use: [],
        inference: {
          purpose: "Personal",
          purpose_confidence: "likely",
          login: "not-signed-in",
          login_confidence: "found",
          activity: "dormant",
          activity_confidence: "found",
          activity_words: "Dormant 30 d",
          keychain_bound: false,
        },
        signals: ["home-pattern"],
        notes: [
          "A separate Claude configuration — created manually, by another tool, or by an AI assistant.",
        ],
      },
    ];
  }
  return invoke<DiscoveredInfo[]>("environment_scan");
}

/** Imports a discovered setup as a managed account (byte copy; source untouched). */
export async function accountImport(
  harness: string,
  sourcePath: string,
  label: string,
  plan: string | null,
  origin: string | null = null,
  degraded = false,
): Promise<ImportResult> {
  if (!hasBackend) {
    // AJ-E304 test seam: substrings of source paths that reject with a
    // coded error, so the partial-failure accept flow (one item fails,
    // the rest complete) is reviewable in a plain browser.
    // AJ-E307 test seam: reject with the coded duplicate error.
    const duplicateLabel = (
      window as unknown as { __AIJ_IMPORT_DUPLICATE_LABEL__?: string }
    ).__AIJ_IMPORT_DUPLICATE_LABEL__;
    if (duplicateLabel !== undefined) {
      throw {
        code: "already_imported",
        retryable: false,
        user_message: `Already imported as «${duplicateLabel}» — open it from Accounts instead of importing again`,
      };
    }
    // AJ-E1022 test seam: reject with a given error (a moved folder…).
    const forced = (window as unknown as { __AIJ_IMPORT_ERROR__?: unknown }).__AIJ_IMPORT_ERROR__;
    if (forced !== undefined) throw forced;
    const failPaths =
      (window as unknown as { __AIJ_IMPORT_FAIL_PATHS__?: string[] })
        .__AIJ_IMPORT_FAIL_PATHS__ ?? [];
    for (const needle of failPaths) {
      if (sourcePath.includes(needle)) {
        throw {
          code: "io",
          retryable: true,
          user_message: "Disk full — this one couldn't be copied.",
        };
      }
    }
    return {
      account: {
        id: `claude-${label.toLowerCase().replace(/\s+/g, "-")}`,
        provider: harness,
        label,
        plan,
        login_state: "logged_out",
        cleanup_opt_in: false,
        renewal_date: null,
        preview_opt_in: false,
      },
      files_copied: degraded ? 8 : 128,
      bytes_copied: degraded ? 6_000_000 : 46_000_000,
      folders: degraded
        ? [{ name: "settings.json", bytes: 60 }, { name: ".credentials.json", bytes: 40 }]
        : [
            { name: "projects/", bytes: 40_000_000 },
            { name: "settings.json", bytes: 60 },
            { name: ".credentials.json", bytes: 40 },
          ],
      degraded,
      skipped: degraded
        ? ["projects/ (37.3 GiB history — skipped at your choice)", "shell-snapshots/ (rebuildable cache)"]
        : ["shell-snapshots/ (rebuildable cache)", "statsig/ (rebuildable cache)"],
      login: "logged_out",
      login_note:
        "Your Claude login lives in the macOS Keychain, locked to the original folder — it can't (and shouldn't) be moved. Sign in once inside the imported account; it takes a few seconds.",
    };
  }
  return invoke<ImportResult>("account_import", {
    harness,
    sourcePath,
    label,
    plan,
    origin,
    degraded,
  });
}

/** AJ-E307 Req.3: inspect a candidate import source BEFORE any copy
 * (metadata-only): sizes, per-entry breakdown, evidence, login. */
export async function sourceInspect(
  harness: string,
  sourcePath: string,
): Promise<SourceInspectInfo> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_SOURCE_INSPECT__?: SourceInspectInfo })
        .__AIJ_SOURCE_INSPECT__ ?? {
        total_bytes: 46_000_000,
        file_count: 128,
        folders: [
          { name: "projects/", bytes: 40_000_000 },
          { name: "settings.json", bytes: 60 },
          { name: ".credentials.json", bytes: 40 },
        ],
        truncated: false,
        ceiling_bytes: 2_147_483_648,
        is_config: true,
        already_managed: false,
        login: "logged_in",
        keychain_bound: true,
      }
    );
  }
  return invoke<SourceInspectInfo>("source_inspect", { harness, sourcePath });
}

/** Opens the vendor's own web sign-in page in the system browser. */
export async function loginOpenWeb(harness: string): Promise<void> {
  return invoke<void>("login_open_web", { harness });
}

export async function accountLoginCommand(accountId: string): Promise<string> {
  if (!hasBackend) {
    return `CLAUDE_CONFIG_DIR='~/Library/Application Support/AI Juggler/accounts/claude-code/${accountId}' claude`;
  }
  return invoke<string>("account_login_command", { accountId });
}

export async function accountLoginState(accountId: string): Promise<string> {
  if (!hasBackend) {
    // AJ-E308 test seam: specs inject the probed state.
    return (
      (window as unknown as { __AIJ_LOGIN_STATE__?: string }).__AIJ_LOGIN_STATE__ ??
      "logged_out"
    );
  }
  return invoke<string>("account_login_state", { accountId });
}

export async function launchRecipesList(): Promise<LaunchRecipeInfo[]> {
  if (!hasBackend) {
    // AJ-E305 test seam: visual/flow specs inject the recipe set.
    return (
      (window as unknown as { __AIJ_RECIPES__?: LaunchRecipeInfo[] }).__AIJ_RECIPES__ ??
      MOCK_PROFILES
    );
  }
  return invoke<LaunchRecipeInfo[]>("launch_recipes_list");
}

/** @deprecated legacy alias (AJ-E107) — use launchRecipesList. */
export const profilesList = launchRecipesList;

export async function createProfile(
  accountId: string,
  name: string,
  model: string | null,
  defaultCwd: string | null,
): Promise<LaunchRecipeInfo> {
  return invoke<LaunchRecipeInfo>("profile_create", {
    accountId,
    name,
    model,
    defaultCwd,
  });
}

export async function deleteProfile(profileId: string): Promise<void> {
  await invoke("profile_delete", { profileId });
}

/** AJ-E1001: who a Launch setting's Account is signed in as — checked
 * before anything launches (read-only). */
export async function identityCheck(recipeId: string): Promise<IdentityCheckInfo> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_IDENTITY_CHECK__?: Record<string, IdentityCheckInfo> };
    const seeded = seams.__AIJ_IDENTITY_CHECK__?.[recipeId];
    if (seeded) return seeded;
    return {
      state: "verified",
      provider: "claude-code",
      account_id: "",
      account_label: "",
      model: null,
      expected: null,
      detected: null,
      reason: null,
      alternatives: [],
    };
  }
  return invoke<IdentityCheckInfo>("identity_check", { recipeId });
}

/** What the user chose in the identity-mismatch dialog. */
export type IdentityDecision = { kind: "switch"; recipeId: string } | { kind: "recheck" } | { kind: "cancel" };

/** The event the app's identity dialog listens for (one dialog serves
 * every launch entry point). */
export const IDENTITY_MISMATCH_EVENT = "aij:identity-mismatch";

export type IdentityMismatchRequest = {
  check: IdentityCheckInfo;
  resolve: (decision: IdentityDecision) => void;
};

/** Asks the user (through the app's dialog) what to do about a mismatch. */
function askIdentityDecision(check: IdentityCheckInfo): Promise<IdentityDecision> {
  return new Promise((resolve) => {
    const detail: IdentityMismatchRequest = { check, resolve };
    const delivered = window.dispatchEvent(new CustomEvent(IDENTITY_MISMATCH_EVENT, { detail, cancelable: true }));
    // No dialog mounted (a test, a detached view): never continue silently.
    if (delivered) {
      window.setTimeout(() => {
        if (!(detail as { claimed?: boolean }).claimed) resolve({ kind: "cancel" });
      }, 0);
    }
  });
}

/** AJ-E1002: the worktree offer for launching into `cwd` (`null` when
 * nobody else works there). */
export async function isolationOffer(cwd: string): Promise<IsolationOfferInfo | null> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_ISOLATION_OFFER__?: IsolationOfferInfo | null };
    return seams.__AIJ_ISOLATION_OFFER__ ?? null;
  }
  return invoke<IsolationOfferInfo | null>("isolation_offer", { cwd });
}

/** AJ-E1002: creates the worktree the user confirmed; returns its path. */
export async function worktreeCreate(cwd: string, path: string, branch: string): Promise<string> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_LAUNCH_CALLS__?: string[] };
    seams.__AIJ_LAUNCH_CALLS__ = [...(seams.__AIJ_LAUNCH_CALLS__ ?? []), `worktree:${branch}:${path}`];
    return path;
  }
  return invoke<string>("worktree_create", { cwd, path, branch });
}

/** AJ-E1002: what removing a worktree would discard. */
export async function worktreeRemovalPreview(path: string): Promise<WorktreeRemovalInfo> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_WORKTREE_REMOVAL__?: WorktreeRemovalInfo };
    if (seams.__AIJ_WORKTREE_REMOVAL__) return seams.__AIJ_WORKTREE_REMOVAL__;
    throw new Error("no worktree here");
  }
  return invoke<WorktreeRemovalInfo>("worktree_removal_preview", { path });
}

/** AJ-E1002: removes a worktree AI Juggler created (typed confirmation
 * when uncommitted work would be discarded). */
export async function worktreeRemove(path: string, confirm: string | null): Promise<void> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_LAUNCH_CALLS__?: string[] };
    seams.__AIJ_LAUNCH_CALLS__ = [...(seams.__AIJ_LAUNCH_CALLS__ ?? []), `worktree-remove:${path}:${confirm ?? ""}`];
    return;
  }
  return invoke<void>("worktree_remove", { path, confirm });
}

/** AJ-E1002: an Instance's git status, read now (on focus). */
export async function gitStatusRefresh(sessionId: string): Promise<GitStatusInfo | null> {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_GIT_STATUS__?: Record<string, GitStatusInfo> };
    return seams.__AIJ_GIT_STATUS__?.[sessionId] ?? null;
  }
  return invoke<GitStatusInfo | null>("git_status_refresh", { sessionId });
}

/** What the user chose in the isolation offer. */
export type IsolationDecision =
  | { kind: "worktree"; path: string; branch: string }
  | { kind: "shared" }
  | { kind: "cancel" };

export const ISOLATION_OFFER_EVENT = "aij:isolation-offer";

export type IsolationOfferRequest = {
  offer: IsolationOfferInfo;
  resolve: (decision: IsolationDecision) => void;
};

function askIsolation(offer: IsolationOfferInfo): Promise<IsolationDecision> {
  return new Promise((resolve) => {
    const detail: IsolationOfferRequest & { claimed?: boolean } = { offer, resolve };
    window.dispatchEvent(new CustomEvent(ISOLATION_OFFER_EVENT, { detail }));
    // No dialog mounted: launch nothing rather than share silently.
    window.setTimeout(() => {
      if (!detail.claimed) resolve({ kind: "cancel" });
    }, 0);
  });
}

/** Launches a session — after the AJ-E1001 identity check. A mismatch
 * pauses for the user (switch selection / check again / cancel); there
 * is no "continue anyway", and the backend refuses one regardless. */
export async function launchSession(
  profileId: string,
  cwd: string | null,
): Promise<{ session_id: string; pid: number } | null> {
  let recipe = profileId;
  for (;;) {
    const check = await identityCheck(recipe);
    if (check.state !== "mismatch") break;
    const decision = await askIdentityDecision(check);
    if (decision.kind === "cancel") {
      throw new Error(
        `Not launched — «${check.account_label}» is signed in as ${check.detected ?? "someone else"}, not ${check.expected ?? "its account"}.`,
      );
    }
    if (decision.kind === "switch") recipe = decision.recipeId;
  }
  // AJ-E1002: another Instance already works here — offer its own
  // worktree (the default), created only once the user confirms.
  let directory = cwd;
  if (directory) {
    const offer = await isolationOffer(directory);
    if (offer) {
      const decision = await askIsolation(offer);
      if (decision.kind === "cancel") throw new Error("Not launched — cancelled.");
      if (decision.kind === "worktree") directory = await worktreeCreate(directory, decision.path, decision.branch);
    }
  }
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_LAUNCH_CALLS__?: string[] };
    seams.__AIJ_LAUNCH_CALLS__ = [...(seams.__AIJ_LAUNCH_CALLS__ ?? []), recipe];
    return null; // preview mode: nothing really launches
  }
  return invoke<{ session_id: string; pid: number }>("launch_session", {
    profileId: recipe,
    cwd: directory,
  });
}

/** AJ-E1001: forgets an Account's remembered identity (learned again at
 * its next launch, shown as "first seen"). */
export async function accountForgetIdentity(accountId: string): Promise<void> {
  if (!hasBackend) return;
  return invoke<void>("account_forget_identity", { accountId });
}

/** Tracked projects — the juggling anchor (which instance is on each directory). */
export async function projectsList(): Promise<ProjectInfo[]> {
  if (!hasBackend) {
    return MOCK_PROJECTS;
  }
  return invoke<ProjectInfo[]>("projects_list");
}

export async function instancesList(): Promise<InstanceInfo[]> {
  if (!hasBackend) {
    return MOCK_SESSIONS;
  }
  return invoke<InstanceInfo[]>("instances_list");
}

/** @deprecated legacy alias (AJ-E107) — use instancesList. */
export const sessionsList = instancesList;

/** AJ-E413: launch the Provider's desktop app under an Account. */
export async function desktopLaunch(accountId: string): Promise<DesktopLaunchInfo> {
  if (!hasBackend) {
    return {
      session_id: "sess-desktop-1",
      pid: 42_042,
      bundle_id: "dev.aijuggler.fake-desktop",
      isolation_label: "Single-account app — Juggler tracks, cannot isolate",
    };
  }
  return invoke<DesktopLaunchInfo>("desktop_launch", { accountId });
}

/** AJ-E414: run the focus tiers for one desktop Instance — activate,
 * match the recorded window identity, clear AXMinimized, AXRaise. The
 * returned outcome says exactly which tier ran (never overstated). */
export async function desktopFocus(sessionId: string): Promise<FocusOutcomeInfo> {
  if (!hasBackend) {
    // Harness seam (AJ-E802): captures may pin the outcome they render.
    const w = window as unknown as { __AIJ_FOCUS_OUTCOME__?: FocusOutcomeInfo };
    return (
      w.__AIJ_FOCUS_OUTCOME__ ?? {
        outcome: "Focused window",
        permission_missing: false,
        window_title: "notes-a",
      }
    );
  }
  return invoke<FocusOutcomeInfo>("desktop_focus", { sessionId });
}


/** AJ-E415: the deterministic mock preview frame (a rendered fake-app
 * window; harness captures pin phases via __AIJ_PREVIEW_PHASE__). */
const MOCK_PREVIEW_PNG_B64 = "iVBORw0KGgoAAAANSUhEUgAAAUAAAADICAIAAAAWZq/8AAACA0lEQVR42u3cMQqDQBCGUQ8hGEhY0SZ4hRSpQ4ocQSxzBk/vBazE5ncfvBMsfNXOTPN6f4FQjScAAQMCBgQMAgYEDAgYEDAIGBAwIGAQMCBgQMCAgEHAgIABAQMCBgEDAgYOB1yGCQglYEgOuO0KEErAIGBAwICAQcCAgAEBAwIGAQMCBgQMAgYEDAgYEDBcP+DlvwKhBAwCBgQMCBgEDAgYEDAgYBAwIGBAwFBzwMbBwTYSIGBAwCBgQMCAgAEBg4ABAQMCBgEDiQF/fjMQSsAgYEDAgIBBwICAAQEDewH7CgeTWICAAQGDgAEBAwIGBAwCBgQMCBgEDCQG/OifQCgBg4ABAQMCBgEDAgYEDAgYBAwIGBAw1Bzw7T4CoQQMAgYEDAgYBAwIGBAwIGAQMCBgQMBQc8DGwcE2EiBgQMAgYEDAgIABAYOAAQEDAgYBA+5CA7aRAAGDgAEBAwIGBAwCBgQMCBgEDAgYcBcasI0EAgYEDAgYEDAIGBAwIGAQMCBgQMCAu9BgGwkQMCBgELBXAAEDAgYEDAIGBAwIGBAwuAsN2EYCBAwCBvICbrsChBIwCBgQMCBgEDAgYEDAgIBBwICAAQGDgAEBAwIGBAwCBgQMCBgQMAgYEDAgYBAwIGBAwICAQcCAgAEBAwIGAQMCBgQMAgYEDAgYEDAIGBAwcIoN1C7gkSubCwYAAAAASUVORK5CYII=";

/** AJ-E415: starts the read-only live preview for the selected desktop
 * Instance (requires the Account's opt-in; one at a time). */
export async function previewStart(sessionId: string): Promise<void> {
  if (!hasBackend) {
    return;
  }
  await invoke("preview_start", { sessionId });
}

/** AJ-E415: stops the live preview (bounded teardown <1 s). */
export async function previewStop(): Promise<void> {
  if (!hasBackend) {
    return;
  }
  await invoke("preview_stop");
}

/** AJ-E415: sets the per-Account live-preview opt-in (the Screen
 * Recording consent moment; off by default). */
export async function accountSetPreviewOptIn(
  accountId: string,
  optIn: boolean,
): Promise<AccountInfo> {
  if (!hasBackend) {
    return {
      ...(MOCK_ACCOUNTS.find((a) => a.id === accountId) ?? MOCK_ACCOUNTS[0]!),
      preview_opt_in: optIn,
    };
  }
  return invoke<AccountInfo>("account_set_preview_opt_in", { accountId, optIn });
}

/** AJ-E415: live frames (filtered to one session by the caller). */
export function onPreviewFrame(
  handler: (frame: PreviewFrameInfo) => void,
): () => void {
  if (!hasBackend) {
    // Harness mode: emit one deterministic frame immediately so the
    // panel (and captures) render a real preview; the phase comes from
    // onPreviewState below.
    handler({
      session_id: "sess-desktop-1",
      png_b64: MOCK_PREVIEW_PNG_B64,
      at_ms: Date.now(),
      seq: 0,
    });
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<PreviewFrameInfo>(EVENTS.previewFrame, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {
      /* backend gone; the panel shows its paused state */
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** AJ-E415: phase changes (live | paused | denied | unsupported). */
export function onPreviewState(
  handler: (state: PreviewStateInfo) => void,
): () => void {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_PREVIEW_PHASE__?: PreviewStateInfo };
    const seeded = w.__AIJ_PREVIEW_PHASE__;
    if (seeded) {
      handler(seeded);
    } else {
      handler({
        session_id: "sess-desktop-1",
        phase: "live",
        text: "",
      });
    }
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<PreviewStateInfo>(EVENTS.previewState, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {
      /* backend gone */
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** Mock app settings (browser preview and tests): kept for the page. */
const appSettingSeams = () =>
  window as unknown as { __AIJ_APP_SETTINGS__?: Record<string, string> };

/** Reads one app-level setting (schema v10 kv); null = never set. */
export async function appSettingGet(key: string): Promise<string | null> {
  if (!hasBackend) {
    const stored = appSettingSeams().__AIJ_APP_SETTINGS__?.[key];
    if (stored !== undefined) return stored === "" ? null : stored;
    return key === "go_to_hotkey" ? "Alt+Space" : null;
  }
  return invoke<string | null>("app_setting_get", { key });
}

/** Writes one app-level setting (empty value = reset to default). */
export async function appSettingSet(key: string, value: string): Promise<void> {
  if (!hasBackend) {
    const seams = appSettingSeams();
    seams.__AIJ_APP_SETTINGS__ = { ...(seams.__AIJ_APP_SETTINGS__ ?? {}), [key]: value };
    return;
  }
  await invoke("app_setting_set", { key, value });
}

/** Settings › Notifications (wireframe 26): the digest's quiet hours and
 * the Accounts left out of it. */
export type { NotificationPrefsInfo };
const notificationSeams = () =>
  window as unknown as { __AIJ_NOTIFICATION_PREFS__?: NotificationPrefsInfo; __AIJ_CLEANUP_DEFAULT__?: boolean };

/** Reads Settings › Notifications. */
export async function notificationPrefsGet(): Promise<NotificationPrefsInfo> {
  if (!hasBackend) {
    return { quiet_start: 22, quiet_end: 7, digest_muted_accounts: [], ...notificationSeams().__AIJ_NOTIFICATION_PREFS__ };
  }
  return invoke<NotificationPrefsInfo>("notification_prefs_get");
}

/** Saves Settings › Notifications (hours 0–23; equal hours = none). */
export async function notificationPrefsSet(prefs: NotificationPrefsInfo): Promise<NotificationPrefsInfo> {
  if (!hasBackend) {
    const saved = {
      ...prefs,
      digest_muted_accounts: [...new Set(prefs.digest_muted_accounts)].sort(),
    };
    notificationSeams().__AIJ_NOTIFICATION_PREFS__ = saved;
    return saved;
  }
  return invoke<NotificationPrefsInfo>("notification_prefs_set", { prefs });
}

/** D-25: what this OS must allow (one entry per thing it makes possible;
 * an empty list means the first-run access step is skipped). */
export type { OsAccessInfo };
const osAccessSeams = () =>
  window as unknown as { __AIJ_OS_ACCESS__?: OsAccessInfo[]; __AIJ_OS_ACCESS_CALLS__?: string[] };
const MOCK_OS_ACCESS: OsAccessInfo[] = [
  {
    id: "focus",
    title: "Jump to a session's own window",
    why: "Brings the right terminal window to the front when you open a session.",
    system_name: "Accessibility and Automation",
    status: "not_asked",
    action: "prompt",
    note: null,
  },
  {
    id: "preview",
    title: "See a live preview of each session",
    why: "Shows a session's window inside AI Juggler, on this Mac only; nothing is recorded or sent.",
    system_name: "Screen Recording",
    status: "not_asked",
    action: "prompt",
    note: "macOS may ask you to reopen AI Juggler after you allow it.",
  },
];

/** What this OS needs, as it stands (never prompts — safe to poll). */
export async function osAccessList(): Promise<OsAccessInfo[]> {
  if (!hasBackend) return (osAccessSeams().__AIJ_OS_ACCESS__ ?? MOCK_OS_ACCESS).map((entry) => ({ ...entry }));
  return invoke<OsAccessInfo[]>("os_access_list");
}

/** Allow: raises the system's prompts for one entry. */
export async function osAccessRequest(id: string): Promise<OsAccessInfo[]> {
  if (!hasBackend) {
    const seams = osAccessSeams();
    seams.__AIJ_OS_ACCESS_CALLS__ = [...(seams.__AIJ_OS_ACCESS_CALLS__ ?? []), `request:${id}`];
    // The preview's user says yes in the system dialog.
    seams.__AIJ_OS_ACCESS__ = (seams.__AIJ_OS_ACCESS__ ?? MOCK_OS_ACCESS).map((entry) =>
      entry.id === id ? { ...entry, status: "allowed", action: "none", note: null } : entry,
    );
    return osAccessList();
  }
  return invoke<OsAccessInfo[]>("os_access_request", { id });
}

/** Opens the settings pane for one entry (a fixed link per id). */
export async function osAccessOpenSettings(id: string): Promise<void> {
  if (!hasBackend) {
    const seams = osAccessSeams();
    seams.__AIJ_OS_ACCESS_CALLS__ = [...(seams.__AIJ_OS_ACCESS_CALLS__ ?? []), `settings:${id}`];
    return;
  }
  await invoke("os_access_open_settings", { id });
}

/** Whether new Accounts start with left-behind dev servers tidied (D-08:
 * off unless the user chooses it). */
export async function cleanupDefaultGet(): Promise<boolean> {
  if (!hasBackend) return notificationSeams().__AIJ_CLEANUP_DEFAULT__ ?? false;
  return invoke<boolean>("cleanup_default_get");
}

/** Sets the cleanup default for new Accounts (existing ones keep theirs). */
export async function cleanupDefaultSet(on: boolean): Promise<boolean> {
  if (!hasBackend) {
    notificationSeams().__AIJ_CLEANUP_DEFAULT__ = on;
    return on;
  }
  return invoke<boolean>("cleanup_default_set", { on });
}

/** Rebinds the global "Go to instance…" hotkey (registers with the OS
 * first; a failure restores the previous binding). */
export async function goToHotkeySet(hotkey: string): Promise<void> {
  if (!hasBackend) {
    return;
  }
  await invoke("go_to_hotkey_set", { hotkey });
}

/** Opens the system Accessibility privacy pane (the explainer's deep
 * link; the target is fixed in Rust — never caller-supplied). */
export async function openAccessibilitySettings(): Promise<void> {
  if (!hasBackend) {
    return;
  }
  await invoke("open_accessibility_settings");
}

/** Fires when the global "Go to instance…" hotkey is pressed anywhere
 * in the system (AJ-E414 Req.4). No-backend: the palette stays
 * reachable in-app via ⌘K. */
export function onGoToHotkey(handler: () => void): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen(EVENTS.goTo, () => handler())
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {
      // backend gone; ⌘K remains
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** AJ-E409: D-07 quality signals for the selected Instance (no score). */
export async function instanceQuality(sessionId: string): Promise<QualitySignalsInfo | null> {
  if (!hasBackend) {
    return MOCK_QUALITY[sessionId] ?? null;
  }
  return invoke<QualitySignalsInfo>("instance_quality", { sessionId });
}

/**
 * AJ-E410: the Processes tab payload — live tree rows, the 2 s spark
 * ring, and the confirmed orphans for one Instance.
 */
export async function instanceProcesses(sessionId: string): Promise<ProcessesInfo | null> {
  if (!hasBackend) {
    return MOCK_PROCESSES[sessionId] ?? null;
  }
  return invoke<ProcessesInfo>("instance_processes", { sessionId });
}

/** AJ-E505: one port-inventory row (classification is the key). */
export interface PortRowInfo {
  port: number;
  pid: number;
  class: string;
  session: string | null;
  provider: string | null;
  account: string | null;
  project: string | null;
  listener_age_ms: number | null;
  name: string | null;
  hint: string | null;
  killable: boolean;
}

/** AJ-E505: the inventory response (honest probe note + hog facts). */
export interface PortInventoryInfo {
  rows: PortRowInfo[];
  note: string | null;
  hogs: Array<{
    pid: number;
    session: string;
    provider: string;
    rss_bytes: number;
    cpu_pct: number | null;
    rule: string;
  }>;
}

/** AJ-E505: the pre-launch collision warning (copy-exact message). */
export interface PortWarningInfo {
  port: number;
  pid: number;
  instance: string;
  age_ms: number;
  message: string;
}

/** AJ-E509: one cleanup-audit history row (outcome vocabulary). */
export interface AuditHistoryRow {
  id: number;
  at_ms: number;
  actor: string;
  automatic: boolean;
  pid: number;
  cmdline_trunc: string;
  session_id: string;
  provider: string;
  account_id: string | null;
  machine_id: string;
  outcome: string;
  ports_before: string | null;
  ports_after: string | null;
}

/** AJ-E509: the audit export (JSONL text; first line = redaction note). */
export async function cleanupAuditExport(): Promise<string> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_AUDIT_EXPORT__?: string }).__AIJ_AUDIT_EXPORT__
      ?? JSON.stringify({ note: "cleanup audit export — command lines are hashed and truncated; full cmdlines are never stored" });
  }
  return invoke<string>("cleanup_audit_export");
}

/** AJ-E506: the read-only Cleanup Centre DTOs (zero kill fields). */
export type { CleanupRowInfo, EvidenceChainInfo, VerificationBadgeInfo } from "../components/ProcessRow";
export type { CleanupGroupInfo } from "../screens/CleanupCentre";
import type { CleanupInventoryData } from "../screens/CleanupCentre";

/** AJ-E512: one passive Cleanup Centre alert (hog / idle nudge). */
export interface CleanupAlertData {
  kind: "hog" | "idle" | string;
  text: string;
  pid: number;
  session: string;
}

/** AJ-E512: the passive alert set (hog alerts + idle-orphan nudges). */
export interface CleanupAlertsData {
  alerts: CleanupAlertData[];
}

/** AJ-E512: reads the passive alerts (hog alerts name the top helper;
 * idle nudges fire once per orphan). Purely passive — no alert
 * carries a signal path; the hog alert routes to the staged sheet. */
/** AJ-E609: one cost rollup row (session × model × day). */
export interface CostRollupRow {
  key: string;
  model?: string;
  day?: string;
  tokens: { input: number; output: number; cache_read: number; cache_write: number; reasoning: number };
  usd: { usd?: number; label: string };
  credits?: number;
  confidence: "Measured" | "Computed" | "Estimate";
}

/** AJ-E609: the dual subscription view ("≈$X at API prices · you pay flat"). */
export interface SubscriptionViewData {
  api_equivalent_usd?: number;
  flat_fee_usd?: number;
  value_ratio?: number;
}

/** AJ-E609: the cost matrix payload (the UI and the CSV read the same rows). */
export interface CostRollupsData {
  rows: CostRollupRow[];
  subscription: SubscriptionViewData;
  pricing_stale: boolean;
  pricing_last_synced_ms?: number;
  total_usd?: number;
  scope: string;
}

/** AJ-E611: one ranked (or filtered) account row. */
export interface RankedRowData {
  account: string;
  score?: number;
  filtered_reason?: string;
  top_factors: string[];
  factors: [string, number][];
  headroom_unknown: boolean;
  confidence: string;
  tte_min?: number;
  suggested: boolean;
  /** AJ-E306 Req.7: the launch admission verdict (SPEC §5.2) —
   * `ok|tight|no`; undefined when headroom or burn is unknown
   * (renders "headroom unknown", never a fabricated number). Warns,
   * never blocks. */
  admission?: "ok" | "tight" | "no";
}

/** AJ-E611: the explainable ranking — suggest-only, the user clicks. */
export async function rankingRows(project?: string): Promise<RankedRowData[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_RANKING__?: RankedRowData[] })
      .__AIJ_RANKING__ ?? [];
  }
  return invoke<RankedRowData[]>("ranking_rows", { project });
}

/** AJ-E611: one emitted alert (the notification region's payload).
 * `digest` lines are the hourly batch — visually quieter than window
 * alerts (a renewal item never reads as "switch now"). */
export interface AlertEventData {
  kind: "tray-amber" | "actionable" | "exhausted" | "reset-reached" | "digest";
  account: string;
  text: string;
  at_ms: number;
}

/** AJ-E611: one alert evaluation pass (at-most-once by the machines).
 * The webview's wall-clock local hour rides along — the quiet-hours
 * gate for the digest flush (the backend carries no tz database). */
export async function alertsTick(): Promise<AlertEventData[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_ALERTS__?: AlertEventData[] })
      .__AIJ_ALERTS__ ?? [];
  }
  return invoke<AlertEventData[]>("alerts_tick", { localHour: new Date().getHours() });
}

/** AJ-E611: one reset-queue row. */
export interface ResetQueueRow {
  id: string;
  session: string;
  account: string;
  resume_at_ms: number;
  state: string;
}

/** AJ-E611: the queue rows (countdown / ready / stale explained). */
export async function resetQueueList(): Promise<ResetQueueRow[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_RESET_QUEUE__?: ResetQueueRow[] })
      .__AIJ_RESET_QUEUE__ ?? [];
  }
  return invoke<ResetQueueRow[]>("reset_queue_list");
}

/** AJ-E611: queue a resume at the reset (passive timer; the resume
 * itself is always an explicit click). */
export async function resetQueueQueue(
  session: string,
  account: string,
  resumeAtMs: number,
): Promise<void> {
  if (!hasBackend) return;
  await invoke("reset_queue_queue", { session, account, resumeAtMs });
}

/** AJ-E611: dismiss a queued resume. */
export async function resetQueueDismiss(id: string): Promise<void> {
  if (!hasBackend) return;
  await invoke("reset_queue_dismiss", { id });
}

/** AJ-E610: one window clock's typed rendered state. */
export interface WindowDisplay {
  state: "unknown" | "learning" | "tracking" | "stale" | "exhausted";
  percentage?: number;
  badge?: string;
  stale_note_ms?: number;
  resets_at_ms?: number;
  burned_tokens: number;
  burned_credits?: number;
  headroom_tokens?: number | null;
}

/** AJ-E610: the subscription clock's typed state (calendar line, D-12). */
export interface RenewalDisplay {
  state: "none" | "set" | "approaching" | "overdue";
  label: string;
  entered_by_user: boolean;
}

/** AJ-E610: the forecast pill data. */
export interface ForecastDisplay {
  state: "learning" | "wont-exhaust" | "exhausts" | "no-headroom";
  tte_min?: number;
  range_min?: number;
  range_max?: number;
  badge: string;
}

/** AJ-E610: the two-clock display for one account. */
export interface QuotaDisplayData {
  account: string;
  five_hour?: WindowDisplay;
  five_hour_omitted_reason?: string;
  weekly?: WindowDisplay;
  weekly_omitted_reason?: string;
  renewal: RenewalDisplay;
  forecast: ForecastDisplay;
  unavailable_reason?: string;
}

/** AJ-E306: one window's token headroom (cap − burned) — present
 * ONLY when the cap is known; the admission verdict's input. */
export interface WindowHeadroom {
  headroom_tokens?: number | null;
}

/** AJ-E610: one account's two clocks (engine-evaluated). */
export async function quotaDisplay(account: string): Promise<QuotaDisplayData> {
  if (!hasBackend) {
    const all = await quotaDisplayAll();
    return (
      all.find((entry) => entry.account === account) ?? {
        account,
        renewal: { state: "none", label: "Renewal: unknown — add date", entered_by_user: false },
        forecast: { state: "learning", badge: "estimated" },
      }
    );
  }
  return invoke<QuotaDisplayData>("quota_display", { account });
}

/** AJ-E610: every account's two clocks (rail/row source; one query). */
export async function quotaDisplayAll(): Promise<QuotaDisplayData[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_QUOTA_DISPLAY__?: QuotaDisplayData[] })
      .__AIJ_QUOTA_DISPLAY__ ?? [];
  }
  return invoke<QuotaDisplayData[]>("quota_display_all");
}

/** AJ-E610: set/clear the user-entered flat fee (dual view ratio). */
export async function accountSetFee(account: string, feeUsd: number | null): Promise<void> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_SET_FEE__?: (a: string, f: number | null) => void };
    w.__AIJ_SET_FEE__?.(account, feeUsd);
    return;
  }
  await invoke("account_set_fee", { account, feeUsd });
}

/** AJ-E609: the session cost matrix + dual subscription view. */
export async function costRollups(account?: string): Promise<CostRollupsData> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_COST_ROLLUPS__?: CostRollupsData })
        .__AIJ_COST_ROLLUPS__ ?? {
        rows: [],
        subscription: {},
        pricing_stale: false,
        scope: account ?? "ALL",
      }
    );
  }
  return invoke<CostRollupsData>("cost_rollups", { account });
}

/** AJ-E609: stream the CSV export; resolves to the written file path. */
export async function costCsvExport(account?: string): Promise<string> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_COST_CSV__?: string }).__AIJ_COST_CSV__ ?? "";
  }
  return invoke<string>("cost_csv_export", { account });
}

export async function cleanupAlerts(): Promise<CleanupAlertsData> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_CLEANUP_ALERTS__?: CleanupAlertsData })
        .__AIJ_CLEANUP_ALERTS__ ?? { alerts: [] }
    );
  }
  return invoke<CleanupAlertsData>("cleanup_alerts");
}

/** AJ-E508: the automation status (banners + pause + Tier-2 notices). */
export interface CleanupStatusData {
  paused: boolean;
  suspended: boolean;
  suspended_until_ms: number | null;
  tier2_pending: {
    pid: number;
    session: string;
    cmdline_trunc: string;
    deadline_ms: number;
  }[];
}

/** AJ-E508: per-Account tier opt-ins (two separate toggles). */
export interface CleanupTierSettingsData {
  tier1_opt_in: boolean;
  tier2_opt_in: boolean;
}

/** AJ-E510: the one-time recovery summary. */
export interface RecoverySummaryData {
  summary: string;
  adopted: number;
  left_behind: number;
  unverifiable: number;
}

/** Reads the unconsumed recovery summary, if pending (shown once). */
export async function recoverySummaryGet(): Promise<RecoverySummaryData | null> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_RECOVERY__?: RecoverySummaryData }).__AIJ_RECOVERY__
      ?? null;
  }
  return invoke<RecoverySummaryData | null>("recovery_summary_get");
}

/** Dismisses the recovery summary (the only way it goes away). */
export async function recoverySummaryDismiss(): Promise<void> {
  if (!hasBackend) {
    return Promise.resolve();
  }
  return invoke("recovery_summary_dismiss");
}

/** Reads the automation status (Tier 0's fallback — never signals). */
export async function cleanupStatus(): Promise<CleanupStatusData> {
  if (!hasBackend) {
    // Test seam: the status read fails (Settings shows the error and a retry).
    const failure = (window as unknown as { __AIJ_CLEANUP_STATUS_ERROR__?: unknown }).__AIJ_CLEANUP_STATUS_ERROR__;
    if (failure !== undefined) throw failure;
    return (
      (window as unknown as { __AIJ_CLEANUP_STATUS__?: CleanupStatusData }).__AIJ_CLEANUP_STATUS__
      ?? { paused: false, suspended: false, suspended_until_ms: null, tier2_pending: [] }
    );
  }
  return invoke<CleanupStatusData>("cleanup_status");
}

/** Sets the global `Pause automatic cleanup` kill-switch. */
export async function cleanupPauseSet(paused: boolean): Promise<CleanupStatusData> {
  if (!hasBackend) {
    return {
      paused,
      suspended: false,
      suspended_until_ms: null,
      tier2_pending: [],
    };
  }
  return invoke<CleanupStatusData>("cleanup_pause_set", { paused });
}

/** The Tier-2 notice decision: `keep` = Undo / Keep running. */
export async function cleanupTier2Decide(
  pid: number,
  keep: boolean,
): Promise<CleanupStatusData> {
  if (!hasBackend) {
    return { paused: false, suspended: false, suspended_until_ms: null, tier2_pending: [] };
  }
  return invoke<CleanupStatusData>("cleanup_tier2_decide", { pid, keep });
}

/** Re-confirms automation after the 24 h crash suspension. */
export async function cleanupResumeAfterCrash(): Promise<CleanupStatusData> {
  if (!hasBackend) {
    // Test seam: Re-confirm fails (the banner stays and says why).
    const failure = (window as unknown as { __AIJ_CLEANUP_RESUME_ERROR__?: unknown }).__AIJ_CLEANUP_RESUME_ERROR__;
    if (failure !== undefined) throw failure;
    return { paused: false, suspended: false, suspended_until_ms: null, tier2_pending: [] };
  }
  return invoke<CleanupStatusData>("cleanup_resume_after_crash");
}

/** Reads one Account's two tier opt-ins (absent rows = both off). */
export async function cleanupTiersGet(accountId: string): Promise<CleanupTierSettingsData> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_CLEANUP_TIERS__?: CleanupTierSettingsData })
        .__AIJ_CLEANUP_TIERS__ ?? { tier1_opt_in: false, tier2_opt_in: false }
    );
  }
  return invoke<CleanupTierSettingsData>("cleanup_tiers_get", { accountId });
}

/** Sets one Account's two SEPARATE toggles. */
export async function cleanupTiersSet(
  accountId: string,
  tier1OptIn: boolean,
  tier2OptIn: boolean,
): Promise<CleanupTierSettingsData> {
  if (!hasBackend) {
    return { tier1_opt_in: tier1OptIn, tier2_opt_in: tier2OptIn };
  }
  return invoke<CleanupTierSettingsData>("cleanup_tiers_set", {
    accountId,
    tier1OptIn,
    tier2OptIn,
  });
}
import type {
  KillExecuteData,
  KillScope,
  KillVerifyData,
} from "../components/KillSheet";

/** AJ-E506: the read-only inventory (Tier 0). */
export async function cleanupInventory(
  filter: string,
  machineScope: "this" | "all" = "this",
): Promise<CleanupInventoryData> {
  if (!hasBackend) {
    const seeded = (window as unknown as { __AIJ_CLEANUP__?: CleanupInventoryData })
      .__AIJ_CLEANUP__;
    if (seeded === undefined) {
      return {
        groups: [],
        clean_instances: 0,
        local_machine: "",
        ledger_degraded: false,
        history: null,
      };
    }
    // Mirror the backend contract for the seam (AJ-E511): the
    // default scope lists THIS machine's rows only; "all" widens.
    if (machineScope !== "all" && seeded.local_machine !== "") {
      const local = seeded.local_machine;
      const groups = seeded.groups
        .map((group) => ({
          ...group,
          rows: group.rows.filter(
            (row) => row.machine_id === undefined || row.machine_id === local,
          ),
        }))
        .filter((group) => group.rows.length > 0);
      return { ...seeded, groups };
    }
    return seeded;
  }
  return invoke<CleanupInventoryData>("cleanup_inventory", { filter, machineScope });
}

/** AJ-E507: the kill sheet's step-1 verify (zero signalling). */
export async function killVerify(sessionId: string, pid: number): Promise<KillVerifyData> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_KILL_VERIFY__?: (session: string, pid: number) => KillVerifyData })
      .__AIJ_KILL_VERIFY__?.(sessionId, pid) ?? {
      pid,
      session: sessionId,
      verified: false,
      refusal: "This PID now belongs to a different program — Juggler will not touch it.",
      exe_path: "?",
      cmdline: "",
      cwd: "",
      ports: [],
      pgid: null,
      escaped_group: false,
      members: [],
      tree_pids: [],
      platform: "TERM → grace → KILL",
      grace_secs: 5,
      provider: "unknown",
      account_id: null,
      project_id: null,
    };
  }
  return invoke<KillVerifyData>("kill_verify", { sessionId, pid });
}

/** AJ-E507: the audited TERM→grace→KILL ladder (step 3). */
export async function killExecute(
  sessionId: string,
  pid: number,
  scope: KillScope,
  attempt: string,
  goalPort: number | null,
): Promise<KillExecuteData> {
  if (!hasBackend) {
    const fixture = (
      window as unknown as {
        __AIJ_KILL_EXECUTE__?: (scope: KillScope, goalPort: number | null) => KillExecuteData;
      }
    ).__AIJ_KILL_EXECUTE__;
    if (fixture !== undefined) {
      return fixture(scope, goalPort);
    }
    // The countdown state's seam: a ladder that never resolves.
    if ((window as unknown as { __AIJ_KILL_EXECUTE_PENDING__?: boolean }).__AIJ_KILL_EXECUTE_PENDING__) {
      return new Promise<KillExecuteData>(() => {});
    }
    return {
      refusal: null,
      members: [{ pid, outcome: "already gone" }],
      cancelled: false,
      all_succeeded: true,
      still_holding: [],
      reoffer_pid: null,
    };
  }
  return invoke<KillExecuteData>("kill_execute", { sessionId, pid, scope, attempt, goalPort });
}

/** AJ-E507: cancel a running ladder's grace window. */
export async function killCancel(attempt: string): Promise<void> {
  if (!hasBackend) {
    return Promise.resolve();
  }
  return invoke("kill_cancel", { attempt });
}

/** AJ-E507: the grace window's per-process exit ticks. */
export async function killLiveness(pids: number[]): Promise<boolean[]> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_KILL_LIVENESS__?: (pids: number[]) => boolean[] })
      .__AIJ_KILL_LIVENESS__?.(pids) ?? pids.map(() => false);
  }
  return invoke<boolean[]>("kill_liveness", { pids });
}

/** AJ-E505: the port occupancy map. */
export async function portInventory(): Promise<PortInventoryInfo> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_PORTS__?: PortInventoryInfo }).__AIJ_PORTS__
      ?? { rows: [], note: null, hogs: [] };
  }
  return invoke<PortInventoryInfo>("port_inventory");
}

/** AJ-E505: the launch pre-check — a warning NEVER blocks launch. */
export async function launchPortWarning(projectId: string | null): Promise<PortWarningInfo | null> {
  if (!hasBackend) {
    return (window as unknown as { __AIJ_PORT_WARNING__?: PortWarningInfo }).__AIJ_PORT_WARNING__
      ?? null;
  }
  return invoke<PortWarningInfo | null>("launch_port_warning", { projectId });
}

/** AJ-E410: stop one orphan (allow-list guarded, TERM→KILL ladder). */
export async function orphanStop(sessionId: string, pid: number): Promise<StopOutcomeInfo> {
  if (!hasBackend) {
    // Mock: the allow-list guard refuses anything but fixture orphans.
    const orphan = MOCK_PROCESSES[sessionId]?.orphans.find((row) => row.pid === pid);
    const state = orphan
      ? orphan.stop_state ?? "gone"
      : "refused - not a recorded descendant";
    return { pid, state };
  }
  return invoke<StopOutcomeInfo>("orphan_stop", { sessionId, pid });
}

/** AJ-E410: stop every confirmed orphan of one Instance. */
export async function orphanStopAll(sessionId: string): Promise<StopOutcomeInfo[]> {
  if (!hasBackend) {
    const orphans = MOCK_PROCESSES[sessionId]?.orphans ?? [];
    return orphans.map((orphan) => ({ pid: orphan.pid, state: orphan.stop_state ?? "gone" }));
  }
  return invoke<StopOutcomeInfo[]>("orphan_stop_all", { sessionId });
}

/** AJ-E410: re-check the ppid chain for one orphan pid. */
export async function orphanVerify(sessionId: string, pid: number): Promise<OrphanVerifyInfo> {
  if (!hasBackend) {
    const orphan = MOCK_PROCESSES[sessionId]?.orphans.find((row) => row.pid === pid);
    if (orphan) {
      return {
        pid,
        alive: !orphan.stop_state,
        parent_pid: 1,
        note: "alive — re-parented to init/launchd (its parent exited)",
      };
    }
    return { pid, alive: false, parent_pid: null, note: "process is gone" };
  }
  return invoke<OrphanVerifyInfo>("orphan_verify", { sessionId, pid });
}

/** AJ-E411 (D-12): set (validate server-side) or clear the user-entered renewal date. */
export async function accountSetRenewal(
  accountId: string,
  date: string | null,
): Promise<AccountInfo> {
  if (!hasBackend) {
    // AJ-E309: the wizard sets a date on accounts created in this
    // preview session — record it on a seam instead of throwing.
    const seams = window as unknown as { __AIJ_RENEWAL_SET__?: [string, string | null][] };
    seams.__AIJ_RENEWAL_SET__ = [...(seams.__AIJ_RENEWAL_SET__ ?? []), [accountId, date]];
    const account = MOCK_ACCOUNTS.find((row) => row.id === accountId);
    if (!account) {
      return {
        id: accountId,
        provider: "claude-code",
        label: accountId,
        plan: null,
        login_state: "logged_out",
        cleanup_opt_in: false,
        renewal_date: date,
        preview_opt_in: false,
      };
    }
    return { ...account, renewal_date: date };
  }
  return invoke<AccountInfo>("account_set_renewal", { accountId, date });
}

/** AJ-E410 (D-08): set the per-Account Tier-1 cleanup opt-in. */
export async function accountSetCleanup(accountId: string, optIn: boolean): Promise<AccountInfo> {
  if (!hasBackend) {
    const account = MOCK_ACCOUNTS.find((row) => row.id === accountId);
    if (!account) throw new Error(`unknown account ${accountId}`);
    return { ...account, cleanup_opt_in: optIn };
  }
  return invoke<AccountInfo>("account_set_cleanup", { accountId, optIn });
}

export async function stopSession(sessionId: string): Promise<void> {
  if (!hasBackend) {
    return;
  }
  await invoke("stop_session", { sessionId });
}

export async function dashboardStats(): Promise<StatsInfo> {
  if (!hasBackend) {
    return MOCK_STATS;
  }
  return invoke<StatsInfo>("dashboard_stats");
}

export function onSessionsUpdated(
  handler: (sessions: InstanceInfo[]) => void,
): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<InstanceInfo[]>(EVENTS.sessionsUpdated, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {
      // backend gone; callers poll via sessionsList
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/**
 * 5 s resource tick (AJ-E401): coalesced I-07/I-08 deltas for live
 * Instances — merge by `id` into the session rows; absent ids keep
 * their honest `null`s.
 */
export function onSessionsResources(
  handler: (deltas: InstanceResourcesInfo[]) => void,
): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<InstanceResourcesInfo[]>(EVENTS.sessionsResources, (event) =>
    handler(event.payload),
  )
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {
      // backend gone; the row payload still carries resource fields
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/**
 * 30 s quota tick (AJ-E401): coalesced I-05 deltas — merge by `id`;
 * `quota: null` is the honest "no feed exists for this Provider".
 */
export function onSessionsQuota(
  handler: (deltas: InstanceQuotaInfo[]) => void,
): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<InstanceQuotaInfo[]>(EVENTS.sessionsQuota, (event) =>
    handler(event.payload),
  )
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {
      // backend gone; the row payload still carries the quota field
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

export function onAppReady(handler: (info: AppInfo) => void): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<AppInfo>(EVENTS.appReady, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
      }
    })
    .catch(() => {});
  return () => {
    disposed = true;
    unlisten?.();
  };
}

// ---- embedded terminal (M1-4) --------------------------------------------

export const TERMINAL_EVENTS = {
  output: "terminal://output",
  exit: "terminal://exit",
} as const;

/** Spawns the guided login for an account inside an embedded terminal. */
export async function terminalSpawnLogin(accountId: string): Promise<TerminalInfo> {
  if (!hasBackend) return { id: `login-preview-${accountId}` };
  return invoke<TerminalInfo>("terminal_spawn_login", { accountId });
}

/** Starts streaming a supervised session into the dock (idempotent). */
export async function terminalAttach(id: string): Promise<TerminalInfo> {
  return invoke<TerminalInfo>("terminal_attach", { id });
}

/** Sends keystrokes to a terminal. */
export async function terminalWrite(id: string, data: string): Promise<void> {
  return invoke<void>("terminal_write", { id, data });
}

/** Resizes a terminal's PTY. */
export async function terminalResize(
  id: string,
  cols: number,
  rows: number,
): Promise<void> {
  return invoke<void>("terminal_resize", { id, cols, rows });
}

/** Stops a terminal's process (graceful ladder). */
export async function terminalKill(id: string): Promise<void> {
  return invoke<void>("terminal_kill", { id });
}

/** The retained output tail (base64) for replay on panel reopen. */
export async function terminalSnapshot(id: string): Promise<string> {
  return invoke<string>("terminal_snapshot", { id });
}

/** Live output stream for one terminal (base64 chunks). */
export function onTerminalOutput(
  id: string,
  handler: (base64: string) => void,
): () => void {
  return listenScoped(TERMINAL_EVENTS.output, (payload: { id: string; data: string }) => {
    if (payload.id === id) handler(payload.data);
  });
}

/** A terminal's process exited. */
export function onTerminalExit(handler: (id: string) => void): () => void {
  return listenScoped(TERMINAL_EVENTS.exit, (payload: { id: string }) =>
    handler(payload.id),
  );
}

function listenScoped<T>(
  event: string,
  handler: (payload: T) => void,
): () => void {
  if (!hasBackend) {
    return () => {};
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<T>(event, (event_) => handler(event_.payload)).then((stop) => {
    if (disposed) {
      stop();
    } else {
      unlisten = stop;
    }
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}


// ---- golden path: backfill, handover, directory picking -------------------

/** Backfills the projects rail from imported transcript history (idempotent). */
export async function projectsBackfill(): Promise<number> {
  if (!hasBackend) return 0;
  return invoke<number>("projects_backfill");
}

/** The juggle: move a session's context to another account and resume it. */
export async function sessionHandover(
  sessionId: string,
  targetProfileId: string,
): Promise<LaunchResult> {
  return invoke<LaunchResult>("session_handover", { sessionId, targetProfileId });
}

/** AJ-E205: the preflight verdict for one switch (rendered by the
 * freeze dialog before anything executes). */
export interface PreflightInfoData {
  mechanism: string;
  fidelity: string;
  score: number;
  warnings: string[];
  zombies: string[];
  diff_snapshotted: boolean;
  source_live: boolean;
  experimental_copy: string;
  /** AJ-E206 §8 door verdict: "one-click" | "wizard". */
  door: "one-click" | "wizard";
  /** Wizard step titles in §8 order (empty for 1-click). */
  wizard_steps: string[];
  /** Why the wizard opened (one line per failed §8 condition). */
  wizard_reasons: string[];
  /** The target is the project's dominant account. */
  usual_account: boolean;
  /** The carry-list the door renders (AJ-E206). */
  carry: CarryListData;
  /** AJ-E208: the destination Provider's redaction policy. */
  redaction: RedactionInfoData;
  /** AJ-E208: detected secrets (masked) — the per-item un-redact list. */
  redaction_items: RedactionItemData[];
  /** AJ-E808: validated per the fidelity register (absent = not validated). */
  validated?: boolean;
}

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

/** The carry-list grouped by disposition (AJ-E206). */
export interface CarryListData {
  carried: string[];
  regenerated: string[];
  dropped: string[];
}

/** AJ-E206: the executed switch's handles (the edge is the undo handle). */
export interface SwitchExecutedData {
  session_id: string;
  pid: number;
  edge_id: string;
  source_session_id: string;
}

/** The no-backend preflight default (fixtures merge over it —
 * AJ-E206 door facts + AJ-E208 redaction policy). */
const DEFAULT_PREFLIGHT: PreflightInfoData = {
  mechanism: "transplant",
  fidelity: "F1",
  score: 0.85,
  warnings: [],
  zombies: [],
  diff_snapshotted: false,
  source_live: true,
  experimental_copy:
    "Experimental: only the main transcript moves. Subagent files and rewind history stay on the source. The source stays resumable.",
  door: "wizard",
  wizard_steps: ["Target", "What's carried", "Confirm"],
  wizard_reasons: ["This isn't the usual account for this project."],
  usual_account: false,
  carry: {
    carried: ["chat transcript", "rules & guidelines", "uncommitted git diff"],
    regenerated: [],
    dropped: ["todo list", "subagent state"],
  },
  redaction: {
    global: "strip",
    override: null,
    effective: "strip",
    source: "global",
    line: "Redaction for claude-code: Strip — replace keys with ••• (your global choice)",
  },
  redaction_items: [],
};

/** AJ-E205: preflight — ordered checks stop BEFORE any byte moves;
 * warnings render in the freeze dialog. */
export async function switchPreflight(
  sessionId: string,
  targetProfileId: string,
): Promise<PreflightInfoData> {
  if (!hasBackend) {
    const seam = (window as unknown as { __AIJ_SWITCH_PREFLIGHT__?: Partial<PreflightInfoData> })
      .__AIJ_SWITCH_PREFLIGHT__;
    // Merge over the default so fixtures that predate a field keep
    // working (the seam overrides only what it defines).
    if (seam) {
      return {
        ...DEFAULT_PREFLIGHT,
        ...seam,
        redaction: { ...DEFAULT_PREFLIGHT.redaction, ...seam.redaction },
      } as PreflightInfoData;
    }
    return DEFAULT_PREFLIGHT;
  }
  return invoke<PreflightInfoData>("switch_preflight", { sessionId, targetProfileId });
}

/** AJ-E205: executes the switch with the user's freeze choice
 * (`finish-turn` default | `stop-now`) — the pipeline stages,
 * verifies, launches, and records the edge. */
/** AJ-E1103: why the next switch happens — set when the user picks a
 * target (the suggested one, or another), read once by
 * [`switchExecute`]. */
let pendingSwitchTrigger: "suggestion" | "manual" | null = null;

/** AJ-E1103: the user picked a switch target. */
export function noteSwitchPick(suggested: boolean): void {
  pendingSwitchTrigger = suggested ? "suggestion" : "manual";
}

export async function switchExecute(
  sessionId: string,
  targetProfileId: string,
  freezeChoice: string,
  redactionChoice?: string | null,
  redactionUnredact?: number[],
): Promise<SwitchExecutedData> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_SWITCH_EXECUTE_FAIL__?: string;
      __AIJ_SWITCH_EXECUTE_HANG__?: boolean;
      __AIJ_SWITCH_EXECUTE_REDACTION__?: {
        redactionChoice: string | null | undefined;
        redactionUnredact: number[];
      }[];
    };
    // The guardrail seam: the redaction payload is observable so
    // tests can prove it fires only from the click.
    seams.__AIJ_SWITCH_EXECUTE_REDACTION__ = [
      ...(seams.__AIJ_SWITCH_EXECUTE_REDACTION__ ?? []),
      {
        redactionChoice,
        redactionUnredact: redactionUnredact ?? [],
      },
    ];
    if (seams.__AIJ_SWITCH_EXECUTE_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_SWITCH_EXECUTE_FAIL__);
    }
    if (seams.__AIJ_SWITCH_EXECUTE_HANG__) {
      return new Promise<SwitchExecutedData>(() => {});
    }
    return {
      session_id: `sess-preview-${Date.now()}`,
      pid: 0,
      edge_id: `edge-preview-${Date.now()}`,
      source_session_id: sessionId,
    };
  }
  const trigger = pendingSwitchTrigger ?? "manual";
  pendingSwitchTrigger = null;
  return invoke<SwitchExecutedData>("switch_execute", {
    sessionId,
    targetProfileId,
    freezeChoice,
    redactionChoice: redactionChoice ?? null,
    redactionUnredact: redactionUnredact ?? [],
    trigger,
  });
}

/** AJ-E206 §8.2: undoes a completed switch — stops the continuation
 * instance, resumes the source's native session, records the
 * compensating edge. User-click only. */
export async function switchUndo(edgeId: string): Promise<LaunchResult> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_SWITCH_UNDO_CALLS__?: string[];
      __AIJ_SWITCH_UNDO_FAIL__?: string;
    };
    // The guardrail seam: undo is observable so tests can prove it
    // fires ONLY from the click.
    seams.__AIJ_SWITCH_UNDO_CALLS__ = [
      ...(seams.__AIJ_SWITCH_UNDO_CALLS__ ?? []),
      edgeId,
    ];
    if (seams.__AIJ_SWITCH_UNDO_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_SWITCH_UNDO_FAIL__);
    }
    return { session_id: `sess-undo-${Date.now()}`, pid: 0 };
  }
  return invoke<LaunchResult>("switch_undo", { edgeId });
}

/** Native macOS folder picker; null when cancelled. */
export async function pickDirectory(): Promise<string | null> {
  if (!hasBackend) {
    // AJ-E307 test seam: specs inject the "picked" folder.
    return (
      (window as unknown as { __AIJ_PICK_DIRECTORY__?: string })
        .__AIJ_PICK_DIRECTORY__ ?? null
    );
  }
  const selection = await openDialog({ directory: true, multiple: false });
  return typeof selection === "string" ? selection : null;
}

/** Folders dragged onto the window (Tauri drag-drop events). */
export function onFolderDropped(handler: (path: string) => void): () => void {
  if (!hasBackend) return () => {};
  let disposed = false;
  let unlisten: (() => void) | null = null;
  import("@tauri-apps/api/webview")
    .then(({ getCurrentWebview }) =>
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "drop") {
          for (const path of event.payload.paths) handler(path);
        }
      }),
    )
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** AJ-E207: one tracked-artifact conflict (three radio choices each). */
export interface SwitchBackConflictData {
  rel_path: string;
  base_sha: string | null;
  away_sha: string | null;
  original_sha: string | null;
}

/** AJ-E207: the switch-back plan (suggest-only; the user clicks). */
export interface SwitchBackPlanData {
  suggestion: string;
  original_account: string;
  away_account: string;
  original_native_id: string;
  original_alive: boolean;
  sf7_fallback: string;
  sf7_copy: string;
  away_live: boolean;
  both_live_copy: string;
  divergence: string;
  delta_preview: string;
  away_only_turns: number;
  original_only_turns: number;
  base_turns: number;
  conflicts: SwitchBackConflictData[];
}

/** AJ-E207: one suggested return (the card the user clicks). */
export interface SwitchBackSuggestionData {
  session_id: string;
  original_account: string;
  away_account: string;
  copy: string;
}

/** AJ-E207 Req.3: the suggested returns whose original account has
 * quota headroom again (the reset trigger) — suggest-only. */
export async function switchBackSuggestions(): Promise<SwitchBackSuggestionData[]> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_SWITCH_BACK_SUGGESTIONS__?: SwitchBackSuggestionData[] })
        .__AIJ_SWITCH_BACK_SUGGESTIONS__ ?? []
    );
  }
  return invoke<SwitchBackSuggestionData[]>("switch_back_suggestions");
}

/** AJ-E207: the plan for one return — verbatim suggestion, S-F7
 * verdict, both-live warning, divergence counts, delta preview,
 * conflict list. Read-only. */
export async function switchBackPlan(sessionId: string): Promise<SwitchBackPlanData> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_SWITCH_BACK_PLAN__?: SwitchBackPlanData })
        .__AIJ_SWITCH_BACK_PLAN__ ?? {
        suggestion:
          "Work Max has reset. Resume the original conversation there? Work from Personal arrives as one 'while you were away' update.",
        original_account: "Work Max",
        away_account: "Personal",
        original_native_id: "native-5",
        original_alive: true,
        sf7_fallback: "",
        sf7_copy: "",
        away_live: true,
        both_live_copy:
          "Both accounts are now live on this work item: Work Max (resumed) and Personal (still running). Check which one you type into.",
        divergence:
          "Both sides moved since the switch. 3 new turns and 1 file conflicts need your choice — nothing is merged silently.",
        delta_preview:
          "While you were away on Personal — 3 new turns (5 shared before the switch, 1 new here on Work Max).\n\n- [user] fixed the login bug\n- [assistant] suite green — 34 passing\n\nFull detail: the attached switch pack (not replayed).",
        away_only_turns: 3,
        original_only_turns: 1,
        base_turns: 5,
        conflicts: [
          { rel_path: "todos.json", base_sha: "h0", away_sha: "h1", original_sha: "h2" },
        ],
      }
    );
  }
  return invoke<SwitchBackPlanData>("switch_back_plan", { sessionId });
}

/** AJ-E207: executes the return with the user's choices — resumes
 * the ORIGINAL native session with one delta turn; never a
 * copy-back. */
export async function switchBackExecute(
  sessionId: string,
  choices: { rel_path: string; choice: string }[],
  keepAwayRunning: boolean,
): Promise<LaunchResult> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_SWITCH_BACK_EXECUTE_FAIL__?: string;
      __AIJ_SWITCH_BACK_EXECUTE_HANG__?: boolean;
      __AIJ_SWITCH_BACK_EXECUTE_CALLS__?: unknown[];
    };
    // The guardrail seam: executions are observable so tests can
    // prove they fire ONLY from the click.
    seams.__AIJ_SWITCH_BACK_EXECUTE_CALLS__ = [
      ...(seams.__AIJ_SWITCH_BACK_EXECUTE_CALLS__ ?? []),
      { sessionId, choices, keepAwayRunning },
    ];
    if (seams.__AIJ_SWITCH_BACK_EXECUTE_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_SWITCH_BACK_EXECUTE_FAIL__);
    }
    if (seams.__AIJ_SWITCH_BACK_EXECUTE_HANG__) {
      return new Promise<LaunchResult>(() => {});
    }
    return { session_id: `sess-return-${Date.now()}`, pid: 0 };
  }
  return invoke<LaunchResult>("switch_back_execute", {
    sessionId,
    choices,
    keepAwayRunning,
  });
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
  return invoke<RedactionInfoData>("redaction_policy_get", { provider });
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
  await invoke("redaction_policy_set_global", { policy });
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
  await invoke("redaction_policy_set_override", { provider, policy });
}

/** AJ-E209: one ranked, badged switch destination (the handover
 * section's rows — §Q6 extended with switch cost + return bonus).
 * Suggest-only: the click is the user's. */
export interface SwitchTargetData {
  recipe_id: string;
  /** The target's Provider id (AJ-E1103 counts suggestions by it). */
  provider?: string;
  account_id: string;
  account_label: string;
  /** Ordered-pair badge wire name ("F1 · Beta" — never "lossless"
   * below F4; cross-Provider always carries Beta, D-04/D-11). */
  badge: string;
  /** Validated state (false until E210's fixtures flip cells). */
  validated: boolean;
  cross_provider: boolean;
  /** The pair's Drop list (the badge's honest companion). */
  drops: string[];
  score?: number;
  filtered_reason?: string;
  top_factors: string[];
  factors: [string, number][];
  confidence: string;
  tte_min?: number;
  suggested: boolean;
}

/** The no-backend switch-targets default (preview mode shows a
 * realistic ranked, badged set; fixtures override wholesale). */
const DEFAULT_SWITCH_TARGETS: SwitchTargetData[] = [
  {
    recipe_id: "recipe-work-max",
    account_id: "acct-work",
    account_label: "Work Max",
    badge: "F1 · Beta",
    validated: false,
    cross_provider: true,
    drops: ["todo list", "subagent state"],
    score: 0.62,
    top_factors: ["80% headroom", "usual account for this repo"],
    factors: [["80% headroom", 0.36], ["usual account for this repo", 0.2], ["fidelity 1 · Beta", 0.15], ["≈$0.12 replay", 0.07], ["quiet", 0.1]],
    confidence: "Computed",
    tte_min: 262,
    suggested: true,
  },
  {
    recipe_id: "recipe-personal",
    account_id: "acct-personal",
    account_label: "Personal",
    badge: "F1 · Beta",
    validated: false,
    cross_provider: true,
    drops: ["todo list", "subagent state"],
    score: 0.41,
    top_factors: ["35% headroom", "quiet"],
    factors: [["35% headroom", 0.16], ["quiet", 0.1], ["fidelity 1 · Beta", 0.15]],
    confidence: "Estimate",
    tte_min: 95,
    suggested: false,
  },
  {
    recipe_id: "recipe-lab",
    account_id: "acct-lab",
    account_label: "Lab",
    badge: "F2 · Beta",
    validated: false,
    cross_provider: true,
    drops: ["rewind history", "subagent state"],
    filtered_reason: "exhausted — resets in 3h",
    top_factors: [],
    factors: [],
    confidence: "Measured",
    suggested: false,
  },
];

/** AJ-E209 Req.1/3: the ranked + badged destinations for one source
 * session. Read-only — ranking suggests, the user clicks. */
export async function switchTargets(sessionId: string): Promise<SwitchTargetData[]> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_SWITCH_TARGETS__?: SwitchTargetData[] })
        .__AIJ_SWITCH_TARGETS__ ?? DEFAULT_SWITCH_TARGETS
    );
  }
  return invoke<SwitchTargetData[]>("switch_targets", { sessionId });
}

/** AJ-E209: one epoch node on a Work's timeline. */
export interface EpochNodeData {
  id: string;
  provider: string;
  account_label: string;
  kind: string;
  /** A native session exists AND its transcript survives (the
   * Resume action's honest gate). */
  resumable: boolean;
  session_id?: string | null;
}

/** AJ-E209: one labelled timeline edge. */
export interface GraphEdgeData {
  from: string;
  to?: string | null;
  /** "transplant F1 · 12:04" / "replay F1 · ≈41k tok" / "switch-back ↺". */
  label: string;
  failed_reason?: string | null;
  switch_back: boolean;
}

/** AJ-E209: one Work item's timeline (epochs past the collapse cut). */
export interface WorkGraphData {
  title: string;
  epochs: EpochNodeData[];
  edges: GraphEdgeData[];
  /** Older epochs collapsed away ("+n switches" — never unbounded). */
  collapsed: number;
  /** The S-F9 hop line (inform, never block). */
  hop_note?: string | null;
}

/** The no-backend graph default (preview mode; fixtures override). */
const DEFAULT_SWITCH_GRAPH: WorkGraphData[] = [
  {
    title: "Login refactor",
    epochs: [
      { id: "ep-1", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-a" },
      { id: "ep-2", provider: "codex", account_label: "Personal", kind: "terminal", resumable: true, session_id: "sess-b" },
      { id: "ep-3", provider: "claude-code", account_label: "Work Max", kind: "terminal", resumable: true, session_id: "sess-c" },
    ],
    edges: [
      { from: "ep-1", to: "ep-2", label: "transplant F1 · 12:04", switch_back: false },
      { from: "ep-2", to: "ep-3", label: "switch-back ↺ · F1", switch_back: true },
    ],
    collapsed: 2,
    hop_note: "3 switches so far ≈41k tokens",
  },
];

/** AJ-E209 Req.4: the per-Work Switch Graph timelines for one
 * project. Read-only. */
export async function switchGraphOverview(projectId: string): Promise<WorkGraphData[]> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_SWITCH_GRAPH__?: WorkGraphData[] })
        .__AIJ_SWITCH_GRAPH__ ?? DEFAULT_SWITCH_GRAPH
    );
  }
  return invoke<WorkGraphData[]>("switch_graph_overview", { projectId });
}

/** AJ-E209 Req.5: resumes an epoch's native session (the graph's
 * Resume action — the user's click, never automatic). */
export async function epochResume(epochId: string): Promise<LaunchResult> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_EPOCH_RESUME_CALLS__?: string[];
      __AIJ_EPOCH_RESUME_FAIL__?: string;
    };
    // The guardrail seam: resume is observable so tests can prove it
    // fires ONLY from the click.
    seams.__AIJ_EPOCH_RESUME_CALLS__ = [...(seams.__AIJ_EPOCH_RESUME_CALLS__ ?? []), epochId];
    if (seams.__AIJ_EPOCH_RESUME_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_EPOCH_RESUME_FAIL__);
    }
    return { session_id: `sess-resume-${Date.now()}`, pid: 0 };
  }
  return invoke<LaunchResult>("epoch_resume", { epochId });
}

/** AJ-E209 Req.5: exports an epoch's pack through the AJ-E208 GLOBAL
 * redaction policy — returns the pack directory for the toast. */
export async function packExport(epochId: string): Promise<string> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_PACK_EXPORT_CALLS__?: string[];
      __AIJ_PACK_EXPORT_FAIL__?: string;
    };
    // The guardrail seam: exports are observable so tests can prove
    // they fire ONLY from the click.
    seams.__AIJ_PACK_EXPORT_CALLS__ = [...(seams.__AIJ_PACK_EXPORT_CALLS__ ?? []), epochId];
    if (seams.__AIJ_PACK_EXPORT_FAIL__ !== undefined) {
      return Promise.reject(seams.__AIJ_PACK_EXPORT_FAIL__);
    }
    return `~/Library/Application Support/AI Juggler/exports/pack-preview-${Date.now()}`;
  }
  return invoke<string>("pack_export", { epochId });
}

/** AJ-E210: one recorded live dogfood cell as the Beta table reads it. */
export interface DogfoodCellSummaryData {
  at: string;
  project_shape: string;
  verdict: "pass" | "fail" | string;
  source_sha256: string;
  target_sha256: string;
  note: string;
}

/** AJ-E210: one ordered-pair validation matrix row (Settings →
 * Beta). The SAME matrix the switch badges derive from — one
 * source, no drift (D-04). */
export interface ValidationCellData {
  from: string;
  to: string;
  mechanism: string;
  researched_ceiling: string;
  advertised: string;
  badge: string;
  validated: boolean;
  mechanism_green: boolean;
  ci_green: boolean;
  dogfood?: DogfoodCellSummaryData | null;
  status: string;
  notes: string[];
}

/** The no-backend matrix default (mirrors the researched table:
 * one real-CI row, everything honestly open beta debt). */
const DEFAULT_VALIDATION_MATRIX: ValidationCellData[] = [
  ["claude-code", "claude-code", "transplant", "F1", true],
  ["codex", "codex", "transplant", "F1", false],
  ["zcode", "zcode", "transplant", "F1", false],
  ["claude-code", "codex", "replay", "F1", false],
  ["codex", "claude-code", "replay", "F1", false],
  ["claude-code", "zcode", "replay", "F0", false],
  ["zcode", "claude-code", "replay", "F1", false],
  ["codex", "zcode", "replay", "F0", false],
  ["zcode", "codex", "replay", "F1", false],
  ["claude-code", "gemini-cli", "pack-only", "F0", false],
  ["claude-code", "opencode", "pack-only", "F0", false],
].map(([from, to, mechanism, ceiling, ciGreen]) => ({
  from: from as string,
  to: to as string,
  mechanism: mechanism as string,
  researched_ceiling: ceiling as string,
  advertised: ceiling as string,
  badge: `${ceiling as string} · Beta`,
  validated: false,
  mechanism_green: true,
  ci_green: ciGreen as boolean,
  dogfood: null,
  status: ciGreen
    ? "CI fixtures green · live dogfood pending — open beta debt"
    : "Real-adapter fixtures not landed · live dogfood pending — open beta debt",
  notes: [],
}));

/** AJ-E210: the validation matrix (read-only disclosure — the UI
 * shows the same table the badges derive from; cells are recorded
 * by the maintainer, never from this surface). */
export async function validationMatrix(): Promise<ValidationCellData[]> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_VALIDATION_MATRIX__?: ValidationCellData[] })
        .__AIJ_VALIDATION_MATRIX__ ?? DEFAULT_VALIDATION_MATRIX
    );
  }
  return invoke<ValidationCellData[]>("validation_matrix");
}

/** AJ-E301: extracts the CURATED message from any command rejection —
 * coded errors carry `user_message`; JS-side causes fall back to
 * their string. The UI never renders a raw OS string. */
export function errorText(cause: unknown): string {
  if (cause !== null && typeof cause === "object" && "user_message" in cause) {
    const wire = cause as { user_message?: unknown };
    return typeof wire.user_message === "string" && wire.user_message.length > 0
      ? wire.user_message
      : "Something went wrong — try again.";
  }
  const text = String(cause);
  return text === "[object Object]" ? "Something went wrong — try again." : text;
}

/** AJ-E309 Req.6: appends one funnel event to the LOCAL store (the
 * backend enforces the allow-list schema — counts, vocabulary words,
 * lowercase codes; never paths or labels). Preview mode records it on
 * `window.__AIJ_FUNNEL_EVENTS__` for specs. */
export async function funnelEventEmit(
  kind: string,
  fields: Record<string, string>,
): Promise<void> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_FUNNEL_EVENTS__?: { kind: string; fields: Record<string, string> }[];
    };
    seams.__AIJ_FUNNEL_EVENTS__ = [...(seams.__AIJ_FUNNEL_EVENTS__ ?? []), { kind, fields }];
    return;
  }
  return invoke<void>("funnel_event", { kind, fields });
}

export type { UsageStatsInfo } from "../types/generated/UsageStatsInfo";
export type { UsageStatsOverview } from "../types/generated/UsageStatsOverview";
export type { UsageStatsSetResult } from "../types/generated/UsageStatsSetResult";
export type { UsageReportRow } from "../types/generated/UsageReportRow";
export type { UsageStatRow } from "../types/generated/UsageStatRow";
import type { UsageStatsInfo as UsageStatsInfoData } from "../types/generated/UsageStatsInfo";
import type { UsageStatsOverview as UsageStatsOverviewData } from "../types/generated/UsageStatsOverview";
import type { UsageStatsSetResult as UsageStatsSetResultData } from "../types/generated/UsageStatsSetResult";

/** The usage-stats modes. */
export type UsageStatsMode = "off" | "local" | "share";

/** Preview mode's usage-stats state (tests and gallery captures set it). */
type StatsSeam = {
  __AIJ_USAGE_STATS__?: UsageStatsOverviewData;
  __AIJ_USAGE_STATS_REPORTS__?: Record<string, string>;
  __AIJ_USAGE_STATS_SETS__?: string[];
};

function previewStats(): UsageStatsOverviewData {
  const seams = window as unknown as StatsSeam;
  seams.__AIJ_USAGE_STATS__ ??= {
    info: {
      included: true,
      chosen: "off",
      effective: "off",
      can_upload: false,
      reason: null,
      decided_at_ms: null,
      consent_version: 0,
      current_consent_version: 1,
    },
    week: "2026-W42",
    rows: [],
    reports: [],
    next_upload_ms: null,
    last_error: null,
    docs_url: "https://github.com/figurehead-ai/AI-juggler/blob/main/docs/usage-stats.md",
    sync: { offered: false, on: false, email: null, notice: null },
  };
  return seams.__AIJ_USAGE_STATS__;
}

/** AJ-E1102: the usage-stats choice and what applies. */
export async function usageStatsInfo(): Promise<UsageStatsInfoData> {
  if (!hasBackend) return previewStats().info;
  return invoke<UsageStatsInfoData>("usage_stats_info");
}

/** AJ-E1102: sets the mode (`off` deletes everything counted). */
export async function usageStatsSet(mode: UsageStatsMode): Promise<UsageStatsSetResultData> {
  if (!hasBackend) {
    const seams = window as unknown as StatsSeam;
    seams.__AIJ_USAGE_STATS_SETS__ = [...(seams.__AIJ_USAGE_STATS_SETS__ ?? []), mode];
    const current = previewStats();
    const info: UsageStatsInfoData = {
      ...current.info,
      chosen: mode,
      effective: current.info.reason ? current.info.effective : mode,
      can_upload: mode === "share" && !current.info.reason,
      decided_at_ms: Date.now(),
      consent_version: current.info.current_consent_version,
    };
    seams.__AIJ_USAGE_STATS__ = mode === "off" ? { ...current, info, rows: [], reports: [] } : { ...current, info };
    return {
      info,
      message:
        mode === "off"
          ? "Usage stats are off. Local stats deleted."
          : mode === "local"
            ? "Usage stats are counted on this Mac only."
            : "Usage stats are on. One small report a week will be sent.",
    };
  }
  return invoke<UsageStatsSetResultData>("usage_stats_set", { mode });
}

/** AJ-E1105: everything Settings ▸ Privacy shows. */
export async function usageStatsOverview(): Promise<UsageStatsOverviewData> {
  if (!hasBackend) return previewStats();
  return invoke<UsageStatsOverviewData>("usage_stats_overview");
}

/** AJ-E1105: one report's exact JSON (read-only). */
export async function usageStatsReport(status: string, week: string): Promise<string> {
  if (!hasBackend) {
    const reports = (window as unknown as StatsSeam).__AIJ_USAGE_STATS_REPORTS__ ?? {};
    return reports[`${status}/${week}`] ?? "{}";
  }
  return invoke<string>("usage_stats_report", { status, week });
}

/** AJ-E1105: "Delete local stats" (the mode is kept). */
export async function usageStatsDeleteLocal(): Promise<UsageStatsSetResultData> {
  if (!hasBackend) {
    const current = previewStats();
    (window as unknown as StatsSeam).__AIJ_USAGE_STATS__ = { ...current, rows: [], reports: [], last_error: null };
    return { info: current.info, message: "Local stats deleted." };
  }
  return invoke<UsageStatsSetResultData>("usage_stats_delete_local");
}

/** AJ-E1109: turns the personal stats sync on or off. */
export async function usageStatsSetSync(on: boolean): Promise<UsageStatsOverviewData> {
  if (!hasBackend) {
    const current = previewStats();
    (window as unknown as StatsSeam).__AIJ_USAGE_STATS__ = {
      ...current,
      sync: { ...current.sync, on, notice: on ? null : current.sync.notice },
    };
    return previewStats();
  }
  return invoke<UsageStatsOverviewData>("usage_stats_set_sync", { on });
}

export type { CrashInfo } from "../types/generated/CrashInfo";
export type { CrashReport } from "../types/generated/CrashReport";
import type { CrashInfo as CrashInfoData } from "../types/generated/CrashInfo";

function previewCrash(): CrashInfoData {
  const seams = window as unknown as { __AIJ_CRASH__?: CrashInfoData };
  seams.__AIJ_CRASH__ ??= { setting: "never", effective: "never", reason: null, asking: [], queued: 0, can_send: false };
  return seams.__AIJ_CRASH__;
}

/** AJ-E1107: the crash-report setting and the reports waiting. */
export async function crashInfo(): Promise<CrashInfoData> {
  if (!hasBackend) return previewCrash();
  return invoke<CrashInfoData>("crash_info");
}

/** AJ-E1107: `never` (deletes every kept report) | `ask` | `always`. */
export async function crashSet(setting: "never" | "ask" | "always"): Promise<CrashInfoData> {
  if (!hasBackend) {
    const current = previewCrash();
    const next: CrashInfoData = {
      ...current,
      setting,
      effective: current.reason ? "never" : setting,
      ...(setting === "never" ? { asking: [], queued: 0 } : {}),
    };
    (window as unknown as { __AIJ_CRASH__?: CrashInfoData }).__AIJ_CRASH__ = next;
    return next;
  }
  return invoke<CrashInfoData>("crash_set", { setting });
}

/** AJ-E1107: the answer to "Send a crash report?" for one report. */
export async function crashAnswer(id: string, answer: "send" | "dont_send" | "always_send"): Promise<CrashInfoData> {
  if (!hasBackend) {
    const current = previewCrash();
    const asking = current.asking.filter((report) => report.id !== id);
    const next: CrashInfoData = {
      ...current,
      asking,
      queued: current.queued + (answer === "dont_send" ? 0 : 1),
      setting: answer === "always_send" ? "always" : current.setting,
    };
    (window as unknown as { __AIJ_CRASH__?: CrashInfoData; __AIJ_CRASH_ANSWERS__?: string[] }).__AIJ_CRASH__ = next;
    const w = window as unknown as { __AIJ_CRASH_ANSWERS__?: string[] };
    w.__AIJ_CRASH_ANSWERS__ = [...(w.__AIJ_CRASH_ANSWERS__ ?? []), `${id}:${answer}`];
    return next;
  }
  return invoke<CrashInfoData>("crash_answer", { id, answer });
}

/** AJ-E1105: opens the public list of every metric. */
export async function usageStatsOpenDocs(): Promise<void> {
  if (!hasBackend) return;
  return invoke<void>("usage_stats_open_docs");
}

/** AJ-E1103: the usage-statistics metrics the interface counts. The
 * backend accepts only these, and only with the schema's values (a
 * `provider` may be any Provider id; it is mapped there). */
export type UiStatMetric =
  | "ui.screen_opened"
  | "ui.inspector_tab"
  | "ui.notification"
  | "ui.shortcut_used"
  | "cleanup_centre.opened"
  | "usage.suggestion"
  | "resource.alert"
  | "onboarding.login"
  | "onboarding.flow"
  | "update.flow"
  | "perf.board_interactive_ms";

/** AJ-E1103: counts one interface event — fire and forget, never awaited
 * on a user path, and a no-op unless the user opted in to usage stats. */
export function statsEvent(metric: UiStatMetric, dims: Record<string, string> = {}, value?: number): void {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_STATS_EVENTS__?: { metric: string; dims: Record<string, string>; value?: number }[];
    };
    seams.__AIJ_STATS_EVENTS__ = [...(seams.__AIJ_STATS_EVENTS__ ?? []), { metric, dims, value }];
    return;
  }
  void invoke<void>("usage_stats_ui_event", { metric, dims, value: value ?? null }).catch(() => undefined);
}

/** AJ-E1103: a display preference as this week's gauge (fire and forget). */
export function statsGauge(metric: "ui.density" | "ui.theme", value: string): void {
  if (!hasBackend) {
    const seams = window as unknown as { __AIJ_STATS_GAUGES__?: Record<string, string> };
    seams.__AIJ_STATS_GAUGES__ = { ...(seams.__AIJ_STATS_GAUGES__ ?? {}), [metric]: value };
    return;
  }
  void invoke<void>("usage_stats_ui_gauge", { metric, value }).catch(() => undefined);
}

/** AJ-E309 Req.7: the LOCAL funnel report (markdown; never a network
 * call). Preview mode returns a short honest placeholder. */
export async function funnelExport(): Promise<string> {
  if (!hasBackend) {
    return "# Onboarding funnel — local export\n\n(preview mode — no events are stored)\n";
  }
  return invoke<string>("funnel_export");
}

/** AJ-E309 Req.5: the Expert-setup facts for a Provider (no account
 * yet) or one Account (+ optionally one of its Launch settings). */
export async function expertSetupInfo(
  provider: string,
  accountId: string | null,
  recipeId: string | null = null,
): Promise<ExpertSetupInfo> {
  if (!hasBackend) {
    const seam = (window as unknown as { __AIJ_EXPERT__?: Partial<ExpertSetupInfo> })
      .__AIJ_EXPERT__;
    const root = `/Users/you/Library/Application Support/AI Juggler/accounts/${provider}`;
    const dir = accountId === null ? root : `${root}/${accountId}`;
    return {
      provider,
      account_id: accountId,
      state_dir: dir,
      exists: accountId !== null,
      shell_snippet: accountId === null ? null : `CLAUDE_CONFIG_DIR='${dir}' claude`,
      protected_env_keys: ["CLAUDE_CONFIG_DIR", "AI_JUGGLER_CLAUDE_BIN"],
      note:
        accountId === null
          ? "Each new account gets its own isolated folder in here — the shell line appears once the account exists."
          : "This account keeps all of its state here — isolated from your other accounts and from your own install.",
      ...seam,
    };
  }
  return invoke<ExpertSetupInfo>("expert_setup_info", { provider, accountId, recipeId });
}

/** AJ-E309 Req.5: [Reveal in Finder] — the backend resolves the
 * managed folder itself (never a caller-supplied path). */
export async function expertReveal(provider: string, accountId: string | null): Promise<void> {
  if (!hasBackend) {
    return;
  }
  return invoke<void>("expert_reveal", { provider, accountId });
}

/** AJ-E309 Req.5: one Launch settings' environment overrides. */
export async function launchRecipeEnvGet(recipeId: string): Promise<Record<string, string>> {
  if (!hasBackend) {
    const store = (window as unknown as { __AIJ_RECIPE_ENV__?: Record<string, Record<string, string>> })
      .__AIJ_RECIPE_ENV__;
    return { ...(store?.[recipeId] ?? {}) };
  }
  return invoke<Record<string, string>>("launch_recipe_env_get", { recipeId });
}

/** AJ-E309 Req.5: replaces one Launch settings' environment overrides
 * (validated server-side: isolation and credential-shaped keys are
 * refused with a coded `validation` error). */
export async function launchRecipeEnvSet(
  recipeId: string,
  env: Record<string, string>,
): Promise<Record<string, string>> {
  if (!hasBackend) {
    const seams = window as unknown as {
      __AIJ_RECIPE_ENV__?: Record<string, Record<string, string>>;
      __AIJ_RECIPE_ENV_REJECT__?: string;
    };
    if (seams.__AIJ_RECIPE_ENV_REJECT__) {
      throw { code: "validation", retryable: false, user_message: seams.__AIJ_RECIPE_ENV_REJECT__ };
    }
    seams.__AIJ_RECIPE_ENV__ = { ...(seams.__AIJ_RECIPE_ENV__ ?? {}), [recipeId]: { ...env } };
    return { ...env };
  }
  return invoke<Record<string, string>>("launch_recipe_env_set", { recipeId, env });
}

// --- AJ-E702: Fleet identity + paired-peer registry ----------------------

type FleetSeams = {
  __AIJ_FLEET_SELF__?: Partial<FleetSelfInfo>;
  __AIJ_FLEET_PEERS__?: FleetPeerInfo[];
  __AIJ_FLEET_CALLS__?: string[];
};

const fleetSeams = () => window as unknown as FleetSeams;

function mockFleetSelf(): FleetSelfInfo {
  return {
    machine_id: "machine-7f3a",
    label: "Studio Mac",
    fingerprint: "3F2A 9C01 B7E4 55D0 12AF 8C3E 0D91 66B2",
    trust: "trusted",
    trust_reason: null,
    peer_count: fleetSeams().__AIJ_FLEET_PEERS__?.length ?? 0,
    ...fleetSeams().__AIJ_FLEET_SELF__,
  };
}

function recordFleetCall(call: string): void {
  const seams = fleetSeams();
  seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), call];
}

/** This Mac's Fleet identity (label, fingerprint, trust state). */
export async function fleetSelf(): Promise<FleetSelfInfo> {
  if (!hasBackend) return mockFleetSelf();
  return invoke<FleetSelfInfo>("fleet_self");
}

/** The machines paired with this Mac. */
export async function fleetPeers(): Promise<FleetPeerInfo[]> {
  if (!hasBackend) return [...(fleetSeams().__AIJ_FLEET_PEERS__ ?? [])];
  return invoke<FleetPeerInfo[]>("fleet_peers");
}

/** Unpairs one machine (deletes its pin). */
export async function fleetUnpair(machineId: string): Promise<void> {
  if (!hasBackend) {
    recordFleetCall(`unpair:${machineId}`);
    const seams = fleetSeams();
    seams.__AIJ_FLEET_PEERS__ = (seams.__AIJ_FLEET_PEERS__ ?? []).filter(
      (peer) => peer.machine_id !== machineId,
    );
    return;
  }
  return invoke<void>("fleet_unpair", { machineId });
}

/** Renames this Mac locally. */
export async function fleetSetLabel(label: string): Promise<FleetSelfInfo> {
  if (!hasBackend) {
    recordFleetCall(`label:${label.trim()}`);
    fleetSeams().__AIJ_FLEET_SELF__ = { ...fleetSeams().__AIJ_FLEET_SELF__, label: label.trim() };
    return mockFleetSelf();
  }
  return invoke<FleetSelfInfo>("fleet_set_label", { label });
}

/** Clears a broken registry so pairing can start again. */
export async function fleetResetTrust(): Promise<FleetSelfInfo> {
  if (!hasBackend) {
    recordFleetCall("reset-trust");
    fleetSeams().__AIJ_FLEET_PEERS__ = [];
    fleetSeams().__AIJ_FLEET_SELF__ = { ...fleetSeams().__AIJ_FLEET_SELF__, trust: "trusted", trust_reason: null };
    return mockFleetSelf();
  }
  return invoke<FleetSelfInfo>("fleet_reset_trust");
}

/** Generates a new identity for this Mac; every peer must pair again. */
export async function fleetResetIdentity(): Promise<FleetSelfInfo> {
  if (!hasBackend) {
    recordFleetCall("reset-identity");
    fleetSeams().__AIJ_FLEET_PEERS__ = [];
    fleetSeams().__AIJ_FLEET_SELF__ = {
      ...fleetSeams().__AIJ_FLEET_SELF__,
      fingerprint: "A1C4 77E0 5B2D 9F13 C8E6 0447 DD21 9A5F",
    };
    return mockFleetSelf();
  }
  return invoke<FleetSelfInfo>("fleet_reset_identity");
}

// --- AJ-E704: Link on/off, discovery, pairing -----------------------------
//
// The mock is a small scripted state machine so the wizard can be
// driven end-to-end in the browser (Playwright, gallery):
// `window.__AIJ_PAIR_SIMULATE__("joiner" | "mismatch" | "expire" |
// "cancel")` plays the other machine's side of an "Add a machine"
// window; a joiner code of 000000 fails as a mismatch, and
// `__AIJ_PAIR_JOIN_FAIL__ = "<reason>"` fails a join with that reason.

/** What the permissions step grants the other machine. */
export type PairPermission = "view" | "control";

type LinkSeams = FleetSeams & {
  __AIJ_LINK__?: LinkStatusInfo;
  __AIJ_DISCOVERED__?: DiscoveredMachineInfo[];
  __AIJ_PAIR_HOST__?: PairingStateInfo;
  __AIJ_PAIR_JOIN__?: PairingStateInfo;
  __AIJ_PAIR_LOCK__?: "locked_out" | "disabled";
  __AIJ_PAIR_JOIN_FAIL__?: string;
  __AIJ_PAIR_SIMULATE__?: (event: string) => void;
};

const linkSeams = () => window as unknown as LinkSeams;

const MOCK_JOINER = {
  label: "Laptop",
  fingerprint: "9B10 44CE 7A0F E2D3 51B8 06C9 3E7D A425",
};
const MOCK_HOST = {
  label: "Mac mini",
  fingerprint: "C2E9 1F7A 0B36 8D54 E1A0 7C29 46BF 3D18",
};

const idlePairing = (): PairingStateInfo => ({
  phase: "idle",
  code: null,
  link: null,
  expires_at_ms: null,
  peer_label: null,
  peer_fingerprint: null,
  message: null,
  reason: null,
});

function mockLink(): LinkStatusInfo {
  return linkSeams().__AIJ_LINK__ ?? { enabled: false, port: null, error: null };
}

function mockWaiting(code: string, reason: string | null, message: string | null): PairingStateInfo {
  return {
    ...idlePairing(),
    phase: "waiting",
    code,
    link: `aijuggler-pair:v1:${code.replace(" ", "")}:${mockFleetSelf().fingerprint.replace(/ /g, "")}`,
    expires_at_ms: Date.now() + 120_000,
    reason,
    message,
  };
}

function mockPin(label: string, fingerprint: string, permission: PairPermission): void {
  const seams = linkSeams();
  const id = `machine-${label.toLowerCase().replace(/[^a-z]/g, "")}`;
  const peer: FleetPeerInfo = {
    machine_id: id,
    label,
    fingerprint,
    permission,
    permission_label: permission === "control" ? "View + control" : "View only",
    ask_each_time: false,
    paired_via: "code",
    paired_at_ms: Date.now(),
    last_seen_ms: Date.now(),
    connection: mockLink().enabled ? "connected" : "link_off",
    protocol: mockLink().enabled ? "v1.0" : null,
    connection_detail: null,
    queued_changes: null,
  };
  seams.__AIJ_FLEET_PEERS__ = [
    ...(seams.__AIJ_FLEET_PEERS__ ?? []).filter((existing) => existing.machine_id !== id),
    peer,
  ];
}

function installPairSimulator(): void {
  const seams = linkSeams();
  if (seams.__AIJ_PAIR_SIMULATE__) return;
  seams.__AIJ_PAIR_SIMULATE__ = (event: string) => {
    const host = seams.__AIJ_PAIR_HOST__ ?? idlePairing();
    if (event === "joiner" && host.phase === "waiting") {
      seams.__AIJ_PAIR_HOST__ = {
        ...idlePairing(),
        phase: "confirm",
        peer_label: MOCK_JOINER.label,
        peer_fingerprint: MOCK_JOINER.fingerprint,
      };
    } else if (event === "mismatch") {
      seams.__AIJ_PAIR_HOST__ = mockWaiting("731 058", "mismatch", "That code didn't match — here's a new one.");
    } else if (event === "expire") {
      seams.__AIJ_PAIR_HOST__ = {
        ...idlePairing(),
        phase: "failed",
        reason: "expired",
        message: "The pairing window closed after 2 minutes.",
      };
    } else if (event === "cancel") {
      seams.__AIJ_PAIR_HOST__ = mockWaiting(
        host.code ?? "482 913",
        "cancelled",
        "Pairing was cancelled on one of the machines — the code still works.",
      );
    }
  };
}

/** Link status (off by default). */
export async function linkStatus(): Promise<LinkStatusInfo> {
  if (!hasBackend) return mockLink();
  return invoke<LinkStatusInfo>("link_status");
}

/** Turns Link on or off (a user click; remembered). */
export async function linkSetEnabled(enabled: boolean): Promise<LinkStatusInfo> {
  if (!hasBackend) {
    recordFleetCall(`link:${enabled ? "on" : "off"}`);
    linkSeams().__AIJ_LINK__ = { enabled, port: enabled ? 47821 : null, error: null };
    const seams = linkSeams();
    seams.__AIJ_FLEET_PEERS__ = (seams.__AIJ_FLEET_PEERS__ ?? []).map((peer) => ({
      ...peer,
      connection: enabled ? (peer.connection === "link_off" ? "connected" : peer.connection) : "link_off",
      protocol: enabled ? (peer.protocol ?? "v1.0") : null,
    }));
    return mockLink();
  }
  return invoke<LinkStatusInfo>("link_set_enabled", { enabled });
}

/** Machines on this network that aren't paired. */
export async function fleetDiscovered(): Promise<DiscoveredMachineInfo[]> {
  if (!hasBackend) return mockLink().enabled ? [...(linkSeams().__AIJ_DISCOVERED__ ?? [])] : [];
  return invoke<DiscoveredMachineInfo[]>("fleet_discovered");
}

/** Opens "Add a machine" (shows a code for 2 minutes). */
export async function pairingHostOpen(): Promise<PairingStateInfo> {
  if (!hasBackend) {
    recordFleetCall("pair-host-open");
    installPairSimulator();
    const lock = linkSeams().__AIJ_PAIR_LOCK__;
    linkSeams().__AIJ_PAIR_HOST__ = lock
      ? {
          ...idlePairing(),
          phase: "failed",
          reason: lock,
          message:
            lock === "disabled"
              ? "Pairing is turned off after too many wrong codes today — turn it back on in Fleet."
              : "Too many wrong codes — pairing is paused for a minute.",
        }
      : mockWaiting("482 913", null, null);
    return { ...linkSeams().__AIJ_PAIR_HOST__! };
  }
  return invoke<PairingStateInfo>("pairing_host_open");
}

/** The host's pairing state (polled while the wizard is open). */
export async function pairingHostState(): Promise<PairingStateInfo> {
  if (!hasBackend) return { ...(linkSeams().__AIJ_PAIR_HOST__ ?? idlePairing()) };
  return invoke<PairingStateInfo>("pairing_host_state");
}

/** The host user's decision: pair (granting `permission`) or cancel. */
export async function pairingHostDecide(commit: boolean, permission: PairPermission): Promise<PairingStateInfo> {
  if (!hasBackend) {
    recordFleetCall(`pair-host-decide:${commit ? permission : "cancel"}`);
    const host = linkSeams().__AIJ_PAIR_HOST__ ?? idlePairing();
    if (commit && host.phase === "confirm") {
      mockPin(host.peer_label ?? "Laptop", host.peer_fingerprint ?? "", permission);
      linkSeams().__AIJ_PAIR_HOST__ = { ...idlePairing(), phase: "paired", peer_label: host.peer_label };
    } else {
      linkSeams().__AIJ_PAIR_HOST__ = mockWaiting(
        "482 913",
        "cancelled",
        "Pairing was cancelled on one of the machines — the code still works.",
      );
    }
    return { ...linkSeams().__AIJ_PAIR_HOST__! };
  }
  return invoke<PairingStateInfo>("pairing_host_decide", { commit, permission });
}

/** Closes "Add a machine". */
export async function pairingHostClose(): Promise<void> {
  if (!hasBackend) {
    recordFleetCall("pair-host-close");
    linkSeams().__AIJ_PAIR_HOST__ = idlePairing();
    return;
  }
  return invoke<void>("pairing_host_close");
}

/** Turns pairing back on after the daily disable (a user click). */
export async function pairingReEnable(): Promise<void> {
  if (!hasBackend) {
    recordFleetCall("pair-re-enable");
    linkSeams().__AIJ_PAIR_LOCK__ = undefined;
    return;
  }
  return invoke<void>("pairing_re_enable");
}

/** Starts joining a machine found on the network, or at an address. */
export async function pairingJoinStart(
  target: { instanceId: string } | { address: string },
  code: string,
): Promise<PairingStateInfo> {
  if (!hasBackend) {
    recordFleetCall(`pair-join:${"address" in target ? target.address : target.instanceId}`);
    const digits = code.startsWith("aijuggler-pair:") ? code.split(":")[2] ?? "" : code.replace(/\D/g, "");
    const fail = linkSeams().__AIJ_PAIR_JOIN_FAIL__ ?? (digits === "000000" ? "mismatch" : undefined);
    linkSeams().__AIJ_PAIR_JOIN__ = fail
      ? { ...idlePairing(), phase: "failed", reason: fail, message: null }
      : {
          ...idlePairing(),
          phase: "confirm",
          peer_label: MOCK_HOST.label,
          peer_fingerprint: MOCK_HOST.fingerprint,
        };
    return { ...linkSeams().__AIJ_PAIR_JOIN__! };
  }
  return invoke<PairingStateInfo>("pairing_join_start", {
    instanceId: "instanceId" in target ? target.instanceId : null,
    address: "address" in target ? target.address : null,
    code,
  });
}

/** The joiner's pairing state (polled while the wizard is open). */
export async function pairingJoinState(): Promise<PairingStateInfo> {
  if (!hasBackend) return { ...(linkSeams().__AIJ_PAIR_JOIN__ ?? idlePairing()) };
  return invoke<PairingStateInfo>("pairing_join_state");
}

/** The joiner user's decision: pair (granting `permission`) or cancel. */
export async function pairingJoinDecide(commit: boolean, permission: PairPermission): Promise<PairingStateInfo> {
  if (!hasBackend) {
    recordFleetCall(`pair-join-decide:${commit ? permission : "cancel"}`);
    const join = linkSeams().__AIJ_PAIR_JOIN__ ?? idlePairing();
    if (commit && join.phase === "confirm") {
      mockPin(join.peer_label ?? "Mac mini", join.peer_fingerprint ?? "", permission);
      linkSeams().__AIJ_PAIR_JOIN__ = { ...idlePairing(), phase: "paired", peer_label: join.peer_label };
    } else {
      linkSeams().__AIJ_PAIR_JOIN__ = { ...idlePairing(), phase: "failed", reason: "cancelled", message: null };
    }
    return { ...linkSeams().__AIJ_PAIR_JOIN__! };
  }
  return invoke<PairingStateInfo>("pairing_join_decide", { commit, permission });
}

/** Resets the joiner (Done / Start over / Close). */
export async function pairingJoinReset(): Promise<void> {
  if (!hasBackend) {
    linkSeams().__AIJ_PAIR_JOIN__ = idlePairing();
    return;
  }
  return invoke<void>("pairing_join_reset");
}

// --- AJ-E1202/E1203: the optional Figurehead account ----------------------
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

const MOCK_FEATURES: CloudFeatureInfo[] = [
  { key: "cloudBackup", access: "included", tier: "free", on: false },
  { key: "fleetDiscovery", access: "included", tier: "free", on: false },
  { key: "usageSync", access: "included", tier: "free", on: false },
];

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

function fhCall(call: string): void {
  const seams = fhSeams();
  seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), call];
}

function fhLinked(name?: string): FigureheadStatusInfo {
  return fhSet({
    ...fhStatus(),
    state: "linked",
    email: "you@example.com",
    plan: "Free",
    device_name: name?.trim() || fhStatus().device_name,
    features: (fhSeams().__AIJ_FIGUREHEAD_FEATURES__ ?? MOCK_FEATURES).map((f) => ({ ...f })),
    link: null,
    refreshed_at_ms: Date.now(),
    notice: null,
  });
}

/** The Figurehead status (local — never a network call). */
export async function figureheadStatus(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) return { ...fhStatus() };
  return invoke<FigureheadStatusInfo>("figurehead_status");
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
  return invoke<FigureheadStatusInfo>("figurehead_link_start", { name: name ?? null });
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
  return invoke<FigureheadStatusInfo>("figurehead_link_poll");
}

/** Cancels a browser link. */
export async function figureheadLinkCancel(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) return fhSet({ ...fhStatus(), state: "signed_out", link: null });
  return invoke<FigureheadStatusInfo>("figurehead_link_cancel");
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
  return invoke<FigureheadStatusInfo>("figurehead_claim", { code, name: name ?? null });
}

/** Refreshes the account now. */
export async function figureheadRefresh(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) return { ...fhStatus() };
  return invoke<FigureheadStatusInfo>("figurehead_refresh");
}

/** Renames this Mac on Figurehead. */
export async function figureheadRename(name: string): Promise<FigureheadStatusInfo> {
  if (!hasBackend) {
    fhCall(`fh-rename:${name.trim()}`);
    return fhSet({ ...fhStatus(), device_name: name.trim() });
  }
  return invoke<FigureheadStatusInfo>("figurehead_rename", { name });
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
  return invoke<FigureheadStatusInfo>("figurehead_unlink");
}

/** Dismisses the one-time notice. */
export async function figureheadDismissNotice(): Promise<FigureheadStatusInfo> {
  if (!hasBackend) return fhSet({ ...fhStatus(), notice: null });
  return invoke<FigureheadStatusInfo>("figurehead_dismiss_notice");
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
  return invoke<FigureheadStatusInfo>("figurehead_save_choices", { features });
}

/** Opens a Figurehead page (`verify` | `dashboard` | `devices` | `upgrade`). */
export async function figureheadOpenPage(target: "verify" | "dashboard" | "devices" | "upgrade"): Promise<void> {
  if (!hasBackend) {
    fhCall(`fh-open:${target}`);
    return;
  }
  return invoke<void>("figurehead_open_page", { target });
}

// --- AJ-E705: what paired machines report (remote inspector) -------------

type RemoteSeams = {
  __AIJ_FLEET_REMOTE__?: RemoteMachineInfo[];
  __AIJ_REMOTE_STOP__?: RemoteCommandOutcomeInfo;
  __AIJ_FLEET_CALLS__?: string[];
};

const remoteSeams = () => window as unknown as RemoteSeams;

/** Every paired machine with what it last reported (source + freshness). */
export async function fleetRemote(): Promise<RemoteMachineInfo[]> {
  if (!hasBackend) return [...(remoteSeams().__AIJ_FLEET_REMOTE__ ?? [])];
  return invoke<RemoteMachineInfo[]>("fleet_remote");
}

/** AJ-E707: asks a paired machine to clean one process it owns (it
 * re-verifies and runs its own ladder). */
export async function fleetRemoteClean(
  machineId: string,
  instanceId: string,
  pid: number,
): Promise<RemoteCommandOutcomeInfo> {
  if (!hasBackend) {
    const seams = remoteSeams() as RemoteSeams & { __AIJ_REMOTE_CLEAN__?: RemoteCommandOutcomeInfo };
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `remote-clean:${machineId}:${instanceId}:${pid}`];
    return seams.__AIJ_REMOTE_CLEAN__ ?? { accepted: true, reason_code: null, message: `Exited (pid ${pid})` };
  }
  return invoke<RemoteCommandOutcomeInfo>("fleet_remote_clean", { machineId, instanceId, pid });
}

/** AJ-E711: asks a paired machine to bring one of its desktop windows
 * forward — executed there, with its permissions. */
export async function fleetRemoteFocus(machineId: string, instanceId: string): Promise<RemoteCommandOutcomeInfo> {
  if (!hasBackend) {
    const seams = remoteSeams() as RemoteSeams & { __AIJ_REMOTE_FOCUS__?: RemoteCommandOutcomeInfo };
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `remote-focus:${machineId}:${instanceId}`];
    return seams.__AIJ_REMOTE_FOCUS__ ?? { accepted: true, reason_code: null, message: "Focused window" };
  }
  return invoke<RemoteCommandOutcomeInfo>("fleet_remote_focus", { machineId, instanceId });
}

/** Asks a paired machine to stop one of its Instances; the node decides. */
export async function fleetRemoteStop(machineId: string, instanceId: string): Promise<RemoteCommandOutcomeInfo> {
  if (!hasBackend) {
    const seams = remoteSeams();
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `remote-stop:${machineId}:${instanceId}`];
    return (
      seams.__AIJ_REMOTE_STOP__ ?? { accepted: true, reason_code: null, message: "Stopping" }
    );
  }
  return invoke<RemoteCommandOutcomeInfo>("fleet_remote_stop", { machineId, instanceId });
}

// --- AJ-E709: permissions changed on this Mac; "Ask each time" prompts ---

type PromptSeams = {
  __AIJ_FLEET_PROMPTS__?: FleetPromptInfo[];
  __AIJ_FLEET_CALLS__?: string[];
  __AIJ_FLEET_PEERS__?: FleetPeerInfo[];
};
const promptSeams = () => window as unknown as PromptSeams;

/** Changes what a paired machine may do on THIS Mac (only ever here). */
export async function fleetSetPermission(
  machineId: string,
  permission: "view" | "control",
  askEachTime: boolean,
): Promise<FleetPeerInfo[]> {
  if (!hasBackend) {
    const seams = promptSeams();
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `permission:${machineId}:${permission}:${askEachTime}`];
    seams.__AIJ_FLEET_PEERS__ = (seams.__AIJ_FLEET_PEERS__ ?? []).map((peer) =>
      peer.machine_id === machineId
        ? {
            ...peer,
            permission,
            permission_label: permission === "control" ? "View + control" : "View only",
            ask_each_time: permission === "control" && askEachTime,
          }
        : peer,
    );
    return [...(seams.__AIJ_FLEET_PEERS__ ?? [])];
  }
  return invoke<FleetPeerInfo[]>("fleet_set_permission", { machineId, permission, askEachTime });
}

/** Control commands waiting for this Mac's user ("Ask each time"). */
export async function fleetPrompts(): Promise<FleetPromptInfo[]> {
  if (!hasBackend) return [...(promptSeams().__AIJ_FLEET_PROMPTS__ ?? [])];
  return invoke<FleetPromptInfo[]>("fleet_prompts");
}

/** Allows or denies a waiting command. */
export async function fleetPromptDecide(id: number, allow: boolean): Promise<void> {
  if (!hasBackend) {
    const seams = promptSeams();
    seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), `prompt:${id}:${allow ? "allow" : "deny"}`];
    seams.__AIJ_FLEET_PROMPTS__ = (seams.__AIJ_FLEET_PROMPTS__ ?? []).filter((p) => p.id !== id);
    return;
  }
  return invoke<void>("fleet_prompt_decide", { id, allow });
}

/**
 * AJ-E308: the ready-check across login-state generations — the
 * taxonomy's `signed_in` plus the legacy `logged_in` string (scan
 * candidates still carry it). Everything else (partial / expired /
 * sso_pending / api_key / logged_out / unknown) is not ready.
 */
export function signedIn(state: string | null | undefined): boolean {
  return state === "signed_in" || state === "logged_in";
}

/** True when the coded rejection says a retry can succeed. */
export function errorRetryable(cause: unknown): boolean {
  return (
    cause !== null &&
    typeof cause === "object" &&
    "retryable" in cause &&
    (cause as { retryable?: unknown }).retryable === true
  );
}

/** AJ-E301: one Step-0 install-gate row. */
export interface ProviderInstallData {
  provider: string;
  display_name: string;
  install_url: string;
  /** installed | not_installed | unverified | probe_failed */
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
const DEFAULT_SCAN_REPORT: ScanReportData = {
  installs: [
    {
      harness: "claude-code",
      path: "/Users/you/.claude",
      origin: "vendor-default",
      login: "logged_in",
      bytes: 48_000_000,
      last_used_secs: Math.floor(Date.now() / 1000) - 86_400,
      managed: false,
      in_use: [],
      notes: ["Claude Code's standard location — what a plain `claude` command uses."],
      inference: {
        purpose: "Main",
        purpose_confidence: "guessed",
        login: "signed-in",
        login_confidence: "found",
        activity: "active",
        activity_confidence: "found",
        activity_words: "Active this week",
        keychain_bound: false,
      },
      signals: ["vendor-default"],
    },
    {
      harness: "claude-code",
      path: "/Users/you/.claude-side",
      origin: "home-pattern",
      login: "logged_out",
      bytes: 1_200_000,
      last_used_secs: Math.floor(Date.now() / 1000) - 40 * 86_400,
      managed: false,
      in_use: [],
      notes: ["A separate Claude configuration — created manually or by another tool."],
      inference: {
        purpose: "Personal",
        purpose_confidence: "likely",
        login: "not-signed-in",
        login_confidence: "found",
        activity: "dormant",
        activity_confidence: "found",
        activity_words: "Dormant 40 d",
        keychain_bound: false,
      },
      signals: ["home-pattern", "CLAUDE_CONFIG_DIR in ~/.zshrc:12"],
    },
  ],
  probes: [
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
  ].map(([id, label, detail]) => ({
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
    return seam ?? DEFAULT_SCAN_REPORT;
  }
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  const { listen } = await import("@tauri-apps/api/event");
  let stop: (() => void) | undefined;
  if (progress) {
    stop = await listen<{ done: number; total: number }>("scan-progress", (event) => {
      progress(event.payload.done, event.payload.total);
    }).then((unlisten) => unlisten);
  }
  void getCurrentWebview;
  try {
    return await invoke<ScanReportData>("environment_scan_full");
  } finally {
    stop?.();
  }
}

// --- AJ-E706: cross-machine switching ------------------------------------

type CrossSeams = {
  __AIJ_FLEET_SWITCH_TARGETS__?: RemoteSwitchTargetInfo[];
  __AIJ_FLEET_SWITCH_OUTCOME__?: FleetSwitchOutcomeInfo;
  __AIJ_FLEET_SWITCH_BACK_PLAN__?: FleetSwitchBackPlanInfo | null;
  __AIJ_FLEET_SWITCH_BACK_OUTCOMES__?: FleetSwitchOutcomeInfo[];
  __AIJ_FLEET_DIVERGENCES__?: DivergenceRecordInfo[];
  __AIJ_FLEET_CALLS__?: string[];
  __AIJ_FLEET_SWITCH_PROGRESS__?: (update: FleetSwitchProgressInfo) => void;
};

const crossSeams = () => window as unknown as CrossSeams;
const crossCall = (call: string) => {
  const seams = crossSeams();
  seams.__AIJ_FLEET_CALLS__ = [...(seams.__AIJ_FLEET_CALLS__ ?? []), call];
};

/** Paired machines as switch destinations for one source Instance —
 * computed from what they published; no transcript moves to list them. */
export async function fleetSwitchTargets(sessionId: string): Promise<RemoteSwitchTargetInfo[]> {
  if (!hasBackend) return [...(crossSeams().__AIJ_FLEET_SWITCH_TARGETS__ ?? [])];
  return invoke<RemoteSwitchTargetInfo[]>("fleet_switch_targets", { sessionId });
}

/** Runs one confirmed cross-machine switch (after the user's click). */
export async function fleetSwitchExecute(request: FleetSwitchRequest): Promise<FleetSwitchOutcomeInfo> {
  if (!hasBackend) {
    crossCall(`switch:${request.machine_id}:${request.recipe_id}:${request.mechanism}:${request.project_id ?? ""}`);
    const progress = crossSeams().__AIJ_FLEET_SWITCH_PROGRESS__;
    progress?.({ machine_id: request.machine_id, phase: "receiving", received_bytes: 6_000_000, total_bytes: 12_000_000 });
    return (
      crossSeams().__AIJ_FLEET_SWITCH_OUTCOME__ ?? {
        accepted: true,
        reason_code: null,
        message: "Continuing under Desktop Max in storefront",
        instance_id: "sess-remote-new",
        edge_id: "edge-x",
        divergence: null,
      }
    );
  }
  return invoke<FleetSwitchOutcomeInfo>("fleet_switch_execute", { request });
}

/** Cancels the push to `machineId` (within one chunk). */
export async function fleetSwitchCancel(machineId: string): Promise<boolean> {
  if (!hasBackend) {
    crossCall(`switch-cancel:${machineId}`);
    return true;
  }
  return invoke<boolean>("fleet_switch_cancel", { machineId });
}

/** Push progress while a cross-machine switch runs. */
export function onFleetSwitchProgress(handler: (update: FleetSwitchProgressInfo) => void): () => void {
  if (!hasBackend) {
    crossSeams().__AIJ_FLEET_SWITCH_PROGRESS__ = handler;
    return () => {
      delete crossSeams().__AIJ_FLEET_SWITCH_PROGRESS__;
    };
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<FleetSwitchProgressInfo>(EVENTS.fleetSwitchProgress, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {
      /* backend gone */
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** The switch-back plan for an Instance that came from another machine
 * (`null` for any other Instance). */
export async function fleetSwitchBackPlan(sessionId: string): Promise<FleetSwitchBackPlanInfo | null> {
  if (!hasBackend) return crossSeams().__AIJ_FLEET_SWITCH_BACK_PLAN__ ?? null;
  return invoke<FleetSwitchBackPlanInfo | null>("fleet_switch_back_plan", { sessionId });
}

/** Switches an away Instance back to its original machine; `choice` is
 * empty, or `keep_mine` / `keep_both` after a diverged refusal. */
export async function fleetSwitchBackExecute(
  sessionId: string,
  choice: "" | "keep_mine" | "keep_both",
  keepAwayRunning: boolean,
): Promise<FleetSwitchOutcomeInfo> {
  if (!hasBackend) {
    crossCall(`switch-back:${sessionId}:${choice}`);
    const queue = crossSeams().__AIJ_FLEET_SWITCH_BACK_OUTCOMES__ ?? [];
    const next = queue.shift();
    return (
      next ?? {
        accepted: true,
        reason_code: null,
        message: "Resumed the original under Work Max.",
        instance_id: "sess-back",
        edge_id: "edge-back",
        divergence: null,
      }
    );
  }
  return invoke<FleetSwitchOutcomeInfo>("fleet_switch_back_execute", { sessionId, choice, keepAwayRunning });
}

/** Every unresolved divergence (the sync chip's "Diverged — N needs you"). */
export async function fleetDivergences(): Promise<DivergenceRecordInfo[]> {
  if (!hasBackend) return [...(crossSeams().__AIJ_FLEET_DIVERGENCES__ ?? [])];
  return invoke<DivergenceRecordInfo[]>("fleet_divergences");
}

/** "Keep theirs": leave both sides as they are (nothing is sent). */
export async function fleetDivergenceKeepTheirs(sessionId: string): Promise<void> {
  if (!hasBackend) {
    crossCall(`keep-theirs:${sessionId}`);
    const seams = crossSeams();
    seams.__AIJ_FLEET_DIVERGENCES__ = (seams.__AIJ_FLEET_DIVERGENCES__ ?? []).filter(
      (row) => row.session_id !== sessionId,
    );
    return;
  }
  return invoke<void>("fleet_divergence_keep_theirs", { sessionId });
}

// --- AJ-E1003: park until reset --------------------------------------------

type ParkSeams = {
  __AIJ_PARKED_OFFERS__?: ParkedOfferInfo[];
  __AIJ_PARK_CALLS__?: string[];
  __AIJ_PARKED_CHECK__?: ParkedInfo;
};
const parkSeams = () => window as unknown as ParkSeams;
const parkCall = (call: string) => {
  const seams = parkSeams();
  seams.__AIJ_PARK_CALLS__ = [...(seams.__AIJ_PARK_CALLS__ ?? []), call];
};

/** Parks a quota-blocked Instance until its window resets (pack staged). */
export async function instancePark(sessionId: string): Promise<ParkedInfo> {
  if (!hasBackend) {
    parkCall(`park:${sessionId}`);
    return {
      session_id: sessionId,
      account_label: "Work Max",
      parked_at_ms: Date.now(),
      resets_at_ms: Date.now() + 2 * 3_600_000,
      confidence: "measured",
      due: false,
      pack_dir: "",
    };
  }
  return invoke<ParkedInfo>("instance_park", { sessionId });
}

/** The resume offers due now (polled with the alerts — never a timer of
 * its own). */
export async function parkedOffers(): Promise<ParkedOfferInfo[]> {
  if (!hasBackend) return [...(parkSeams().__AIJ_PARKED_OFFERS__ ?? [])];
  return invoke<ParkedOfferInfo[]>("parked_offers");
}

/** "Not now": snoozes once, then stays parked quietly. */
export async function parkedDismiss(sessionId: string): Promise<void> {
  if (!hasBackend) {
    parkCall(`dismiss:${sessionId}`);
    const seams = parkSeams();
    seams.__AIJ_PARKED_OFFERS__ = (seams.__AIJ_PARKED_OFFERS__ ?? []).filter((row) => row.session_id !== sessionId);
    return;
  }
  return invoke<void>("parked_dismiss", { sessionId });
}

/** Re-reads an Estimated reset ("check now"). */
export async function parkedCheckNow(sessionId: string): Promise<ParkedInfo> {
  if (!hasBackend) {
    parkCall(`check:${sessionId}`);
    const seeded = parkSeams().__AIJ_PARKED_CHECK__;
    if (seeded) return seeded;
    throw new Error("still estimated");
  }
  return invoke<ParkedInfo>("parked_check_now", { sessionId });
}

/** Resumes a parked Instance — the original session, on its Account. */
export async function parkedResume(sessionId: string): Promise<{ session_id: string; pid: number } | null> {
  if (!hasBackend) {
    parkCall(`resume:${sessionId}`);
    return null;
  }
  return invoke<{ session_id: string; pid: number }>("parked_resume", { sessionId });
}

// --- AJ-E1004: the morning summary ------------------------------------------

/** What happened since the user last looked (severity-ordered). */
export async function overnightSummary(): Promise<OvernightSummaryInfo> {
  if (!hasBackend) {
    const seeded = (window as unknown as { __AIJ_SUMMARY__?: OvernightSummaryInfo }).__AIJ_SUMMARY__;
    return seeded ?? { since_ms: Date.now(), generated_ms: Date.now(), rows: [], counts: {}, leftovers: 0 };
  }
  return invoke<OvernightSummaryInfo>("overnight_summary", { sinceMs: null });
}

/** The user looked: the next summary starts from now. */
export async function overnightSummarySeen(): Promise<void> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_SUMMARY__?: OvernightSummaryInfo; __AIJ_SUMMARY_SEEN__?: number };
    w.__AIJ_SUMMARY_SEEN__ = (w.__AIJ_SUMMARY_SEEN__ ?? 0) + 1;
    delete w.__AIJ_SUMMARY__;
    return;
  }
  return invoke<void>("overnight_summary_seen");
}

// --- AJ-E1005: morning triage ----------------------------------------------

/** Marks an ended Instance reviewed: it leaves the triage's finished
 * list, and the step is kept in its history. */
export async function instanceMarkReviewed(sessionId: string): Promise<number> {
  if (!hasBackend) {
    const w = window as unknown as { __AIJ_TRIAGE_CALLS__?: string[] };
    w.__AIJ_TRIAGE_CALLS__ = [...(w.__AIJ_TRIAGE_CALLS__ ?? []), `reviewed:${sessionId}`];
    const row = MOCK_SESSIONS.find((session) => session.id === sessionId);
    const at = Date.now();
    if (row) {
      row.lifecycle = "reviewed";
      row.lifecycle_at_ms = at;
    }
    return at;
  }
  return invoke<number>("instance_mark_reviewed", { sessionId });
}

// --- AJ-E1007: session lifecycle — search, archive, replay, retention ------

interface HistorySeams {
  __AIJ_HISTORY_CALLS__?: string[];
  __AIJ_TRANSCRIPT_HITS__?: TranscriptHitInfo[];
  __AIJ_REPLAY__?: ReplayInfo;
  __AIJ_DELETE_PREVIEW__?: DeletePreviewInfo;
  __AIJ_ACCOUNT_STORAGE__?: AccountStorageInfo[];
  __AIJ_RETENTION_PREVIEW__?: RetentionPreviewInfo;
}
const historySeams = () => window as unknown as HistorySeams;
const historyCall = (call: string) => {
  const seams = historySeams();
  seams.__AIJ_HISTORY_CALLS__ = [...(seams.__AIJ_HISTORY_CALLS__ ?? []), call];
};
const setMockLifecycle = (sessionId: string, event: string) => {
  const row = MOCK_SESSIONS.find((session) => session.id === sessionId);
  if (row) {
    row.lifecycle = event;
    row.lifecycle_at_ms = Date.now();
  }
};

/** Archives an ended Instance: out of the active views, nothing deleted. */
export async function instanceArchive(sessionId: string): Promise<number> {
  if (!hasBackend) {
    historyCall(`archive:${sessionId}`);
    setMockLifecycle(sessionId, "archived");
    return Date.now();
  }
  return invoke<number>("instance_archive", { sessionId });
}

/** Unarchives an Instance — back exactly as it was. */
export async function instanceUnarchive(sessionId: string): Promise<number> {
  if (!hasBackend) {
    historyCall(`unarchive:${sessionId}`);
    setMockLifecycle(sessionId, "unarchived");
    return Date.now();
  }
  return invoke<number>("instance_unarchive", { sessionId });
}

/** What deleting an Instance would remove and keep (and its token). */
export async function instanceDeletePreview(sessionId: string): Promise<DeletePreviewInfo> {
  if (!hasBackend) {
    historyCall(`delete-preview:${sessionId}`);
    return (
      historySeams().__AIJ_DELETE_PREVIEW__ ?? {
        session_id: sessionId,
        account_label: "Work Max",
        refused: null,
        removes: ["Its record in AI Juggler", "Its history: 4 events", "Its search index: 120 lines"],
        keeps: ["The Provider's own transcript file", "The process and cleanup audit", "Its usage in your usage totals"],
        token: "mock-token",
      }
    );
  }
  return invoke<DeletePreviewInfo>("instance_delete_preview", { sessionId });
}

/** Deletes an Instance's AI Juggler records (with its preview's token). */
export async function instanceDelete(sessionId: string, token: string): Promise<void> {
  if (!hasBackend) {
    historyCall(`delete:${sessionId}:${token}`);
    const at = MOCK_SESSIONS.findIndex((session) => session.id === sessionId);
    if (at >= 0) MOCK_SESSIONS.splice(at, 1);
    return;
  }
  return invoke<void>("instance_delete", { sessionId, token });
}

/** Transcript text search (local index). */
export async function transcriptSearch(query: string): Promise<TranscriptHitInfo[]> {
  if (!hasBackend) {
    historyCall(`search:${query}`);
    const needle = query.trim().toLowerCase();
    return (historySeams().__AIJ_TRANSCRIPT_HITS__ ?? []).filter((hit) => hit.snippet.toLowerCase().includes(needle));
  }
  return invoke<TranscriptHitInfo[]>("transcript_search", { query });
}

/** The read-only replay of an ended or archived Instance. */
export async function instanceReplay(sessionId: string): Promise<ReplayInfo> {
  if (!hasBackend) {
    historyCall(`replay:${sessionId}`);
    const seeded = historySeams().__AIJ_REPLAY__;
    if (seeded) return { ...seeded, session_id: sessionId };
    throw new Error("No transcript recorded for this session");
  }
  return invoke<ReplayInfo>("instance_replay", { sessionId });
}

/** Every Account's retention policy and storage used. */
export async function accountStorage(): Promise<AccountStorageInfo[]> {
  if (!hasBackend) {
    const seeded = historySeams().__AIJ_ACCOUNT_STORAGE__;
    if (seeded) return seeded.map((row) => ({ ...row }));
    return MOCK_ACCOUNTS.map((account) => {
      const rows = MOCK_SESSIONS.filter((session) => session.account_id === account.id);
      return {
        account_id: account.id,
        label: account.label,
        retention_days: null,
        instances: rows.length,
        history_events: rows.length * 3,
        index_lines: 0,
        transcript_bytes: 0,
      };
    });
  }
  return invoke<AccountStorageInfo[]>("account_storage");
}

/** Sets an Account's retention (null = keep forever). */
export async function retentionSet(accountId: string, days: number | null): Promise<void> {
  if (!hasBackend) {
    historyCall(`retention:${accountId}:${days ?? "forever"}`);
    const row = (historySeams().__AIJ_ACCOUNT_STORAGE__ ?? []).find((entry) => entry.account_id === accountId);
    if (row) row.retention_days = days;
    return;
  }
  return invoke<void>("retention_set", { accountId, days });
}

/** The retention dry run (nothing is deleted). */
export async function retentionPreview(accountId: string): Promise<RetentionPreviewInfo> {
  if (!hasBackend) {
    historyCall(`retention-preview:${accountId}`);
    return (
      historySeams().__AIJ_RETENTION_PREVIEW__ ?? {
        account_id: accountId,
        retention_days: null,
        deletes: [],
        kept: [],
        keeps: [],
        token: null,
      }
    );
  }
  return invoke<RetentionPreviewInfo>("retention_preview", { accountId });
}

/** Applies the retention cleanup its dry run listed. */
export async function retentionApply(accountId: string, token: string): Promise<number> {
  if (!hasBackend) {
    historyCall(`retention-apply:${accountId}:${token}`);
    return historySeams().__AIJ_RETENTION_PREVIEW__?.deletes.length ?? 0;
  }
  return invoke<number>("retention_apply", { accountId, token });
}

// --- AJ-E1009: backup and restore -------------------------------------------

interface BackupSeams {
  __AIJ_BACKUP_CALLS__?: string[];
  __AIJ_BACKUP_DEST__?: string | null;
  __AIJ_BACKUP_FILE__?: string | null;
  __AIJ_BACKUP_EXPORT__?: BackupExportInfo;
  __AIJ_BACKUP_EXPORT_ERROR__?: string;
  __AIJ_RESTORE_PLAN__?: RestorePlanInfo;
  __AIJ_RESTORE_PLAN_ERROR__?: string;
  __AIJ_RESTORE_RESULT__?: RestoreResultInfo;
  __AIJ_BACKUP_PROGRESS__?: Array<[number, number]>;
  __AIJ_BACKUP_HOLD__?: boolean;
}
const backupSeams = () => window as unknown as BackupSeams;
const backupCall = (call: string) => {
  const seams = backupSeams();
  seams.__AIJ_BACKUP_CALLS__ = [...(seams.__AIJ_BACKUP_CALLS__ ?? []), call];
};
let mockProgress: ((done: number, total: number) => void) | null = null;

/** Where to save a backup (native save sheet); null when cancelled. */
export async function pickBackupDestination(): Promise<string | null> {
  if (!hasBackend) return backupSeams().__AIJ_BACKUP_DEST__ ?? null;
  const day = new Date().toISOString().slice(0, 10);
  const chosen = await saveDialog({
    defaultPath: `AI Juggler backup ${day}.ajbackup`,
    filters: [{ name: "AI Juggler backup", extensions: ["ajbackup"] }],
  });
  return typeof chosen === "string" ? chosen : null;
}

/** Which backup to restore (native open sheet); null when cancelled. */
export async function pickBackupFile(): Promise<string | null> {
  if (!hasBackend) return backupSeams().__AIJ_BACKUP_FILE__ ?? null;
  const chosen = await openDialog({
    multiple: false,
    directory: false,
    filters: [{ name: "AI Juggler backup", extensions: ["ajbackup"] }],
  });
  return typeof chosen === "string" ? chosen : null;
}

/** Export progress (bytes done, bytes total). */
export function onBackupProgress(handler: (done: number, total: number) => void): () => void {
  if (!hasBackend) {
    mockProgress = handler;
    return () => {
      if (mockProgress === handler) mockProgress = null;
    };
  }
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<{ done: number; total: number }>(EVENTS.backupProgress, (event) => handler(event.payload.done, event.payload.total))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {});
  return () => {
    disposed = true;
    unlisten?.();
  };
}

/** Writes a backup of AI Juggler's own data to `dest`. */
export async function backupExport(dest: string): Promise<BackupExportInfo> {
  if (!hasBackend) {
    backupCall(`export:${dest}`);
    const seams = backupSeams();
    for (const [done, total] of seams.__AIJ_BACKUP_PROGRESS__ ?? [[1, 1]]) mockProgress?.(done, total);
    if (seams.__AIJ_BACKUP_HOLD__) return new Promise<BackupExportInfo>(() => {});
    if (seams.__AIJ_BACKUP_EXPORT_ERROR__) throw new Error(seams.__AIJ_BACKUP_EXPORT_ERROR__);
    if (seams.__AIJ_BACKUP_EXPORT__) return { ...seams.__AIJ_BACKUP_EXPORT__, path: dest };
    throw new Error("No backend in this preview");
  }
  return invoke<BackupExportInfo>("backup_export", { dest });
}

/** Verifies a backup in full; returns the restore plan (nothing changes yet). */
export async function backupVerify(path: string): Promise<RestorePlanInfo> {
  if (!hasBackend) {
    backupCall(`verify:${path}`);
    const seams = backupSeams();
    if (seams.__AIJ_RESTORE_PLAN_ERROR__) throw new Error(seams.__AIJ_RESTORE_PLAN_ERROR__);
    if (seams.__AIJ_RESTORE_PLAN__) return { ...seams.__AIJ_RESTORE_PLAN__, path };
    throw new Error("No backend in this preview");
  }
  return invoke<RestorePlanInfo>("backup_verify", { path });
}

/** Applies a verified restore (with its plan's token). */
export async function backupRestore(path: string, token: string): Promise<RestoreResultInfo> {
  if (!hasBackend) {
    backupCall(`restore:${path}:${token}`);
    const result = backupSeams().__AIJ_RESTORE_RESULT__;
    if (result) return result;
    throw new Error("No backend in this preview");
  }
  return invoke<RestoreResultInfo>("backup_restore", { path, token });
}

// --- AJ-E1010: updates ---------------------------------------------------------

interface UpdateSeams {
  __AIJ_UPDATE_STATUS__?: UpdateStatusInfo;
  __AIJ_UPDATE_CALLS__?: string[];
  __AIJ_UPDATE_AFTER_CHECK__?: UpdateStatusInfo;
  __AIJ_UPDATE_AFTER_INSTALL__?: UpdateStatusInfo;
  __AIJ_UPDATE_RESTART_ERROR__?: string;
}
const updateSeams = () => window as unknown as UpdateSeams;
const updateCall = (call: string) => {
  const seams = updateSeams();
  seams.__AIJ_UPDATE_CALLS__ = [...(seams.__AIJ_UPDATE_CALLS__ ?? []), call];
};
const mockUpdateStatus = (): UpdateStatusInfo =>
  updateSeams().__AIJ_UPDATE_STATUS__ ?? {
    current_version: "0.0.1",
    channel: "beta",
    edition: "main",
    app_name: "AI Juggler",
    channels: ["stable", "beta"],
    configured: false,
    check_on_launch: true,
    state: "idle",
    available: null,
    progress: null,
    error: null,
    backup_path: null,
    checked_at_ms: null,
  };

/** Where updating stands. */
export async function updateStatus(): Promise<UpdateStatusInfo> {
  if (!hasBackend) return { ...mockUpdateStatus() };
  return invoke<UpdateStatusInfo>("update_status");
}

/** Turns "check when AI Juggler starts" on or off. */
export async function updateSetCheckOnLaunch(enabled: boolean): Promise<UpdateStatusInfo> {
  if (!hasBackend) {
    updateCall(`check-on-launch:${enabled}`);
    const next = { ...mockUpdateStatus(), check_on_launch: enabled };
    updateSeams().__AIJ_UPDATE_STATUS__ = next;
    return next;
  }
  return invoke<UpdateStatusInfo>("update_set_check_on_launch", { enabled });
}

/** D-20: the update channel this install follows. AI Juggler offers
 * stable and beta; AI Juggler Nightly follows nightly (D-23). */
export type UpdateChannel = "stable" | "beta" | "nightly";

/** Chooses the update channel; forgets what the previous feed announced. */
export async function updateSetChannel(channel: UpdateChannel): Promise<UpdateStatusInfo> {
  if (!hasBackend) {
    updateCall(`channel:${channel}`);
    const current = mockUpdateStatus();
    const next: UpdateStatusInfo = {
      ...current,
      channel,
      ...(current.state === "installed" ? {} : { state: "idle", available: null, error: null }),
    };
    updateSeams().__AIJ_UPDATE_STATUS__ = next;
    return next;
  }
  return invoke<UpdateStatusInfo>("update_set_channel", { channel });
}

/** Checks for a newer version now (never installs). */
export async function updateCheck(): Promise<UpdateStatusInfo> {
  if (!hasBackend) {
    updateCall("check");
    const next = updateSeams().__AIJ_UPDATE_AFTER_CHECK__ ?? mockUpdateStatus();
    updateSeams().__AIJ_UPDATE_STATUS__ = next;
    return { ...next };
  }
  return invoke<UpdateStatusInfo>("update_check");
}

/** Installs the announced update: backup, download, verify, install. */
export async function updateInstall(): Promise<UpdateStatusInfo> {
  if (!hasBackend) {
    updateCall("install");
    const next = updateSeams().__AIJ_UPDATE_AFTER_INSTALL__ ?? mockUpdateStatus();
    updateSeams().__AIJ_UPDATE_STATUS__ = next;
    return { ...next };
  }
  return invoke<UpdateStatusInfo>("update_install");
}

/** Restarts into the installed version (refused while Instances run). */
export async function updateRestart(): Promise<void> {
  if (!hasBackend) {
    updateCall("restart");
    const error = updateSeams().__AIJ_UPDATE_RESTART_ERROR__;
    if (error) throw new Error(error);
    return;
  }
  return invoke<void>("update_restart");
}

/** Update status changes pushed by the backend. */
export function onUpdateStatus(handler: (status: UpdateStatusInfo) => void): () => void {
  if (!hasBackend) return () => {};
  let unlisten: (() => void) | null = null;
  let disposed = false;
  listen<UpdateStatusInfo>(EVENTS.updateStatus, (event) => handler(event.payload))
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {});
  return () => {
    disposed = true;
    unlisten?.();
  };
}

// --- AJ-E1204: the backup vault ---------------------------------------------

interface VaultSeams {
  __AIJ_VAULT_STATUS__?: VaultStatusInfo;
  __AIJ_VAULT_CODE__?: string;
  __AIJ_VAULT_CALLS__?: string[];
  __AIJ_VAULT_SAVE_DEST__?: string | null;
}
const vaultSeams = () => window as unknown as VaultSeams;
const vaultCall = (call: string) => {
  const seams = vaultSeams();
  seams.__AIJ_VAULT_CALLS__ = [...(seams.__AIJ_VAULT_CALLS__ ?? []), call];
};

/** Where the vault stands on this Mac. */
export async function vaultStatus(): Promise<VaultStatusInfo> {
  // The preview's default is a Mac whose code is already saved; specs seed
  // the first-time state explicitly.
  if (!hasBackend) return { ...(vaultSeams().__AIJ_VAULT_STATUS__ ?? { has_key: true, code_saved: true }) };
  return invoke<VaultStatusInfo>("vault_status");
}

/** Makes the vault key (first time) and returns its recovery code. */
export async function vaultSetup(): Promise<RecoveryCodeInfo> {
  if (!hasBackend) {
    vaultCall("setup");
    const seams = vaultSeams();
    seams.__AIJ_VAULT_STATUS__ = { has_key: true, code_saved: false };
    return { code: seams.__AIJ_VAULT_CODE__ ?? "ABCD-EFGH-IJKL-MNOP-QRST-UVWX-YZ23-4567-ABCD-EFGH-IJKL-MNOP-QRST-UVWX" };
  }
  return invoke<RecoveryCodeInfo>("vault_setup");
}

/** "I've saved my recovery code". */
export async function vaultConfirmSaved(): Promise<VaultStatusInfo> {
  if (!hasBackend) {
    vaultCall("confirm");
    const next = { has_key: true, code_saved: true };
    vaultSeams().__AIJ_VAULT_STATUS__ = next;
    return next;
  }
  return invoke<VaultStatusInfo>("vault_confirm_saved");
}

/** Saves the recovery code to a text file the user picks; false if cancelled. */
export async function vaultSaveCode(): Promise<boolean> {
  let dest: string | null;
  if (!hasBackend) {
    dest = vaultSeams().__AIJ_VAULT_SAVE_DEST__ ?? null;
  } else {
    const chosen = await saveDialog({
      defaultPath: "AI Juggler recovery code.txt",
      filters: [{ name: "Text", extensions: ["txt"] }],
    });
    dest = typeof chosen === "string" ? chosen : null;
  }
  if (!dest) return false;
  if (!hasBackend) {
    vaultCall(`save:${dest}`);
    return true;
  }
  await invoke<void>("vault_save_code", { dest });
  return true;
}

/** A new Mac enters its recovery code. */
export async function vaultEnterCode(code: string): Promise<VaultStatusInfo> {
  if (!hasBackend) {
    vaultCall(`enter:${code}`);
    const next = { has_key: true, code_saved: true };
    vaultSeams().__AIJ_VAULT_STATUS__ = next;
    return next;
  }
  return invoke<VaultStatusInfo>("vault_enter_code", { code });
}

// --- AJ-E1205: Cloud backup ----------------------------------------------------

interface CloudBackupSeams {
  __AIJ_CLOUD_BACKUP__?: CloudBackupStatusInfo;
  __AIJ_CLOUD_BACKUPS__?: CloudBackupInfo[];
  __AIJ_CLOUD_BACKUP_CALLS__?: string[];
  __AIJ_CLOUD_BACKUP_ERROR__?: string;
  __AIJ_CLOUD_RESTORE_PLAN__?: RestorePlanInfo;
  __AIJ_CLOUD_RESTORE_ERROR__?: string;
  __AIJ_CLOUD_BACKUP_HOLD__?: boolean;
}
const cloudSeams = () => window as unknown as CloudBackupSeams;
const cloudCall = (call: string) => {
  const seams = cloudSeams();
  seams.__AIJ_CLOUD_BACKUP_CALLS__ = [...(seams.__AIJ_CLOUD_BACKUP_CALLS__ ?? []), call];
};
const mockCloud = (): CloudBackupStatusInfo =>
  cloudSeams().__AIJ_CLOUD_BACKUP__ ?? {
    enabled: true,
    code_saved: true,
    schedule: "weekly",
    last: null,
    uploading: false,
    limit_bytes: 25 * 1024 * 1024,
  };

/** Where Cloud backup stands. */
export async function cloudBackupStatus(): Promise<CloudBackupStatusInfo> {
  if (!hasBackend) return { ...mockCloud() };
  return invoke<CloudBackupStatusInfo>("cloud_backup_status");
}

/** Sets the schedule (off | daily | weekly). */
export async function cloudBackupSetSchedule(schedule: string): Promise<CloudBackupStatusInfo> {
  if (!hasBackend) {
    cloudCall(`schedule:${schedule}`);
    const next = { ...mockCloud(), schedule };
    cloudSeams().__AIJ_CLOUD_BACKUP__ = next;
    return next;
  }
  return invoke<CloudBackupStatusInfo>("cloud_backup_set_schedule", { schedule });
}

/** Backs up now (builds, seals and uploads). */
export async function cloudBackupNow(): Promise<LastBackupInfo> {
  if (!hasBackend) {
    cloudCall("now");
    const seams = cloudSeams();
    if (seams.__AIJ_CLOUD_BACKUP_HOLD__) return new Promise<LastBackupInfo>(() => {});
    if (seams.__AIJ_CLOUD_BACKUP_ERROR__) throw new Error(seams.__AIJ_CLOUD_BACKUP_ERROR__);
    const last = { at_ms: Date.now(), ok: true, size_bytes: 412_000, error: null };
    seams.__AIJ_CLOUD_BACKUP__ = { ...mockCloud(), last };
    return last;
  }
  return invoke<LastBackupInfo>("cloud_backup_now");
}

/** The account's backups, newest first. */
export async function cloudBackupList(): Promise<CloudBackupInfo[]> {
  if (!hasBackend) {
    cloudCall("list");
    return [...(cloudSeams().__AIJ_CLOUD_BACKUPS__ ?? [])];
  }
  return invoke<CloudBackupInfo[]>("cloud_backup_list");
}

/** Downloads and opens one backup; returns the verified restore plan. */
export async function cloudBackupPrepareRestore(id: string, recoveryCode: string | null): Promise<RestorePlanInfo> {
  if (!hasBackend) {
    cloudCall(`prepare:${id}:${recoveryCode ? "code" : "key"}`);
    const seams = cloudSeams();
    if (seams.__AIJ_CLOUD_RESTORE_ERROR__) throw new Error(seams.__AIJ_CLOUD_RESTORE_ERROR__);
    if (seams.__AIJ_CLOUD_RESTORE_PLAN__) return seams.__AIJ_CLOUD_RESTORE_PLAN__;
    throw new Error("No backend in this preview");
  }
  return invoke<RestorePlanInfo>("cloud_backup_prepare_restore", { id, recoveryCode });
}

// --- AJ-E1206: Fleet discovery via account ------------------------------------

interface AccountMacSeams {
  __AIJ_ACCOUNT_MACS__?: AccountMacsInfo;
  __AIJ_ACCOUNT_MACS_CALLS__?: string[];
}
const accountMacSeams = () => window as unknown as AccountMacSeams;

/** Your other linked Macs (Fleet discovery on; at most one read a minute). */
export async function fleetAccountMacs(): Promise<AccountMacsInfo> {
  if (!hasBackend) {
    const seams = accountMacSeams();
    seams.__AIJ_ACCOUNT_MACS_CALLS__ = [...(seams.__AIJ_ACCOUNT_MACS_CALLS__ ?? []), "list"];
    const seeded = seams.__AIJ_ACCOUNT_MACS__;
    if (seeded) return { ...seeded, macs: seeded.macs.map((mac) => ({ ...mac })) };
    throw new Error("Fleet discovery is off");
  }
  return invoke<AccountMacsInfo>("fleet_account_macs");
}

/** Pins a listed Mac after its fingerprint was confirmed here. */
export async function fleetAccountPin(deviceId: string, fingerprint: string): Promise<void> {
  if (!hasBackend) {
    const seams = accountMacSeams();
    seams.__AIJ_ACCOUNT_MACS_CALLS__ = [...(seams.__AIJ_ACCOUNT_MACS_CALLS__ ?? []), `pin:${deviceId}:${fingerprint}`];
    const mac = seams.__AIJ_ACCOUNT_MACS__?.macs.find((row) => row.device_id === deviceId);
    if (mac) mac.state = "waiting";
    return;
  }
  return invoke<void>("fleet_account_pin", { deviceId, fingerprint });
}
