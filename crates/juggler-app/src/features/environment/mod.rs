//! Onboarding's look at this Mac: the setups it can import (ADR-0006), the
//! AJ-E302 ten-signal scan, and the AJ-E304 recommendation built from it.

pub mod commands;
pub mod dto;
pub mod recommendation;
pub mod scan;

use std::sync::Arc;

use crate::features::tools::scan_wire::{DiscoveredInfo, discovered_info_of};
use crate::state::AppState;

/// Core implementation shared with integration tests.
#[must_use]
pub fn environment_scan_impl(state: &Arc<AppState>) -> Vec<DiscoveredInfo> {
    let mut out = Vec::new();
    for harness in state.registry.ids() {
        let Some(adapter) = state.registry.get(&harness) else {
            continue;
        };
        for found in adapter.scan_environment() {
            out.push(discovered_info_of(found));
        }
    }
    out
}
