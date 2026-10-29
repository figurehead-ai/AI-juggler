//! Capability flags for harness adapters (spec §4).

/// What a harness integration can do. Adapters advertise their set; the core
/// enables or hides features accordingly and never assumes an unadvertised
/// capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Capabilities(u32);

bitflags::bitflags! {
    impl Capabilities: u32 {
        /// Supports non-interactive single-shot runs (prompt in, result out).
        const HEADLESS = 1 << 0;
        /// Can resume a previous session natively (spec §7 tier T1).
        const RESUME = 1 << 1;
        /// Can export a full transcript in a documented format.
        const TRANSCRIPT_EXPORT = 1 << 2;
        /// Emits live transcript events as sessions write.
        const TRANSCRIPT_WATCH = 1 << 3;
        /// Per-message token usage is available locally.
        const USAGE_TOKENS = 1 << 4;
        /// A vendor quota feed exists (e.g. Claude statusline rate limits).
        const USAGE_QUOTA_FEED = 1 << 5;
        /// AJ-E2302 (ADR-0022): its sessions can be driven through the
        /// tool's own protocol (Codex's app-server, Claude Code's
        /// stream-json): `HarnessAdapter::driven` says how.
        const PROTOCOL_SERVER = 1 << 6;
        /// Sessions can be forked natively.
        const FORK = 1 << 7;
        /// Cross-account session transplant is implemented (spec §7 tier T2).
        const TRANSPLANT = 1 << 8;
        /// The provider ships a desktop-class app that can be launched
        /// and tracked as an Instance (AJ-E413).
        const DESKTOP_APP = 1 << 9;
        /// The desktop app is single-account: Juggler tracks it but
        /// cannot isolate accounts inside it (honest label, Req.1).
        const DESKTOP_SINGLE_ACCOUNT = 1 << 10;
        /// The quota feed names its own windows (e.g. Codex's
        /// `window_minutes`): the core shows the windows it reports, never
        /// ones assumed for the tool or plan (P13).
        const LIMIT_WINDOWS_REPORTED = 1 << 11;
        /// Each usage record's key names its own session (Codex:
        /// `thread/turn/response`), so no two sessions ever share one: the
        /// store keeps no record of who holds them (a resumed copy can't
        /// repeat them under another session).
        const USAGE_KEYS_PER_SESSION = 1 << 12;
        /// AJ-E1704 (D-30): its launch options can add AI Juggler's agent
        /// tools as an MCP server, pre-allowed
        /// (`HarnessAdapter::agent_bridge` says how).
        const AGENT_BRIDGE = 1 << 13;
        /// AJ-E2306 (D-40): its terminal sessions report their exact state
        /// through the tool's own hooks, from a per-launch file in AI
        /// Juggler's folder (`HarnessAdapter::state_reports` says how).
        const STATE_REPORTS = 1 << 14;
        /// 0.2: its records say which process holds a session open — a
        /// per-session file or lock naming its pid
        /// (`HarnessAdapter::live_sessions`), or the files a running
        /// session process holds open (`HarnessAdapter::held_sessions`).
        /// Without it, nothing tells a session AI Juggler started from one
        /// the user started in their own terminal on the same account, so
        /// an account used in place is never linked by its launch alone.
        const SESSION_PROCESS_RECORDED = 1 << 15;
        /// 0.3: its transcripts record the usage limit a turn stopped at,
        /// while the session stays open (`HarnessAdapter::limit_hit` reads
        /// it), so an open session can say it hit its limit.
        const LIMIT_HITS = 1 << 16;
    }
}

#[cfg(test)]
mod tests {
    use super::Capabilities;

    #[test]
    fn capabilities_combine_and_intersect() {
        let base = Capabilities::HEADLESS | Capabilities::RESUME;
        assert!(base.contains(Capabilities::HEADLESS));
        assert!(!base.contains(Capabilities::TRANSPLANT));
        let all = Capabilities::all();
        assert!(all.contains(base));
    }
}
