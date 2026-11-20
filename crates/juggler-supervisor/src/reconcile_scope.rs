//! What the reconciler checks beyond this run's own launches: this
//! computer's id (SK-10: its left-behind rows from earlier runs) and the
//! orphan grace per Provider (AJ-E504 Req.3).

use crate::supervisor::{DEFAULT_ORPHAN_GRACE_SECS, Supervisor};

impl Supervisor {
    /// Names this computer (the app's own machine id). The reconciler then
    /// checks this computer's left-behind rows from earlier runs too, whose
    /// sessions have no spawn context in this one (SK-10).
    pub fn set_machine(&self, machine: juggler_core::model::MachineId) {
        if let Ok(mut slot) = self.machine.lock() {
            *slot = Some(machine);
        }
    }

    /// This computer's id, once the app named it.
    #[must_use]
    pub fn machine(&self) -> Option<juggler_core::model::MachineId> {
        self.machine.lock().ok().and_then(|slot| slot.clone())
    }

    /// Sets the orphan grace for one Provider (seconds); the `""` key
    /// overrides the default for every Provider without its own entry
    /// (AJ-E504 Req.3 — per-Provider tunable, stored in settings by
    /// the app at startup).
    pub fn set_orphan_grace(&self, provider: &str, secs: u64) {
        if let Ok(mut graces) = self.orphan_grace_secs.lock() {
            graces.insert(provider.to_string(), secs.max(1));
        }
    }

    /// The effective orphan grace for `provider` (seconds): its own
    /// entry, else the `""` default override, else
    /// [`DEFAULT_ORPHAN_GRACE_SECS`] (30 — the false-alert budget;
    /// never lowered for demos).
    #[must_use]
    pub fn orphan_grace_for(&self, provider: &str) -> u64 {
        self.orphan_grace_secs
            .lock()
            .ok()
            .and_then(|graces| graces.get(provider).or_else(|| graces.get("")).copied())
            .unwrap_or(DEFAULT_ORPHAN_GRACE_SECS)
    }
}
