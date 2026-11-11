/** The design preview's tool installs, capabilities and environment scan (moved from lib/ipc/providers.ts, AJ-E2018). */
import type { ProviderCapabilityData, ProviderInstallData, ScanReportData } from "../lib/ipc/providers";

/** The no-backend Step-0 default (preview mode shows an installed
 * Claude + a missing second Provider — both gate states visible). */
export const DEFAULT_PROVIDER_INSTALLS: ProviderInstallData[] = [
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

export const DEFAULT_PROVIDER_CAPABILITIES: ProviderCapabilityData[] = [
  { provider: "claude-code", can_use_in_place: true, reports_limits: true },
  // Its rollouts carry its own limit readings (the 0.0.12 report).
  { provider: "codex", can_use_in_place: true, reports_limits: true },
];

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
