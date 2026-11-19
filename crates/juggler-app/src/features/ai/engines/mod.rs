//! ADR-0034 §2 (AJ-E3802): the engines, the only place that names one.
//! The router and the jobs see [`Card`]s; [`Registry`] holds every engine
//! in its fixed order (the router's last tie-break) and builds the facts
//! about this Mac the recommendation reads (AJ-E3811).
//!
//! | Engine | State in this build |
//! |---|---|
//! | [`apple::Apple`], Apple's on-device model | real, behind the Swift helper |
//! | [`local::Local`], downloaded GGUF files | real, behind the llama.cpp helper (D-131) |
//! | [`server::Server`], the user's own server | a stub, last to build |
//!
//! Windows and Linux compile all three: the OS seam
//! ([`juggler_platform::ai`]) says there is no system model there, so the
//! Apple helper is never started; the local engine has no helper there
//! yet, and the server stub serves nothing (D-52).

pub mod apple;
pub mod helper;
pub mod local;
pub mod server;

use std::path::Path;

use juggler_platform::ai::Chip as HostCpu;

use super::engine::{AiEngine, Card, Draft, EngineError, Request, Stop};
use super::models::recommend::{HostChip, MacFacts};
use super::models::{Chip, ModelEntry};

/// Every engine, in the router's order.
#[derive(Debug)]
pub struct Registry {
    apple: apple::Apple,
    local: local::Local,
    server: server::Server,
}

impl Registry {
    /// The engines of this build, nothing started.
    #[must_use]
    pub fn new() -> Self {
        Self::with(apple::Apple::new(), local::Local::default())
    }

    /// The engines with these Apple and local engines (tests).
    #[must_use]
    pub fn with(apple: apple::Apple, local: local::Local) -> Self {
        Self {
            apple,
            local,
            server: server::Server,
        }
    }

    /// Every engine, in order.
    #[must_use]
    pub fn engines(&self) -> [&dyn AiEngine; 3] {
        [&self.apple, &self.local, &self.server]
    }

    /// Every engine's cards for `models`, in order: pure, no I/O.
    #[must_use]
    pub fn cards(&self, models: &[ModelEntry]) -> Vec<Card> {
        self.engines()
            .iter()
            .enumerate()
            .flat_map(|(engine, it)| it.cards(engine, models))
            .collect()
    }

    /// Asks every engine again (may start a short-lived helper).
    pub fn probe(&self) {
        for engine in self.engines() {
            engine.probe();
        }
    }

    /// One generation on `card`'s engine.
    ///
    /// # Errors
    ///
    /// The engine's [`EngineError`].
    pub fn respond(
        &self,
        card: &Card,
        request: &Request,
        stop: &Stop,
    ) -> Result<Draft, EngineError> {
        self.engines()
            .get(card.engine)
            .ok_or(EngineError::Unsupported)?
            .respond(&card.model, request, stop)
    }

    /// Lets every engine go of what it holds.
    pub fn release(&self) {
        for engine in self.engines() {
            engine.release();
        }
    }

    /// This Mac's facts for the recommendation (AJ-E3811), read through
    /// the OS seams: the chip, the memory, the free disk where AI files
    /// would live (`data_dir`'s disk), and Apple Intelligence as the last
    /// probe saw it. Unknown readings fail closed (no memory, no disk).
    #[must_use]
    pub fn mac_facts(&self, data_dir: &Path) -> MacFacts {
        let machine = juggler_platform::ai::machine();
        MacFacts {
            chip: host_chip(machine.chip),
            memory_gb: machine
                .memory_bytes
                .map_or(0, |bytes| bytes / 1_000_000_000),
            free_disk_bytes: juggler_platform::room::free_space(data_dir).unwrap_or(0),
            apple_ai: self.apple.seen().apple_ai(),
        }
    }
}

/// The network path for the model download's pause (D-52): the Apple
/// helper's reading on macOS; `None` on other systems, or when it can't
/// tell.
#[must_use]
pub fn network_path() -> Option<apple::NetworkPath> {
    if cfg!(target_os = "macos") {
        apple::network_path(None)
    } else {
        None
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// The OS seam's chip as the recommendation's: a generation past the
/// newest this build knows counts as newer than all.
fn host_chip(cpu: HostCpu) -> HostChip {
    match cpu {
        HostCpu::Other => HostChip::Intel,
        HostCpu::AppleSilicon(None) => HostChip::AppleNewer,
        HostCpu::AppleSilicon(Some(n)) => Chip::ALL
            .get(usize::from(n.saturating_sub(1)))
            .map_or(HostChip::AppleNewer, |(chip, _)| HostChip::Apple(*chip)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chip_the_build_doesnt_know_is_newer_than_all() {
        assert_eq!(host_chip(HostCpu::Other), HostChip::Intel);
        assert_eq!(
            host_chip(HostCpu::AppleSilicon(Some(1))),
            HostChip::Apple(Chip::M1)
        );
        assert_eq!(
            host_chip(HostCpu::AppleSilicon(Some(5))),
            HostChip::Apple(Chip::M5)
        );
        assert_eq!(
            host_chip(HostCpu::AppleSilicon(Some(9))),
            HostChip::AppleNewer
        );
        assert_eq!(host_chip(HostCpu::AppleSilicon(None)), HostChip::AppleNewer);
    }
}
