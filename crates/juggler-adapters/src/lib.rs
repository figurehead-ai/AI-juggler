//! Concrete harness adapters for AI Juggler.
//!
//! One module per harness — `claude_code`, `codex`, `opencode` — plus `fake`, the
//! scriptable fake harness that drives lifecycle, stuck-detection, and
//! crash-recovery tests across the whole system. Every adapter implements the
//! `juggler_adapter::HarnessAdapter` contract and confines itself to its
//! managed account directories (guardrails in ADR-0003).
//! Requirements: `docs/SPEC.md` §4.1–§4.3.

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

pub mod activity;
pub mod claude_asks;
pub mod claude_bridge;
pub mod claude_code;
pub mod claude_conversations;
mod claude_endpoint;
pub mod claude_environment;
pub mod claude_handover;
pub mod claude_hooks;
pub mod claude_inventory;
pub mod claude_live;
pub mod claude_stream;
pub mod claude_transcript;
pub mod codex;
pub mod codex_appserver;
pub mod codex_bridge;
pub mod codex_conversations;
mod codex_endpoint;
pub mod codex_environment;
pub mod codex_handover;
pub mod codex_inventory;
pub mod codex_live;
pub mod codex_transcript;
pub mod codex_trust;
pub mod config_edit;
pub mod fake;
pub mod fake_driven;
pub mod grok_build;
pub mod grok_environment;
pub mod grok_inventory;
pub mod grok_sessions;
pub mod inference;
pub mod inventory_files;
pub mod kimi_code;
pub mod kimi_environment;
pub mod kimi_inventory;
pub mod kimi_sessions;
pub mod last_turns;
pub mod opencode;
pub mod opencode_environment;
pub mod opencode_inventory;
pub mod opencode_store;
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

pub use activity::{ActivityLog, ActivityTail};
pub use claude_code::{CLAUDE_CODE_ID, ClaudeCodeAdapter, LoginState, claude_code_spec};
pub use claude_transcript::{
    MessageRole, ParsedMessage, ParsedTranscript, dedupe_messages, parse_transcript,
};
pub use codex::{CODEX_ID, CodexAdapter, codex_spec};
pub use fake::{FAKE_HARNESS_ID, FakeHarnessAdapter};
pub use grok_build::{GROK_BUILD_ID, GrokBuildAdapter, grok_build_spec};
pub use kimi_code::{KIMI_CODE_ID, KimiCodeAdapter, kimi_code_spec};
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
pub mod fake_desktop;
#[cfg(test)]
mod stream_equivalence;
pub use fake_desktop::{FAKE_DESKTOP_ID, FakeDesktopAdapter, SINGLE_ACCOUNT_LABEL};
