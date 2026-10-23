//! AJ-E1509 Req.2–3 — keeping the Mac working for the sessions it runs.
//!
//! - **Awake:** while sessions work, an activity that disables idle
//!   system sleep (the display may still sleep), named so
//!   `pmset -g assertions` says "AI Juggler: 2 sessions working". It
//!   never blocks a sleep the user asks for: closing the lid or the Apple
//!   menu sleeps the Mac as ever.
//! - **No App Nap:** while anything is supervised, a user-initiated
//!   activity that still allows idle sleep, so monitoring and timers
//!   aren't throttled in the background.
//!
//! Each activity is held by a thread of its own inside
//! `performActivityWithOptions:reason:usingBlock:`, the one Foundation
//! form that is safe to call (no `unsafe`; the workspace forbids it): the
//! block waits on a channel, and returns — ending the activity — when the
//! holder is dropped. On other systems nothing is held.

/// What should be held now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AwakePlan {
    /// Sessions working now: the Mac is kept awake while this is above 0
    /// and `keep_awake` is on.
    pub working: u32,
    /// Sessions supervised now: no App Nap while this is above 0.
    pub supervised: u32,
    /// The user's settings allow keeping the Mac awake now (on power,
    /// or on battery when they said so).
    pub keep_awake: bool,
}

/// The awake activity's name: what `pmset -g assertions` shows.
#[must_use]
pub fn awake_reason(working: u32) -> String {
    if working == 1 {
        "AI Juggler: 1 session working".to_owned()
    } else {
        format!("AI Juggler: {working} sessions working")
    }
}

/// Only macOS has these activities: elsewhere nothing is held, and
/// nothing claims to be.
const SUPPORTED: bool = cfg!(target_os = "macos");

/// The no-App-Nap activity's name.
pub const SUPERVISING_REASON: &str = "AI Juggler: watching your sessions";

/// The activities held for the sessions; [`AwakeKeeper::apply`] makes them
/// match a plan. Dropping it ends them.
#[derive(Default)]
pub struct AwakeKeeper {
    awake: Option<(u32, imp::Held)>,
    no_nap: Option<imp::Held>,
}

impl std::fmt::Debug for AwakeKeeper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwakeKeeper")
            .field("awake", &self.awake.as_ref().map(|(working, _)| working))
            .field("no_nap", &self.no_nap.is_some())
            .finish()
    }
}

impl AwakeKeeper {
    /// Holds what `plan` asks for and lets go of the rest. A change in
    /// the number of working sessions renames the awake activity (it is
    /// ended and taken again).
    pub fn apply(&mut self, plan: AwakePlan) {
        let want_awake = SUPPORTED && plan.keep_awake && plan.working > 0;
        match (&self.awake, want_awake) {
            (Some((working, _)), true) if *working == plan.working => {}
            (_, true) => {
                self.awake = None; // ends the old one first
                self.awake = Some((plan.working, imp::Held::awake(&awake_reason(plan.working))));
            }
            (_, false) => self.awake = None,
        }
        match (self.no_nap.is_some(), SUPPORTED && plan.supervised > 0) {
            (false, true) => self.no_nap = Some(imp::Held::no_nap(SUPERVISING_REASON)),
            (true, false) => self.no_nap = None,
            _ => {}
        }
    }

    /// Whether the Mac is being kept awake now, and for how many sessions.
    #[must_use]
    pub fn awake_for(&self) -> Option<u32> {
        self.awake.as_ref().map(|(working, _)| *working)
    }

    /// Whether App Nap is held off now.
    #[must_use]
    pub fn holding_off_app_nap(&self) -> bool {
        self.no_nap.is_some()
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::sync::mpsc;

    use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};

    /// One activity, held by its thread until this is dropped.
    pub struct Held {
        release: Option<mpsc::Sender<()>>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl Held {
        /// Idle system sleep disabled; the display may sleep.
        pub fn awake(reason: &str) -> Self {
            Self::hold(NSActivityOptions::IdleSystemSleepDisabled, reason)
        }

        /// User-initiated, still allowing idle sleep: no App Nap.
        pub fn no_nap(reason: &str) -> Self {
            Self::hold(
                NSActivityOptions::UserInitiatedAllowingIdleSystemSleep,
                reason,
            )
        }

        fn hold(options: NSActivityOptions, reason: &str) -> Self {
            let (release, released) = mpsc::channel::<()>();
            let (taken, started) = mpsc::channel::<()>();
            let reason = reason.to_owned();
            let thread = std::thread::Builder::new()
                .name("aij-activity".to_owned())
                .spawn(move || {
                    let block = block2::RcBlock::new(move || {
                        let _ = taken.send(());
                        // Held until the sender is dropped.
                        let _ = released.recv();
                    });
                    NSProcessInfo::processInfo().performActivityWithOptions_reason_usingBlock(
                        options,
                        &NSString::from_str(&reason),
                        &block,
                    );
                })
                .ok();
            // Taken before this returns, so a caller can count on it.
            if thread.is_some() {
                let _ = started.recv_timeout(std::time::Duration::from_secs(2));
            }
            Self {
                release: Some(release),
                thread,
            }
        }
    }

    impl Drop for Held {
        fn drop(&mut self) {
            drop(self.release.take());
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    /// Nothing to hold on this system.
    pub struct Held;

    impl Held {
        pub fn awake(_reason: &str) -> Self {
            Self
        }

        pub fn no_nap(_reason: &str) -> Self {
            Self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn elsewhere_nothing_is_held_or_claimed() {
        let mut keeper = AwakeKeeper::default();
        keeper.apply(AwakePlan {
            working: 2,
            supervised: 2,
            keep_awake: true,
        });
        assert_eq!(keeper.awake_for(), None);
        assert!(!keeper.holding_off_app_nap());
    }

    #[test]
    fn names_say_how_many_sessions_work() {
        assert_eq!(awake_reason(1), "AI Juggler: 1 session working");
        assert_eq!(awake_reason(3), "AI Juggler: 3 sessions working");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn it_holds_what_the_plan_asks_and_lets_go_of_the_rest() {
        let mut keeper = AwakeKeeper::default();
        keeper.apply(AwakePlan {
            working: 2,
            supervised: 3,
            keep_awake: true,
        });
        assert_eq!(keeper.awake_for(), Some(2));
        assert!(keeper.holding_off_app_nap());
        // Not allowed now (on battery): awake goes, no App Nap stays.
        keeper.apply(AwakePlan {
            working: 2,
            supervised: 3,
            keep_awake: false,
        });
        assert_eq!(keeper.awake_for(), None);
        assert!(keeper.holding_off_app_nap());
        // Working, but waiting for the user: nothing keeps it awake.
        keeper.apply(AwakePlan {
            working: 0,
            supervised: 1,
            keep_awake: true,
        });
        assert_eq!(keeper.awake_for(), None);
        keeper.apply(AwakePlan::default());
        assert!(!keeper.holding_off_app_nap());
    }
}
