//! `just dev-bridge-data` (F19, the 0.2 walk): fills a fresh AI Juggler
//! Dev data folder with the setups found on this computer, for a walk on
//! real data through the read-only bridge (`docs/qa/DEV-BRIDGE.md`).
//!
//! Only the Dev edition's folder is ever opened (never AI Juggler's or
//! AI Juggler Nightly's), and only when it doesn't exist yet. No tool is
//! started: the scan reads files only, the setups are used where they are
//! (nothing is written in a tool's folder), accounts are "Account A–Z",
//! and `limits_probe_allowed` is off (`juggler_app::dev_bridge::walk_data`).

use std::process::ExitCode;
use std::sync::Arc;

use juggler_app::dev_bridge::walk_data;
use juggler_app::edition::{self, Edition};

fn main() -> ExitCode {
    // Every name (data folder, Keychain services) is the Dev edition's.
    edition::set_current(Edition::Dev);
    let data_dir = match juggler_app::state::resolve_data_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("dev-bridge-data: {error}");
            return ExitCode::FAILURE;
        }
    };
    if data_dir.file_name().and_then(|name| name.to_str()) != Some(Edition::Dev.data_dir_name()) {
        eprintln!("dev-bridge-data: refusing a data folder that isn't AI Juggler Dev's");
        return ExitCode::FAILURE;
    }
    if data_dir.exists() {
        eprintln!(
            "dev-bridge-data: {} already exists. Set it aside first (rename it, as walks do), \
             then run this again on a fresh folder.",
            data_dir.display()
        );
        return ExitCode::FAILURE;
    }
    let state = match juggler_app::state::AppState::init(data_dir.clone()) {
        Ok(state) => Arc::new(state),
        Err(error) => {
            eprintln!("dev-bridge-data: couldn't open the new folder: {error}");
            return ExitCode::FAILURE;
        }
    };
    match walk_data::fill(&state) {
        Ok(outcome) => {
            for filled in &outcome.filled {
                println!("{} · {}", filled.label, filled.tool);
            }
            println!(
                "dev-bridge-data: {} setups brought in where they are, {} left out; \
                 limits_probe_allowed = off. Now `just dev-bridge`.",
                outcome.filled.len(),
                outcome.left_out
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("dev-bridge-data: {error}");
            ExitCode::FAILURE
        }
    }
}
