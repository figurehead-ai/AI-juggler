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
        /// Exposes a structured protocol server (e.g. Codex app-server).
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
