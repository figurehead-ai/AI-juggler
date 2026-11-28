//! Idle, defined once (ADR-0039 §2): no AI job starting or answering (the
//! router's run), and the Mac neither in Low Power Mode nor showing the
//! low-battery warning (`machine_health`'s readings). `WhenIdle` work
//! runs only then, or at its deadline.

use crate::state::AppState;

/// The power facts idle reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// An AI job is starting or answering.
    pub ai_busy: bool,
    /// Low Power Mode is on.
    pub low_power_mode: bool,
    /// The low-battery warning shows.
    pub battery_low: bool,
}

/// Idle from the facts.
#[must_use]
pub const fn idle(facts: Facts) -> bool {
    !facts.ai_busy && !facts.low_power_mode && !facts.battery_low
}

/// The facts now.
#[must_use]
pub fn facts(state: &AppState) -> Facts {
    let power = state
        .machine
        .lock()
        .ok()
        .and_then(|watch| watch.sampler.snapshot().power);
    Facts {
        ai_busy: crate::features::ai::run::busy(crate::features::ai::service::shared()),
        low_power_mode: power.is_some_and(|p| p.low_power_mode),
        battery_low: power.is_some_and(|p| p.battery_low),
    }
}

/// Idle now.
#[must_use]
pub fn now(state: &AppState) -> bool {
    idle(facts(state))
}

#[cfg(test)]
mod tests {
    use super::{Facts, idle};

    #[test]
    fn idle_needs_no_ai_job_and_no_power_warning() {
        assert!(idle(Facts::default()));
        for facts in [
            Facts {
                ai_busy: true,
                ..Facts::default()
            },
            Facts {
                low_power_mode: true,
                ..Facts::default()
            },
            Facts {
                battery_low: true,
                ..Facts::default()
            },
        ] {
            assert!(!idle(facts), "{facts:?}");
        }
    }
}
