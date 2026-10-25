/**
 * AJ-E1525: the tools catalog the preview and the tests run on, matching
 * what the adapters' specs serve (crates/juggler-adapters claude_code.rs
 * and codex.rs `*_spec()`). Preview data only — the app reads
 * `tools_catalog`. Tests can swap it through `window.__AIJ_TOOLS__`.
 */
import type { ToolSpecInfo } from "../types/generated/ToolSpecInfo";

export const PREVIEW_TOOLS: ToolSpecInfo[] = [
  {
    id: "claude-code",
    display_name: "Claude Code",
    short_name: "Claude",
    vendor: "Anthropic",
    mark: "claude",
    vendor_mark: "anthropic",
    install_url: "https://claude.com/download",
    install_command: "npm install -g @anthropic-ai/claude-code",
    sign_in_own: "Claude Code's own sign-in",
    sign_in_short: "Claude Code's",
    same_as: "as your Claude app",
    desktop_app: "Claude",
    host_app: "claude-app",
    kinds: ["cli", "desktop"],
    rules_files: ["CLAUDE.md"],
    scan_hint: "Your default setup, your Claude profiles and anything else that looks like one. Nothing is changed.",
    setup_groups: [
      { id: "profiles", title: "Your Claude profiles", path_contains: "/ClaudeCodeProfiles/", origin: null, shows_install: false },
      { id: "default", title: "The default setup", path_contains: null, origin: "vendor-default", shows_install: true },
      { id: "other", title: "Other Claude setups", path_contains: null, origin: null, shows_install: false },
    ],
    default_label: "Default",
    flow: { first_run: true, primary: true },
    terms: {
      name: "Anthropic's Consumer Terms",
      url: "https://www.anthropic.com/legal/consumer-terms",
      accounts:
        "For Pro and Max, the Consumer Terms say not to share an account or make it available to anyone else, and Anthropic says those plans' limits assume one person's use.",
      checked: "2026-10-23",
    },
    home_env: ["CLAUDE_CONFIG_DIR", "AI_JUGGLER_CLAUDE_BIN"],
    supports: { in_place: true, limits: true },
  },
  {
    id: "codex",
    display_name: "Codex",
    short_name: "Codex",
    vendor: "OpenAI",
    mark: "openai",
    vendor_mark: null,
    install_url: "https://developers.openai.com/codex",
    install_command: "npm install -g @openai/codex",
    sign_in_own: "Codex's own sign-in (OpenAI)",
    sign_in_short: "Codex's (OpenAI)",
    same_as: "as when you run Codex yourself",
    desktop_app: null,
    host_app: "codex-app",
    kinds: ["cli"],
    rules_files: ["AGENTS.md"],
    scan_hint: "Your default setup and anything else that looks like one. Nothing is changed.",
    setup_groups: [{ id: "codex", title: "Codex", path_contains: null, origin: null, shows_install: true }],
    default_label: "Codex",
    flow: { first_run: true, primary: false },
    terms: {
      name: "OpenAI's Terms of Use",
      url: "https://openai.com/policies/terms-of-use/",
      accounts: "OpenAI says one person can have several accounts, and its Terms of Use say not to share one or get around its usage limits.",
      checked: "2026-10-23",
    },
    home_env: ["CODEX_HOME", "CODEX_SQLITE_HOME", "AI_JUGGLER_CODEX_BIN"],
    supports: { in_place: true, limits: false },
  },
];
