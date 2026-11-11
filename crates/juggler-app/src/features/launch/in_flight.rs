//! AJ-E2907 R3: the launches still under way, per launch settings, so a
//! delete can't race one. A launch is counted from before it checks that
//! its launch settings still exist until its session is recorded (or it
//! failed); the delete refuses while any is counted, under the storage
//! lock it marks the settings deleted with. Either the delete sees the
//! launch and refuses, or the launch sees the settings deleted and stops.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

/// The launches under way, by launch settings id.
#[derive(Debug, Default)]
pub struct InFlight(Arc<Mutex<HashMap<String, usize>>>);

/// One launch under way; it stops counting when dropped.
#[derive(Debug)]
pub struct Flight {
    all: Arc<Mutex<HashMap<String, usize>>>,
    profile: String,
}

impl InFlight {
    /// A launch with `profile` begins (counted until the returned guard
    /// drops).
    #[must_use]
    pub fn begin(&self, profile: &str) -> Flight {
        *self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(profile.to_owned())
            .or_default() += 1;
        Flight {
            all: Arc::clone(&self.0),
            profile: profile.to_owned(),
        }
    }

    /// How many launches with `profile` are under way.
    #[must_use]
    pub fn count(&self, profile: &str) -> usize {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(profile)
            .copied()
            .unwrap_or(0)
    }
}

impl Drop for Flight {
    fn drop(&mut self) {
        let mut all = self.all.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = all.get_mut(&self.profile) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                all.remove(&self.profile);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_launch_counts_until_its_guard_drops() {
        let flights = InFlight::default();
        assert_eq!(flights.count("p"), 0);
        let one = flights.begin("p");
        let two = flights.begin("p");
        assert_eq!(flights.count("p"), 2);
        assert_eq!(flights.count("q"), 0);
        drop(one);
        assert_eq!(flights.count("p"), 1);
        drop(two);
        assert_eq!(flights.count("p"), 0);
    }
}
