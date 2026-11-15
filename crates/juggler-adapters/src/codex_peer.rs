//! AJ-E3013 (D-116, D-119): a Codex app thread can't be attested yet.
//!
//! The Codex app runs every thread in one `codex app-server`, so an MCP
//! server's parent says nothing about which thread is calling, and Codex
//! starts its MCP servers with a small environment that names no thread
//! (`docs/research/desktop-pool-members.md` §4.4). Until a lab probe proves
//! a way (request metadata, or the thread's shell), every keyless call
//! from it is refused, and its threads can't be woken (no per-thread
//! inbox). A Codex app thread stays Watched in 0.4.6 (D-119).

use juggler_adapter::peer::{DesktopPeer, DesktopPlace, PeerVerdict, ProcStep};

/// The Codex app's half of the attestation: none yet.
pub struct CodexAppPeer;

impl DesktopPeer for CodexAppPeer {
    fn attest(&self, _place: &DesktopPlace<'_>, _chain: &[ProcStep]) -> PeerVerdict {
        PeerVerdict::no(
            "a Codex app thread can't be told apart yet: every thread's tools run under one app-server",
        )
    }

    fn can_wake(&self) -> bool {
        false
    }

    fn attests(&self) -> bool {
        false
    }

    /// The coordinating session's ruling, 2026-11-15: no Add for the Codex
    /// app until lab probe C2 shows which thread calls.
    fn not_offered(&self) -> Option<String> {
        Some("Codex can't yet tell which thread is calling".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_codex_app_thread_is_never_attested_nor_woken() {
        let dir = std::path::Path::new("/nonexistent");
        let place = DesktopPlace {
            config_dir: dir,
            instance_dir: dir,
        };
        let chain = [
            ProcStep {
                pid: 2,
                start_secs: 2,
            },
            ProcStep {
                pid: 1,
                start_secs: 1,
            },
        ];
        assert!(matches!(
            CodexAppPeer.attest(&place, &chain),
            PeerVerdict::Unattested { .. }
        ));
        assert!(!CodexAppPeer.can_wake());
        assert!(!CodexAppPeer.attests());
        assert!(
            CodexAppPeer.not_offered().is_some(),
            "no Add until probe C2"
        );
    }
}
