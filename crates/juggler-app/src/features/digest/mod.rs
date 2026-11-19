//! The agent digest (AJ-E3712, D-132, ADR-0036 §1): every so often while
//! agents work, a few short lines on Home's feed saying what each agent
//! is on and what is notable, and one notification for the calm news it
//! carries.

use std::sync::Arc;

use crate::state::AppState;

/// Whether the digest carries the finishes now (ADR-0036 §1.7: with its
/// notification on, sessions that finish are told in the next digest's
/// notification instead of one each). Not until the digest runs.
#[must_use]
pub fn carries_finishes(_state: &Arc<AppState>) -> bool {
    false
}
