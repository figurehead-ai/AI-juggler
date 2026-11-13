//! Concrete harness adapters for AI Juggler.
//!
//! One module per harness — `claude_code`, `codex`, `opencode` — plus `fake`, the
//! scriptable fake harness that drives lifecycle, stuck-detection, and
//! crash-recovery tests across the whole system. Every adapter implements the
//! `juggler_adapter::HarnessAdapter` contract and confines itself to its
//! managed account directories (guardrails in ADR-0003).
//! Requirements: `docs/SPEC.md` §4.1–§4.3.
//!
//! Must not know: Tauri, the window or storage tables; and a tool's module
//! never knows a sibling tool's (shared mechanics live in the neutral
//! modules).

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

/// Words in a process's name or command line that mark it as an AI coding
/// tool's (the cleanup screen's "Harness" role, a guess). Kept here, with
/// the adapters that own the tools' ids (ADR-0019), not in shared code.
pub const HARNESS_PROCESS_HINTS: &[&str] = &["claude", "codex", "gemini", "copilot"];

pub mod activity;
pub mod claude_app_automation;
pub mod claude_app_family;
pub mod claude_asks;
pub mod claude_bridge;
pub mod claude_code;
pub mod claude_conversations;
pub mod claude_cut;
mod claude_endpoint;
mod claude_env;
pub mod claude_environment;
pub mod claude_folds;
pub mod claude_get_it;
pub mod claude_handover;
pub mod claude_hooks;
pub mod claude_inventory;
pub mod claude_limits;
pub mod claude_live;
pub mod claude_memory;
pub mod claude_move;
pub mod claude_ready;
pub mod claude_stream;
pub mod claude_subagents;
pub mod claude_tool_words;
pub mod claude_transcript;
pub mod claude_used;
pub mod codex;
pub mod codex_appserver;
pub mod codex_bridge;
pub mod codex_conversations;
pub mod codex_cut;
mod codex_endpoint;
pub mod codex_environment;
pub mod codex_family;
pub mod codex_goals;
pub mod codex_handover;
pub mod codex_inventory;
pub mod codex_live;
pub mod codex_subagents;
pub mod codex_titles;
pub mod codex_transcript;
pub mod codex_trust;
pub mod codex_turn;
pub mod codex_used;
pub(crate) mod common;
pub mod config_edit;
pub mod conversation_feed;
mod conversation_lines;
pub mod cut;
pub mod fake;
pub mod fake_driven;
pub mod folder_wake;
pub mod grok_bridge;
pub mod grok_build;
pub mod grok_environment;
pub mod grok_inventory;
pub mod grok_sessions;
pub mod grok_subagents;
pub mod inference;
mod inline_data;
pub mod inventory_files;
pub mod kimi_code;
pub mod kimi_environment;
pub mod kimi_inventory;
pub mod kimi_limits;
pub mod kimi_sessions;
pub mod kimi_subagents;
pub mod kimi_usage;
pub mod last_turns;
pub mod local_http;
pub mod muse;
pub mod muse_environment;
pub mod muse_inventory;
pub mod muse_serve;
pub mod muse_sessions;
pub mod muse_usage;
pub mod opencode;
pub mod opencode_bridge;
pub mod opencode_environment;
pub mod opencode_inventory;
pub mod opencode_store;
pub mod opencode_subagents;
pub mod other_managers;
mod presence;
pub mod quota_ingest;
pub mod scan_signals;
pub mod services;
pub mod settings_keys;
pub mod sign_in;
pub mod subtasks;
pub mod tool_summary;
pub mod transcript;
pub mod transcript_tail;
mod transcript_watcher;
pub mod zcode;
pub mod zcode_appserver;
pub mod zcode_environment;
pub mod zcode_inventory;
pub mod zcode_limits;
pub mod zcode_sessions;
pub mod zcode_usage;

pub use activity::{ActivityLog, ActivityTail};
pub use claude_code::{CLAUDE_CODE_ID, ClaudeCodeAdapter, LoginState, claude_code_spec};
pub use claude_transcript::{
    ContentCounts, MessageRole, ParsedMessage, ParsedTranscript, dedupe_messages, parse_transcript,
};
pub use codex::{CODEX_ID, CodexAdapter, codex_spec};
pub use fake::{FAKE_HARNESS_ID, FakeHarnessAdapter};
pub use grok_build::{GROK_BUILD_ID, GrokBuildAdapter, grok_build_spec};
pub use kimi_code::{KIMI_CODE_ID, KimiCodeAdapter, kimi_code_spec};
pub use muse::{MUSE_ID, MuseAdapter, muse_spec};
pub use opencode::{OPENCODE_ID, OpenCodeAdapter, opencode_spec};
pub use quota_ingest::{
    CreditsSnapshot, RolloutCarry, SourceHealth, TailOutcome, codex_rate_limits,
    ensure_launch_overlay, install_statusline, overlay_dir, overlay_spool_path, rollout_events,
    rollout_events_after, rotate_spool_if_needed, spool_path, statusline_samples, tail_rollout,
    tail_spool, tail_transcript, uninstall_statusline, zcode_events, zcode_is_peak,
    zcode_multiplier,
};
pub use subtasks::{Subtask, SubtaskLog, SubtaskState, SubtaskTail, subtasks_of};
pub use transcript::{
    BoundedLines, LINE_MAX, LinesRead, file_sha256, for_each_line, is_compressed, last_line_parses,
    parse_any, read_bounded, read_session_file, records_compaction, session_file_sha256,
    session_lines, transcript_tokens,
};
pub use zcode::{ZCODE_ID, ZCodeAdapter, zcode_spec};
pub mod fake_desktop;
#[cfg(test)]
mod stream_equivalence;
pub use fake_desktop::{FAKE_DESKTOP_ID, FakeDesktopAdapter, SINGLE_ACCOUNT_LABEL};
