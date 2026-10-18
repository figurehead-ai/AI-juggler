//! Concrete harness adapters for AI Juggler.
//!
//! One module per harness — `claude_code`, `codex`, `zcode` — plus `fake`, the
//! scriptable fake harness that drives lifecycle, stuck-detection, and
//! crash-recovery tests across the whole system. Every adapter implements the
//! `juggler_adapter::HarnessAdapter` contract and confines itself to its
//! managed account directories (guardrails in ADR-0003).
//! Requirements: `docs/SPEC.md` §4.1–§4.3.

#![cfg_attr(
    test,
    allow(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)
)]

pub mod claude_code;
pub mod claude_environment;
pub mod claude_handover;
pub mod claude_transcript;
pub mod fake;
pub mod inference;
pub mod quota_ingest;
pub mod scan_signals;

pub use claude_code::{CLAUDE_CODE_ID, ClaudeCodeAdapter, LoginState};
pub use claude_transcript::{
    MessageRole, ParsedMessage, ParsedTranscript, dedupe_messages, parse_transcript,
};
pub use fake::{FAKE_HARNESS_ID, FakeHarnessAdapter};
pub use quota_ingest::{
    SourceHealth, TailOutcome, install_statusline, rollout_events, rotate_spool_if_needed,
    spool_path, statusline_samples, tail_rollout, tail_spool, tail_transcript,
    uninstall_statusline, zcode_events, zcode_is_peak, zcode_multiplier,
};
pub mod fake_desktop;
pub use fake_desktop::{FAKE_DESKTOP_ID, FakeDesktopAdapter, SINGLE_ACCOUNT_LABEL};
